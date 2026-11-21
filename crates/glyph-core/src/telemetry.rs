use crate::models::{
    CpuCoreStat, DiskMount, GpuInfo, MemStats, NetInterfaceStat, TelemetrySnapshot, TempReading,
};
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

pub const GLYPH_STAT_CMD: &str = r#"
top -b -n 1 | head -n 8
echo "===GLYPH_DELIMITER==="
free -m
echo "===GLYPH_DELIMITER==="
df -h --output=source,size,used,avail,pcent,target 2>/dev/null || df -h
echo "===GLYPH_DELIMITER==="
uptime -p
echo "===GLYPH_DELIMITER==="
who | awk '{print $1}' | sort -u | wc -l
echo "===GLYPH_DELIMITER==="
awk 'NR>2{print $1,$2,$10}' /proc/net/dev 2>/dev/null
echo "===GLYPH_DELIMITER==="
grep '^cpu' /proc/stat 2>/dev/null
echo "===GLYPH_DELIMITER==="
sensors 2>/dev/null
echo "===GLYPH_DELIMITER==="
paste <(ls /sys/class/thermal/thermal_zone*/type 2>/dev/null) <(cat /sys/class/thermal/thermal_zone*/temp 2>/dev/null) 2>/dev/null | awk '{type=$1; val=$2; printf "%s %dC\n", type, val/1000}' | head -8
echo "===GLYPH_DELIMITER==="
if command -v nvidia-smi >/dev/null 2>&1; then
  nvidia-smi --query-gpu=index,name,utilization.gpu,temperature.gpu,memory.total,memory.used --format=csv,noheader 2>/dev/null | sed 's/ MiB//g; s/ %//g'
else
  idx=0
  found=0
  for card in /sys/class/drm/card[0-9]*; do
    if [ -d "$card" ]; then
      vendor=$(cat "$card/device/vendor" 2>/dev/null)
      if [ "$vendor" = "0x1002" ]; then
        util=$(cat "$card/device/gpu_busy_percent" 2>/dev/null || echo "0")
        temp_input=$(cat "$card/device/hwmon/hwmon"*/temp1_input 2>/dev/null | head -n1 || echo "0")
        temp=$((temp_input / 1000))
        mem_total=$(cat "$card/device/mem_info_vram_total" 2>/dev/null || echo "0")
        mem_used=$(cat "$card/device/mem_info_vram_used" 2>/dev/null || echo "0")
        echo "$idx, AMD GPU, $util, $temp, $((mem_total/1024/1024)), $((mem_used/1024/1024))"
        idx=$((idx+1))
        found=1
      elif [ "$vendor" = "0x8086" ]; then
        echo "$idx, Intel GPU, 0, 0, 0, 0"
        idx=$((idx+1))
        found=1
      fi
    fi
  done
  if [ $found -eq 0 ]; then echo "NO_GPU"; fi
fi
"#;

#[derive(Debug, Clone, Copy, Default)]
struct ProcCpuCounters {
    user: u64,
    nice: u64,
    sys: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct NetByteCounters {
    rx: u64,
    tx: u64,
}

pub struct TelemetryTracker {
    prev_cpu: HashMap<String, ProcCpuCounters>,
    prev_net: HashMap<String, NetByteCounters>,
    last_tick: Option<Instant>,
    prev_snapshot: TelemetrySnapshot,
}

impl Default for TelemetryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryTracker {
    pub fn new() -> Self {
        Self {
            prev_cpu: HashMap::new(),
            prev_net: HashMap::new(),
            last_tick: None,
            prev_snapshot: TelemetrySnapshot::default(),
        }
    }

    pub fn parse(&mut self, raw_output: &str) -> TelemetrySnapshot {
        let now = Instant::now();
        let elapsed = self
            .last_tick
            .map(|t| now.duration_since(t).as_secs_f64().max(1.0))
            .unwrap_or(3.0);
        self.last_tick = Some(now);
        self.parse_with_elapsed(raw_output, elapsed)
    }

    pub fn parse_with_elapsed(&mut self, raw_output: &str, elapsed_secs: f64) -> TelemetrySnapshot {
        let elapsed = elapsed_secs.max(1.0);
        let parts: Vec<&str> = raw_output
            .split("===GLYPH_DELIMITER===")
            .map(|s| s.trim())
            .collect();

        let top_raw = parts.first().copied().unwrap_or("");
        let free_raw = parts.get(1).copied().unwrap_or("");
        let df_raw = parts.get(2).copied().unwrap_or("");
        let uptime_raw = parts.get(3).copied().unwrap_or("");
        let users_raw = parts.get(4).copied().unwrap_or("");
        let net_raw = parts.get(5).copied().unwrap_or("");
        let stat_raw = parts.get(6).copied().unwrap_or("");
        let sensors_raw = parts.get(7).copied().unwrap_or("");
        let tz_raw = parts.get(8).copied().unwrap_or("");
        let gpu_raw = parts.get(9).copied().unwrap_or("");

        // 1. Network speeds + aggregate physical interfaces
        let (net_interfaces, total_rx_bps, total_tx_bps) = self.parse_network(net_raw, elapsed);

        // 2. CPU total & per-core delta from /proc/stat
        let (cpu_from_stat, cores) = self.parse_proc_stat(stat_raw);

        // 3. Load averages, tasks, top fallback CPU
        let (load_avg, tasks_line, top_cpu) = parse_top(top_raw);
        let cpu_pct = if cpu_from_stat >= 0.0 {
            cpu_from_stat
        } else if let Some(us) = top_cpu {
            us
        } else {
            self.prev_snapshot.cpu_pct
        };

        // 4. Memory & Swap
        let mem = parse_free(free_raw);
        let mem_pct = if mem.total_mb > 0.0 {
            (mem.used_mb / mem.total_mb) * 100.0
        } else {
            self.prev_snapshot.mem_pct
        };

        // 5. Disk mounts
        let disks = parse_df(df_raw);
        let disk_pct = disks
            .first()
            .map(|d| d.pct)
            .unwrap_or(self.prev_snapshot.disk_pct);

        // 6. Temperatures
        let temps = parse_temperatures(sensors_raw, tz_raw);

        // 7. GPUs
        let (gpus, no_gpu_detected, gpu_pct) = parse_gpu(gpu_raw, self.prev_snapshot.gpu_pct);

        let uptime = if !uptime_raw.is_empty() {
            uptime_raw.to_string()
        } else {
            self.prev_snapshot.uptime.clone()
        };
        let active_users = if !users_raw.is_empty() {
            users_raw.to_string()
        } else {
            self.prev_snapshot.active_users.clone()
        };

        let snap = TelemetrySnapshot {
            cpu_pct,
            mem_pct,
            disk_pct,
            gpu_pct,
            uptime,
            active_users,
            load_avg,
            tasks_line,
            cores,
            temps,
            mem,
            disks,
            net_interfaces,
            total_rx_bps,
            total_tx_bps,
            gpus,
            no_gpu_detected,
        };
        self.prev_snapshot = snap.clone();
        snap
    }

    fn parse_network(&mut self, raw: &str, elapsed: f64) -> (Vec<NetInterfaceStat>, f64, f64) {
        let mut current = HashMap::new();
        let mut list = Vec::new();

        for line in raw.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 3 {
                let iface = p[0].trim_end_matches(':').to_string();
                let rx = p[1].parse::<u64>().unwrap_or(0);
                let tx = p[2].parse::<u64>().unwrap_or(0);
                let (rx_bps, tx_bps) = match self.prev_net.get(&iface) {
                    Some(prev) => (
                        (rx.saturating_sub(prev.rx)) as f64 / elapsed,
                        (tx.saturating_sub(prev.tx)) as f64 / elapsed,
                    ),
                    None => (0.0, 0.0),
                };
                current.insert(iface.clone(), NetByteCounters { rx, tx });
                list.push(NetInterfaceStat {
                    iface,
                    rx_bytes: rx,
                    tx_bytes: tx,
                    rx_bps,
                    tx_bps,
                });
            }
        }
        self.prev_net = current;

        let phys: Vec<&NetInterfaceStat> = list
            .iter()
            .filter(|n| !is_virtual_interface(&n.iface))
            .collect();
        let active: Vec<&NetInterfaceStat> = if phys.is_empty() {
            list.iter().collect()
        } else {
            phys
        };
        let total_rx = active.iter().map(|n| n.rx_bps).sum();
        let total_tx = active.iter().map(|n| n.tx_bps).sum();
        (list, total_rx, total_tx)
    }

    fn parse_proc_stat(&mut self, raw: &str) -> (f32, Vec<CpuCoreStat>) {
        let mut current = HashMap::new();
        let mut cores = Vec::new();
        let mut total_pct = -1.0_f32;

        for line in raw.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            let Some(&name) = p.first() else { continue };
            if !is_cpu_label(name) {
                continue;
            }
            let val = |i: usize| p.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            let cur = ProcCpuCounters {
                user: val(1),
                nice: val(2),
                sys: val(3),
                idle: val(4),
                iowait: val(5),
                irq: val(6),
                softirq: val(7),
            };
            current.insert(name.to_string(), cur);

            let pct = if let Some(prev) = self.prev_cpu.get(name) {
                let d_idle =
                    (cur.idle + cur.iowait).saturating_sub(prev.idle + prev.iowait) as f64;
                let cur_total =
                    cur.user + cur.nice + cur.sys + cur.idle + cur.iowait + cur.irq + cur.softirq;
                let prev_total = prev.user
                    + prev.nice
                    + prev.sys
                    + prev.idle
                    + prev.iowait
                    + prev.irq
                    + prev.softirq;
                let d_total = cur_total.saturating_sub(prev_total) as f64;
                if d_total > 0.0 {
                    (((d_total - d_idle) / d_total) * 100.0).clamp(0.0, 100.0) as f32
                } else {
                    0.0
                }
            } else {
                if name == "cpu" {
                    continue;
                }
                0.0
            };

            if name == "cpu" {
                total_pct = pct;
            } else {
                cores.push(CpuCoreStat {
                    id: name.to_string(),
                    pct,
                });
            }
        }
        self.prev_cpu = current;
        (total_pct, cores)
    }
}

fn is_cpu_label(name: &str) -> bool {
    if name == "cpu" {
        return true;
    }
    name.strip_prefix("cpu")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

pub fn is_virtual_interface(iface: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"^(lo|docker\d*|br-|veth|virbr|dummy|bond|ifb|nlmon|teql|tun\d*|tap\d*)")
            .expect("valid virtual interface regex")
    });
    re.is_match(iface)
}

fn parse_top(raw: &str) -> (Option<[String; 3]>, String, Option<f32>) {
    static LOAD_RE: OnceLock<Regex> = OnceLock::new();
    static US_RE: OnceLock<Regex> = OnceLock::new();
    let load_re = LOAD_RE.get_or_init(|| {
        Regex::new(r"load average:\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)").unwrap()
    });
    let us_re = US_RE.get_or_init(|| Regex::new(r"([\d.]+)\s*us").unwrap());

    let first_line = raw.lines().next().unwrap_or("");
    let load_avg = load_re.captures(first_line).map(|c| {
        [
            c[1].to_string(),
            c[2].to_string(),
            c[3].to_string(),
        ]
    });
    let tasks_line = raw
        .lines()
        .find(|l| l.to_ascii_lowercase().contains("tasks"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let top_cpu = us_re
        .captures(raw)
        .and_then(|c| c[1].parse::<f32>().ok());

    (load_avg, tasks_line, top_cpu)
}

fn parse_free(raw: &str) -> MemStats {
    let mut stats = MemStats::default();
    let lines: Vec<&str> = raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return stats;
    }

    let header_line = lines.iter().find(|l| l.to_ascii_lowercase().contains("total"));
    let mem_line = lines
        .iter()
        .find(|l| l.to_ascii_lowercase().starts_with("mem"));
    let swap_line = lines
        .iter()
        .find(|l| l.to_ascii_lowercase().starts_with("swap"));

    if let (Some(hdr), Some(mem)) = (header_line, mem_line) {
        let headers: Vec<String> = hdr.split_whitespace().map(|h| h.to_ascii_lowercase()).collect();
        let vals: Vec<f32> = mem
            .split_whitespace()
            .skip(1)
            .map(|v| v.parse::<f32>().unwrap_or(0.0))
            .collect();
        let lookup = |name: &str| -> f32 {
            headers
                .iter()
                .position(|h| h == name)
                .and_then(|i| vals.get(i).copied())
                .unwrap_or(0.0)
        };
        stats.total_mb = lookup("total");
        stats.used_mb = lookup("used");
        stats.free_mb = lookup("free");
        stats.buff_cache_mb = lookup("buff/cache") + lookup("buffers") + lookup("cached");
    } else if let Some(mem) = mem_line {
        let p: Vec<&str> = mem.split_whitespace().collect();
        stats.total_mb = p.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        stats.used_mb = p.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        stats.free_mb = p.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        stats.buff_cache_mb = p.get(5).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    }

    if let Some(swap) = swap_line {
        let p: Vec<&str> = swap.split_whitespace().collect();
        stats.swap_total_mb = p.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        stats.swap_used_mb = p.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        stats.swap_free_mb = p.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    }

    stats
}

fn parse_df(raw: &str) -> Vec<DiskMount> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .skip(1)
        .filter_map(|line| {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() < 6 {
                return None;
            }
            let pct = p[4].trim_end_matches('%').parse::<f32>().unwrap_or(0.0);
            Some(DiskMount {
                source: p[0].to_string(),
                size: p[1].to_string(),
                used: p[2].to_string(),
                avail: p[3].to_string(),
                pct,
                mountpoint: p[5..].join(" "),
            })
        })
        .collect()
}

fn parse_temperatures(sensors_raw: &str, tz_raw: &str) -> Vec<TempReading> {
    static SENSOR_RE: OnceLock<Regex> = OnceLock::new();
    let re = SENSOR_RE.get_or_init(|| {
        Regex::new(r"(?i)([+\-]?\d+(?:\.\d+)?)\s*°?\s*C(?:\s|$|\))").unwrap()
    });

    let mut out = Vec::new();
    for line in sensors_raw.lines() {
        if let Some(cap) = re.captures(line) {
            if let Ok(celsius) = cap[1].parse::<f32>() {
                let start = cap.get(0).map(|m| m.start()).unwrap_or(line.len());
                let label = line[..start]
                    .trim()
                    .trim_end_matches([':', ' ', '+'])
                    .trim()
                    .to_string();
                out.push(TempReading {
                    label: if label.is_empty() {
                        format!("Zone {}", out.len())
                    } else {
                        label
                    },
                    celsius,
                });
            }
        }
    }

    if out.is_empty() {
        for line in tz_raw.lines() {
            if let Some(cap) = re.captures(line) {
                if let Ok(celsius) = cap[1].parse::<f32>() {
                    let start = cap.get(0).map(|m| m.start()).unwrap_or(line.len());
                    let label = line[..start]
                        .trim()
                        .trim_end_matches([':', ' '])
                        .trim()
                        .to_string();
                    out.push(TempReading {
                        label: if label.is_empty() {
                            format!("Zone {}", out.len())
                        } else {
                            label
                        },
                        celsius,
                    });
                }
            }
        }
    }

    out
}

fn parse_gpu(raw: &str, prev_pct: Option<f32>) -> (Vec<GpuInfo>, bool, Option<f32>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (Vec::new(), false, prev_pct);
    }
    if trimmed == "NO_GPU" {
        return (Vec::new(), true, Some(-1.0));
    }

    let mut gpus = Vec::new();
    for line in trimmed.lines() {
        let p: Vec<&str> = line.split(", ").collect();
        if p.len() >= 6 {
            gpus.push(GpuInfo {
                index: p[0].trim().to_string(),
                name: p[1].trim().to_string(),
                util_pct: p[2].trim().parse::<f32>().unwrap_or(0.0),
                temp_c: p[3].trim().parse::<f32>().unwrap_or(0.0),
                mem_total_mb: p[4].trim().parse::<f32>().unwrap_or(0.0),
                mem_used_mb: p[5].trim().parse::<f32>().unwrap_or(0.0),
            });
        }
    }

    let avg = if gpus.is_empty() {
        prev_pct
    } else {
        Some(gpus.iter().map(|g| g.util_pct).sum::<f32>() / gpus.len() as f32)
    };
    (gpus, false, avg)
}

pub fn fmt_bytes(bytes: u64) -> String {
    let n = bytes as f64;
    if n >= 1e9 {
        format!("{:.2} GB", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1} MB", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1} KB", n / 1e3)
    } else {
        format!("{} B", bytes)
    }
}

pub fn fmt_speed(bps: f64) -> String {
    if bps >= 1e6 {
        format!("{:.1} MB/s", bps / 1e6)
    } else if bps >= 1e3 {
        format!("{:.1} KB/s", bps / 1e3)
    } else {
        format!("{:.0} B/s", bps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_parses_all_sections_and_deltas() {
        let tick1 = "\
top - 12:00:00 up 4 days, 12:31,  1 user,  load average: 0.15, 0.10, 0.05
Tasks: 120 total,   1 running, 119 sleeping,   0 stopped,   0 zombie
%Cpu(s):  2.0 us,  1.0 sy,  0.0 ni, 97.0 id
===GLYPH_DELIMITER===
               total        used        free      shared  buff/cache   available
Mem:            8000        1120        4000         100        2880        6700
Swap:           2048         100        1948
===GLYPH_DELIMITER===
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1       100G  1.0G   94G   1% /
/dev/sda2       500M  100M  400M  20% /boot
===GLYPH_DELIMITER===
up 4 days, 12 hours, 31 minutes
===GLYPH_DELIMITER===
2
===GLYPH_DELIMITER===
lo: 1000 1000
ens3: 10000 20000
docker0: 5000 5000
===GLYPH_DELIMITER===
cpu  100 0 50 850 0 0 0 0 0 0
cpu0 50 0 25 425 0 0 0 0 0 0
cpu1 50 0 25 425 0 0 0 0 0 0
===GLYPH_DELIMITER===
Core 0:        +45.0°C  (high = +80.0°C)
===GLYPH_DELIMITER===

===GLYPH_DELIMITER===
0, NVIDIA GeForce RTX 4090, 25, 52, 24564, 4096
";

        let tick2 = "\
top - 12:00:03 up 4 days, 12:31,  1 user,  load average: 0.25, 0.12, 0.06
Tasks: 121 total,   2 running, 119 sleeping,   0 stopped,   0 zombie
%Cpu(s):  5.0 us,  1.0 sy,  0.0 ni, 94.0 id
===GLYPH_DELIMITER===
               total        used        free      shared  buff/cache   available
Mem:            8000        1120        4000         100        2880        6700
Swap:           2048         100        1948
===GLYPH_DELIMITER===
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1       100G  1.0G   94G   1% /
===GLYPH_DELIMITER===
up 4 days, 12 hours, 31 minutes
===GLYPH_DELIMITER===
2
===GLYPH_DELIMITER===
lo: 2000 2000
ens3: 13000 26000
docker0: 8000 8000
===GLYPH_DELIMITER===
cpu  140 0 60 1000 0 0 0 0 0 0
cpu0 70 0 30 500 0 0 0 0 0 0
cpu1 70 0 30 500 0 0 0 0 0 0
===GLYPH_DELIMITER===
Core 0:        +48.0°C  (high = +80.0°C)
===GLYPH_DELIMITER===

===GLYPH_DELIMITER===
NO_GPU
";

        let mut tracker = TelemetryTracker::new();
        let s1 = tracker.parse_with_elapsed(tick1, 3.0);
        assert_eq!(s1.uptime, "up 4 days, 12 hours, 31 minutes");
        assert_eq!(s1.active_users, "2");
        assert!((s1.mem_pct - 14.0).abs() < 0.1);
        assert!((s1.disk_pct - 1.0).abs() < 0.1);
        assert_eq!(s1.gpus.len(), 1);
        assert!((s1.gpu_pct.unwrap() - 25.0).abs() < 0.1);
        assert_eq!(s1.temps[0].celsius, 45.0);

        let s2 = tracker.parse_with_elapsed(tick2, 3.0);
        // cpu delta: user +40, sys +10, idle +150 => total=200, busy=50 => 25.0%
        assert!((s2.cpu_pct - 25.0).abs() < 0.1);
        assert_eq!(s2.cores.len(), 2);
        // ens3 delta: rx +3000 / 3s = 1000 B/s, tx +6000 / 3s = 2000 B/s (lo and docker0 excluded!)
        assert!((s2.total_rx_bps - 1000.0).abs() < 0.1);
        assert!((s2.total_tx_bps - 2000.0).abs() < 0.1);
        assert!(s2.no_gpu_detected);
        assert_eq!(s2.gpu_pct, Some(-1.0));
    }
}
