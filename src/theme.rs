//! The design-token system — a faithful port of noodle's semantic `Theme`.
//!
//! The whole "designed" feel comes from three ideas baked in here:
//!   1. **Three background layers** (`background` < `panel` < `element`) so panes
//!      and selections read as *elevated* without drawing a single box.
//!   2. **Semantic color roles** (primary/accent/success/... ) instead of raw
//!      fg/bg, so every color means something.
//!   3. **Four border shades** for hierarchy — active vs subtle vs dimmest.
//!
//! Palettes are stored as `Rgb` consts (zero runtime cost) and carry accurate
//! hex values lifted straight from noodle's `theme-data.ts`.

use crate::gradient::{lerp_color, Rgb};
use ratatui::style::{Modifier, Style};

/// A complete semantic palette. Field names mirror noodle's `Theme` interface
/// so cross-referencing is trivial.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,

    // Roles
    pub primary: Rgb,
    pub secondary: Rgb,
    pub accent: Rgb,

    // Status
    pub error: Rgb,
    pub warning: Rgb,
    pub success: Rgb,
    pub info: Rgb,

    // Text
    pub text: Rgb,
    pub text_muted: Rgb,

    // Three background layers — this is the depth trick.
    pub background: Rgb,
    pub background_panel: Rgb,
    pub background_element: Rgb,

    // Four border shades — hierarchy without heavy chrome.
    pub border: Rgb,
    pub border_active: Rgb,
    pub border_subtle: Rgb,
    pub border_dimmest: Rgb,
}

impl Theme {
    /// Interpolate every token from `from` to `to` at `t` in `[0, 1]`.
    /// The interpolated theme carries the *target's* name. This is the whole
    /// engine behind cross-fading the UI on a track change.
    pub fn lerp(from: &Theme, to: &Theme, t: f32) -> Theme {
        let m = |a: Rgb, b: Rgb| lerp_color(a, b, t);
        Theme {
            name: to.name,
            primary: m(from.primary, to.primary),
            secondary: m(from.secondary, to.secondary),
            accent: m(from.accent, to.accent),
            error: m(from.error, to.error),
            warning: m(from.warning, to.warning),
            success: m(from.success, to.success),
            info: m(from.info, to.info),
            text: m(from.text, to.text),
            text_muted: m(from.text_muted, to.text_muted),
            background: m(from.background, to.background),
            background_panel: m(from.background_panel, to.background_panel),
            background_element: m(from.background_element, to.background_element),
            border: m(from.border, to.border),
            border_active: m(from.border_active, to.border_active),
            border_subtle: m(from.border_subtle, to.border_subtle),
            border_dimmest: m(from.border_dimmest, to.border_dimmest),
        }
    }

    // ---- Common styles: build once, use everywhere. ----

    /// Base canvas — the whole screen sits on this.
    pub fn base(&self) -> Style {
        Style::default()
            .bg(self.background.into())
            .fg(self.text.into())
    }

    /// A pane surface — slightly elevated above the base.
    pub fn panel(&self) -> Style {
        Style::default()
            .bg(self.background_panel.into())
            .fg(self.text.into())
    }

    /// A selected / active row — the top elevation layer.
    pub fn element(&self) -> Style {
        Style::default()
            .bg(self.background_element.into())
            .fg(self.text.into())
    }

    /// De-emphasized text (secondary labels, inactive items).
    pub fn muted(&self) -> Style {
        Style::default().fg(self.text_muted.into())
    }

    /// The accent color, bold — for headings and the focused left-bar.
    pub fn heading(&self) -> Style {
        Style::default()
            .fg(self.primary.into())
            .add_modifier(Modifier::BOLD)
    }

    /// Left-accent-bar style for the focused pane / selected row.
    pub fn active_bar(&self) -> Style {
        Style::default().fg(self.border_active.into())
    }

    /// Left-accent-bar style when a pane is present but not focused.
    pub fn idle_bar(&self) -> Style {
        Style::default().fg(self.border_subtle.into())
    }
}

// ---------------------------------------------------------------------------
// Palettes. Values are exact ports from noodle's theme-data.ts.
// ---------------------------------------------------------------------------

const fn c(r: u8, g: u8, b: u8) -> Rgb {
    Rgb::new(r, g, b)
}

pub const TOKYONIGHT: Theme = Theme {
    name: "tokyonight",
    primary: c(0x82, 0xaa, 0xff),
    secondary: c(0xc0, 0x99, 0xff),
    accent: c(0xff, 0x96, 0x6c),
    error: c(0xff, 0x75, 0x7f),
    warning: c(0xff, 0x96, 0x6c),
    success: c(0xc3, 0xe8, 0x8d),
    info: c(0x82, 0xaa, 0xff),
    text: c(0xc8, 0xd3, 0xf5),
    text_muted: c(0x82, 0x8b, 0xb8),
    background: c(0x1a, 0x1b, 0x26),
    background_panel: c(0x1e, 0x20, 0x30),
    background_element: c(0x22, 0x24, 0x36),
    border: c(0x73, 0x7a, 0xa2),
    border_active: c(0x90, 0x99, 0xb2),
    border_subtle: c(0x54, 0x5c, 0x7e),
    border_dimmest: c(0x2a, 0x2c, 0x41),
};

pub const CATPPUCCIN: Theme = Theme {
    name: "catppuccin",
    primary: c(0xcb, 0xa6, 0xf7),
    secondary: c(0x89, 0xb4, 0xfa),
    accent: c(0xf5, 0xc2, 0xe7),
    error: c(0xf3, 0x8b, 0xa8),
    warning: c(0xfa, 0xb3, 0x87),
    success: c(0xa6, 0xe3, 0xa1),
    info: c(0x89, 0xdc, 0xeb),
    text: c(0xcd, 0xd6, 0xf4),
    text_muted: c(0x6c, 0x70, 0x86),
    background: c(0x1e, 0x1e, 0x2e),
    background_panel: c(0x18, 0x18, 0x25),
    background_element: c(0x31, 0x32, 0x44),
    border: c(0x45, 0x47, 0x5a),
    border_active: c(0x58, 0x5b, 0x70),
    border_subtle: c(0x31, 0x32, 0x44),
    border_dimmest: c(0x31, 0x32, 0x44),
};

pub const ROSEPINE: Theme = Theme {
    name: "rosepine",
    primary: c(0x9c, 0xcf, 0xd8),
    secondary: c(0xc4, 0xa7, 0xe7),
    accent: c(0xeb, 0xbc, 0xba),
    error: c(0xeb, 0x6f, 0x92),
    warning: c(0xf6, 0xc1, 0x77),
    success: c(0x31, 0x74, 0x8f),
    info: c(0x9c, 0xcf, 0xd8),
    text: c(0xe0, 0xde, 0xf4),
    text_muted: c(0x6e, 0x6a, 0x86),
    background: c(0x19, 0x17, 0x24),
    background_panel: c(0x1f, 0x1d, 0x2e),
    background_element: c(0x26, 0x23, 0x3a),
    border: c(0x40, 0x3d, 0x52),
    border_active: c(0x9c, 0xcf, 0xd8),
    border_subtle: c(0x21, 0x20, 0x2e),
    border_dimmest: c(0x25, 0x23, 0x38),
};

/// Follows the terminal's own palette. Backgrounds are unset so the terminal
/// background shows through; roles map onto the 16 ANSI colors.
pub const TERMINAL: Theme = Theme {
    name: "terminal",
    primary: Rgb::ansi(6),
    secondary: Rgb::ansi(5),
    accent: Rgb::ansi(4),
    error: Rgb::ansi(1),
    warning: Rgb::ansi(3),
    success: Rgb::ansi(2),
    info: Rgb::ansi(4),
    text: Rgb::terminal(),
    text_muted: Rgb::ansi(8),
    background: Rgb::terminal(),
    background_panel: Rgb::terminal(),
    background_element: Rgb::ansi(8),
    border: Rgb::ansi(8),
    border_active: Rgb::ansi(6),
    border_subtle: Rgb::ansi(8),
    border_dimmest: Rgb::ansi(8),
};


/// Catppuccin Mocha, using Freearchy's `themes/catppuccin-mocha` palette:
/// base `#1e1e2e`, text `#cdd6f4`, accent `#cba6f7`, and the 16-color set
/// from that theme's `colors.toml` / `alacritty.toml`.
pub const MOCHA: Theme = Theme {
    name: "mocha",
    primary: c(0xcb, 0xa6, 0xf7),
    secondary: c(0x89, 0xb4, 0xfa),
    accent: c(0xf5, 0xc2, 0xe7),
    error: c(0xf3, 0x8b, 0xa8),
    warning: c(0xf9, 0xe2, 0xaf),
    success: c(0xa6, 0xe3, 0xa1),
    info: c(0x94, 0xe2, 0xd5),
    text: c(0xcd, 0xd6, 0xf4),
    text_muted: c(0x7f, 0x84, 0x9c),
    background: c(0x1e, 0x1e, 0x2e),
    background_panel: c(0x18, 0x18, 0x25),
    background_element: c(0x31, 0x32, 0x44),
    border: c(0x45, 0x47, 0x5a),
    border_active: c(0xcb, 0xa6, 0xf7),
    border_subtle: c(0x45, 0x47, 0x5a),
    border_dimmest: c(0x31, 0x32, 0x44),
};

pub const GRUVBOX: Theme = Theme {
    name: "gruvbox",
    primary: c(0x83, 0xa5, 0x98),
    secondary: c(0xd3, 0x86, 0x9b),
    accent: c(0x8e, 0xc0, 0x7c),
    error: c(0xfb, 0x49, 0x34),
    warning: c(0xfe, 0x80, 0x19),
    success: c(0xb8, 0xbb, 0x26),
    info: c(0xfa, 0xbd, 0x2f),
    text: c(0xeb, 0xdb, 0xb2),
    text_muted: c(0x92, 0x83, 0x74),
    background: c(0x28, 0x28, 0x28),
    background_panel: c(0x3c, 0x38, 0x36),
    background_element: c(0x50, 0x49, 0x45),
    border: c(0x66, 0x5c, 0x54),
    border_active: c(0xeb, 0xdb, 0xb2),
    border_subtle: c(0x50, 0x49, 0x45),
    border_dimmest: c(0x50, 0x49, 0x45),
};

/// All built-in themes, in picker order.
pub const THEMES: &[Theme] = &[TERMINAL, MOCHA, TOKYONIGHT, CATPPUCCIN, ROSEPINE, GRUVBOX];

/// Resolve a config name. Unknown names fall back to the terminal palette.
/// `terminal` uses `[colors]` in config, so the mocha slot map is the default
/// and a user can retarget any role.
pub fn by_name(name: &str) -> Theme {
    let want = name.trim().to_ascii_lowercase();
    if want == "terminal" {
        return terminal_from_config();
    }
    THEMES
        .iter()
        .copied()
        .find(|t| t.name == want || (want == "catppuccin-mocha" && t.name == "mocha"))
        .unwrap_or_else(terminal_from_config)
}

/// The theme after `name` in picker order, wrapping. Terminal is rebuilt from
/// the role map so `t` does not snap back to the hardcoded ANSI defaults.
pub fn next(name: &str) -> Theme {
    let i = THEMES.iter().position(|t| t.name == name).unwrap_or(0);
    let next = THEMES[(i + 1) % THEMES.len()];
    if next.name == "terminal" {
        terminal_from_config()
    } else {
        next
    }
}

fn terminal_from_config() -> Theme {
    #[cfg(feature = "streaming")]
    {
        let roles = &crate::config::get().colors;
        return Theme {
            name: "terminal",
            primary: slot(&roles.primary),
            secondary: slot(&roles.secondary),
            accent: slot(&roles.accent),
            error: slot(&roles.error),
            warning: slot(&roles.warning),
            success: slot(&roles.success),
            info: slot(&roles.info),
            text: slot(&roles.text),
            text_muted: slot(&roles.text_muted),
            background: slot(&roles.background),
            background_panel: slot(&roles.panel),
            background_element: slot(&roles.element),
            border: slot(&roles.border),
            border_active: slot(&roles.border_active),
            border_subtle: slot(&roles.border_subtle),
            border_dimmest: slot(&roles.border_dimmest),
        };
    }
    #[cfg(not(feature = "streaming"))]
    {
        TERMINAL
    }
}

/// `color0`–`color15`, `foreground`, `background`, `terminal`, or `#rrggbb`.
pub fn slot(name: &str) -> Rgb {
    let name = name.trim().to_ascii_lowercase();
    if let Some(hex) = name.strip_prefix('#') {
        return Rgb::from_hex(hex);
    }
    if name == "foreground" || name == "fg" || name == "background" || name == "bg" || name == "terminal" {
        return Rgb::terminal();
    }
    if let Some(index) = name.strip_prefix("color").or_else(|| name.strip_prefix("ansi")) {
        if let Ok(n) = index.parse::<u8>() {
            if n <= 15 {
                return Rgb::ansi(n);
            }
        }
    }
    if let Ok(n) = name.parse::<u8>() {
        if n <= 15 {
            return Rgb::ansi(n);
        }
    }
    Rgb::ansi(5)
}

