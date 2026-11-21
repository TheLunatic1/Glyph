use anyhow::{anyhow, Context, Result};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortSummary {
    pub name: String,
    pub description: String,
}

impl std::fmt::Display for SerialPortSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.description.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{} — {}", self.name, self.description)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115_200,
            data_bits: 8,
            parity: "None".to_string(),
            stop_bits: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SerialEvent {
    Data(Vec<u8>),
    Closed(Option<String>),
}

#[derive(Clone)]
pub struct SerialHandle {
    tx: std_mpsc::Sender<Vec<u8>>,
    closed: Arc<AtomicBool>,
}

impl SerialHandle {
    pub fn write(&self, bytes: Vec<u8>) {
        let _ = self.tx.send(bytes);
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}

pub fn list_serial_ports() -> Vec<SerialPortSummary> {
    let Ok(ports) = serialport::available_ports() else {
        return Vec::new();
    };
    ports
        .into_iter()
        .map(|p| {
            let desc = match p.port_type {
                serialport::SerialPortType::UsbPort(u) => {
                    u.product.unwrap_or_else(|| "USB Serial".to_string())
                }
                serialport::SerialPortType::PciPort => "PCI Serial".to_string(),
                serialport::SerialPortType::BluetoothPort => "Bluetooth Serial".to_string(),
                serialport::SerialPortType::Unknown => String::new(),
            };
            SerialPortSummary {
                name: p.port_name,
                description: desc,
            }
        })
        .collect()
}

pub struct SerialSession;

impl SerialSession {
    pub fn open(cfg: &SerialConfig) -> Result<(SerialHandle, mpsc::UnboundedReceiver<SerialEvent>)> {
        let port_name = cfg.port_name.trim();
        if port_name.is_empty() {
            return Err(anyhow!("Select or enter a serial port (e.g. COM3 or /dev/ttyUSB0)"));
        }

        let data_bits = match cfg.data_bits {
            5 => serialport::DataBits::Five,
            6 => serialport::DataBits::Six,
            7 => serialport::DataBits::Seven,
            _ => serialport::DataBits::Eight,
        };
        let parity = match cfg.parity.to_ascii_lowercase().as_str() {
            "odd" => serialport::Parity::Odd,
            "even" => serialport::Parity::Even,
            _ => serialport::Parity::None,
        };
        let stop_bits = match cfg.stop_bits {
            2 => serialport::StopBits::Two,
            _ => serialport::StopBits::One,
        };

        let mut reader = serialport::new(port_name, cfg.baud_rate)
            .data_bits(data_bits)
            .parity(parity)
            .stop_bits(stop_bits)
            .timeout(Duration::from_millis(50))
            .open()
            .with_context(|| format!("Failed to open serial port {port_name}"))?;

        let mut writer = reader
            .try_clone()
            .context("Failed to clone serial port handle")?;

        let closed = Arc::new(AtomicBool::new(false));
        let closed_r = Arc::clone(&closed);
        let closed_w = Arc::clone(&closed);

        let (ev_tx, ev_rx) = mpsc::unbounded_channel();
        let (wr_tx, wr_rx) = std_mpsc::channel::<Vec<u8>>();

        thread::Builder::new()
            .name("glyph-serial-read".to_string())
            .spawn(move || {
                let mut buf = [0u8; 2048];
                let mut err_msg = None;
                while !closed_r.load(Ordering::SeqCst) {
                    match reader.read(&mut buf) {
                        Ok(0) => {}
                        Ok(n) => {
                            if ev_tx.send(SerialEvent::Data(buf[..n].to_vec())).is_err() {
                                break;
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                        Err(e) => {
                            err_msg = Some(format!("Serial read error: {e}"));
                            break;
                        }
                    }
                }
                let _ = ev_tx.send(SerialEvent::Closed(err_msg));
            })
            .ok();

        thread::Builder::new()
            .name("glyph-serial-write".to_string())
            .spawn(move || {
                while !closed_w.load(Ordering::SeqCst) {
                    match wr_rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(bytes) => {
                            if writer.write_all(&bytes).is_err() || writer.flush().is_err() {
                                break;
                            }
                        }
                        Err(std_mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std_mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .ok();

        Ok((SerialHandle { tx: wr_tx, closed }, ev_rx))
    }
}
