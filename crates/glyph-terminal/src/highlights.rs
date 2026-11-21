use iced::Color;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CompiledHighlightRule {
    pub regex: Regex,
    pub color: Color,
}

pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let raw = hex.trim().trim_start_matches('#');
    if raw.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&raw[0..2], 16).ok()?;
    let g = u8::from_str_radix(&raw[2..4], 16).ok()?;
    let b = u8::from_str_radix(&raw[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}

/// Compiles `(pattern, color_hex, enabled)` tuples into fast runtime regex matchers.
/// Invalid or disabled rules are skipped without failing.
pub fn compile_highlight_rules<'a>(
    specs: impl IntoIterator<Item = (&'a str, &'a str, bool)>,
) -> Vec<CompiledHighlightRule> {
    specs
        .into_iter()
        .filter(|(_, _, enabled)| *enabled)
        .filter_map(|(pattern, hex, _)| {
            let color = parse_hex_color(hex)?;
            let regex = Regex::new(pattern).ok()?;
            Some(CompiledHighlightRule { regex, color })
        })
        .collect()
}

/// Computes per-character highlight color overrides across `line`.
/// Note: returns a map indexed by **character offset** (`0..line.chars().count()`),
/// matching terminal column layout for single-width glyphs.
pub fn compute_line_highlights(line: &str, rules: &[CompiledHighlightRule]) -> Vec<Option<Color>> {
    let char_count = line.chars().count();
    let mut colors = vec![None; char_count];
    if char_count == 0 || rules.is_empty() {
        return colors;
    }

    // Build byte-offset -> char-index lookup once
    let mut byte_to_char = vec![0usize; line.len() + 1];
    let mut ci = 0usize;
    for (bi, ch) in line.char_indices() {
        let len = ch.len_utf8();
        for slot in &mut byte_to_char[bi..bi + len] {
            *slot = ci;
        }
        ci += 1;
    }
    byte_to_char[line.len()] = ci;

    for rule in rules {
        for m in rule.regex.find_iter(line) {
            let start_c = byte_to_char[m.start()];
            let end_c = byte_to_char[m.end()].min(char_count);
            for slot in &mut colors[start_c..end_c] {
                if slot.is_none() {
                    *slot = Some(rule.color);
                }
            }
        }
    }

    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_highlights_compute_per_char_colors() {
        let rules = compile_highlight_rules([
            (r"\bERROR\b", "#f87171", true),
            (r"\bWARN\b", "#facc15", true),
            (r"[unclosed", "#ffffff", true), // invalid regex gracefully skipped
            (r"\bOK\b", "#4ade80", false),   // disabled skipped
        ]);
        assert_eq!(rules.len(), 2);

        let line = "2026 ERROR disk WARN";
        let cols = compute_line_highlights(line, &rules);
        assert_eq!(cols.len(), line.chars().count());
        assert!(cols[0].is_none());
        assert_eq!(cols[5], parse_hex_color("#f87171"));
        assert_eq!(cols[9], parse_hex_color("#f87171"));
        assert_eq!(cols[16], parse_hex_color("#facc15"));
    }
}
