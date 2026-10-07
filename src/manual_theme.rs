//! Manual theme: map terminal color slots onto myx roles.
//!
//! Album-art recoloring stays the default until `~/.config/myx/colors.toml`
//! sets `mode = "terminal"`. The role map is what makes one terminal theme
//! look like the next: progress, volume, and cava all read `accent` unless
//! the file says otherwise.

use crate::gradient::Rgb;
use crate::theme::Theme;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MODE_TERMINAL: &str = "terminal";

/// Mocha-style assignment, applied to whatever palette is active.
/// `accent` is the theme accent (mauve on Catppuccin Mocha), not a hardcoded hex.
const DEFAULT_ROLES: &[(&str, &str)] = &[
    ("background", "background"),
    ("panel", "color0"),
    ("element", "color8"),
    ("text", "foreground"),
    ("text_muted", "color7"),
    ("primary", "accent"),
    ("secondary", "color5"),
    ("accent", "accent"),
    ("error", "color1"),
    ("warning", "color3"),
    ("success", "color2"),
    ("info", "accent"),
    ("border", "color8"),
    ("border_active", "accent"),
    ("border_subtle", "color0"),
    ("border_dimmest", "color0"),
    ("cava", "accent"),
];

/// Last-resort hexes, only used when a slot is missing from the live palette.
/// These match Freearchy's Catppuccin Mocha `colors.toml`.
const MOCHA_FALLBACK: &[(&str, Rgb)] = &[
    ("background", rgb(0x1e, 0x1e, 0x2e)),
    ("foreground", rgb(0xcd, 0xd6, 0xf4)),
    ("accent", rgb(0xcb, 0xa6, 0xf7)),
    ("color0", rgb(0x45, 0x47, 0x5a)),
    ("color1", rgb(0xf3, 0x8b, 0xa8)),
    ("color2", rgb(0xa6, 0xe3, 0xa1)),
    ("color3", rgb(0xf9, 0xe2, 0xaf)),
    ("color4", rgb(0x89, 0xb4, 0xfa)),
    ("color5", rgb(0xf5, 0xc2, 0xe7)),
    ("color6", rgb(0x94, 0xe2, 0xd5)),
    ("color7", rgb(0xba, 0xc2, 0xde)),
    ("color8", rgb(0x45, 0x47, 0x5a)),
    ("color9", rgb(0xf3, 0x8b, 0xa8)),
    ("color10", rgb(0xa6, 0xe3, 0xa1)),
    ("color11", rgb(0xf9, 0xe2, 0xaf)),
    ("color12", rgb(0x89, 0xb4, 0xfa)),
    ("color13", rgb(0xf5, 0xc2, 0xe7)),
    ("color14", rgb(0x94, 0xe2, 0xd5)),
    ("color15", rgb(0xba, 0xc2, 0xde)),
];

const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb::new(r, g, b)
}

#[derive(Debug, Clone)]
struct ManualSpec {
    mode: String,
    roles: BTreeMap<String, String>,
}

/// Terminal colors are the default. `mode = "album"` is the only way back to
/// cover recoloring. A missing config must not fall through to Tokyo Night.
pub fn enabled() -> bool {
    load_spec().mode != "album"
}

/// The theme to show instead of the album-art palette. `None` only in album mode.
pub fn theme() -> Option<Theme> {
    let spec = load_spec();
    if spec.mode == "album" {
        return None;
    }
    let palette = load_palette();
    let theme = build_theme(&spec, &palette);
    sync_cava(theme.accent, theme.background);
    Some(theme)
}

fn build_theme(spec: &ManualSpec, palette: &BTreeMap<String, Rgb>) -> Theme {
    let role = |name: &str| resolve(spec, palette, name);
    let accent = role("accent");
    Theme {
        name: "terminal",
        primary: role("primary"),
        secondary: role("secondary"),
        accent,
        error: role("error"),
        warning: role("warning"),
        success: role("success"),
        info: role("info"),
        text: role("text"),
        text_muted: role("text_muted"),
        background: role("background"),
        background_panel: role("panel"),
        background_element: role("element"),
        border: role("border"),
        border_active: role("border_active"),
        border_subtle: role("border_subtle"),
        border_dimmest: role("border_dimmest"),
    }
}

fn resolve(spec: &ManualSpec, palette: &BTreeMap<String, Rgb>, role: &str) -> Rgb {
    let slot = spec
        .roles
        .get(role)
        .map(String::as_str)
        .or_else(|| DEFAULT_ROLES.iter().find(|(k, _)| *k == role).map(|(_, v)| *v))
        .unwrap_or("accent");
    if let Some(color) = parse_hex(slot) {
        return color;
    }
    if let Some(color) = palette.get(slot) {
        return *color;
    }
    // `accent` is not an ANSI slot. Themes that only ship color0–15 use magenta.
    if slot == "accent" {
        if let Some(color) = palette.get("color5") {
            return *color;
        }
    }
    fallback(slot)
}

fn fallback(slot: &str) -> Rgb {
    MOCHA_FALLBACK
        .iter()
        .find(|(k, _)| *k == slot)
        .map(|(_, c)| *c)
        .unwrap_or(rgb(0xcb, 0xa6, 0xf7))
}

fn load_spec() -> ManualSpec {
    let mut spec = ManualSpec {
        mode: MODE_TERMINAL.to_string(),
        roles: DEFAULT_ROLES
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
    };
    let Some(text) = read_optional(&colors_path()) else {
        return spec;
    };
    let parsed = parse_assignments(&text);
    if let Some(mode) = parsed.get("mode") {
        if mode == "album" || mode == MODE_TERMINAL {
            spec.mode = mode.clone();
        }
    }
    for (key, value) in parsed {
        if key == "mode" {
            continue;
        }
        spec.roles.insert(key, value);
    }
    spec
}

fn load_palette() -> BTreeMap<String, Rgb> {
    let mut palette = BTreeMap::new();
    for path in palette_paths() {
        if let Some(text) = read_optional(&path) {
            for (key, value) in parse_assignments(&text) {
                if let Some(color) = parse_hex(&value) {
                    palette.insert(key, color);
                }
            }
        }
    }
    palette
}

fn colors_path() -> PathBuf {
    config_dir().join("colors.toml")
}

fn palette_paths() -> Vec<PathBuf> {
    let home = crate::home_dir();
    let mut paths = Vec::new();
    if let Some(home) = home.as_ref() {
        paths.push(home.join(".config/myx/palette.toml"));
        paths.push(home.join(".config/omarchy/current/theme/colors.toml"));
    }
    paths
}

fn config_dir() -> PathBuf {
    crate::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/myx")
}

fn read_optional(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// `key = "value"` assignments, comments and table headers ignored.
/// A later key wins, so a `[roles]` override beats a top-level copy.
fn parse_assignments(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = value.trim().trim_matches('"').trim_matches('\'').trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        out.insert(key, value.to_string());
    }
    out
}

fn parse_hex(value: &str) -> Option<Rgb> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(hex, 16).ok()?;
    Some(Rgb::new(
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ))
}

/// Cava follows the same accent as the progress bar and the volume meter.
/// Only the `[color]` section is rewritten; bar count and input stay put.
pub fn sync_cava(accent: Rgb, background: Rgb) {
    let Some(home) = crate::home_dir() else {
        return;
    };
    let path = home.join(".config/cava/config");
    let color = format!(
        "[color]\n\
         ; written by myx — same accent as the progress bar and volume meter\n\
         gradient = 0\n\
         background = '{bg}'\n\
         foreground = '{fg}'\n",
        bg = hex(background),
        fg = hex(accent),
    );
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let next = replace_color_section(&existing, &color);
    if next == existing {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, next);
    let _ = SystemTime::now();
}

fn replace_color_section(existing: &str, color: &str) -> String {
    if existing.trim().is_empty() {
        return format!(
            "; cava config — [color] is owned by myx while terminal theming is on\n\n{color}"
        );
    }
    let lines: Vec<&str> = existing.lines().collect();
    let start = lines.iter().position(|l| l.trim() == "[color]");
    let Some(start) = start else {
        let mut out = existing.to_string();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(color);
        return out;
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| l.trim().starts_with('[') && l.trim().ends_with(']'))
        .map(|(i, _)| i)
        .unwrap_or(lines.len());
    let mut out = String::new();
    for line in &lines[..start] {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(color);
    if !color.ends_with('\n') {
        out.push('\n');
    }
    for line in &lines[end..] {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn hex(color: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_map_sends_progress_volume_and_cava_through_accent() {
        let spec = ManualSpec {
            mode: MODE_TERMINAL.to_string(),
            roles: DEFAULT_ROLES
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        };
        let palette = MOCHA_FALLBACK
            .iter()
            .map(|(k, v)| ((*k).to_string(), *v))
            .collect();
        let theme = build_theme(&spec, &palette);
        let mauve = rgb(0xcb, 0xa6, 0xf7);
        assert_eq!(theme.accent, mauve);
        assert_eq!(theme.primary, mauve);
        assert_eq!(theme.info, mauve);
        assert_eq!(theme.background, rgb(0x1e, 0x1e, 0x2e));
        assert_eq!(theme.text, rgb(0xcd, 0xd6, 0xf4));
        assert_eq!(theme.error, rgb(0xf3, 0x8b, 0xa8));
    }

    #[test]
    fn a_role_can_name_another_terminal_slot_or_a_hex() {
        let mut roles = BTreeMap::new();
        roles.insert("accent".to_string(), "color4".to_string());
        roles.insert("cava".to_string(), "color6".to_string());
        roles.insert("text".to_string(), "#ffffff".to_string());
        let spec = ManualSpec {
            mode: MODE_TERMINAL.to_string(),
            roles,
        };
        let mut palette = BTreeMap::new();
        palette.insert("color4".to_string(), rgb(0x89, 0xb4, 0xfa));
        palette.insert("accent".to_string(), rgb(0xcb, 0xa6, 0xf7));
        let theme = build_theme(&spec, &palette);
        assert_eq!(theme.accent, rgb(0x89, 0xb4, 0xfa));
        assert_eq!(theme.text, rgb(0xff, 0xff, 0xff));
        assert_eq!(resolve(&spec, &palette, "cava"), rgb(0x94, 0xe2, 0xd5));
    }

    #[test]
    fn color_section_is_replaced_without_touching_the_rest() {
        let existing = "[general]\nbars = 12\n\n[color]\ngradient = 1\nforeground = '#ff0000'\n\n[input]\nmethod = pulse\n";
        let next = replace_color_section(existing, "[color]\ngradient = 0\nforeground = '#cba6f7'\n");
        assert!(next.contains("[general]"));
        assert!(next.contains("bars = 12"));
        assert!(next.contains("[input]"));
        assert!(next.contains("method = pulse"));
        assert!(next.contains("foreground = '#cba6f7'"));
        assert!(!next.contains("#ff0000"));
    }
}
