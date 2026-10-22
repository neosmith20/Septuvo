//! The Septuvo visual theme: brand colour tokens and the Inter typeface
//! (brand/04 - Colors And Tokens, N4-D6).
//!
//! Colour never carries meaning alone: every status has words too (brand
//! rule). Text colours are checked for contrast in the tests below.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use std::sync::Arc;

use crate::state::Tone;

/// A brand colour token, as `#RRGGBB`.
const fn hex(rgb: u32) -> Color32 {
    // The shifts and masks keep each value within 0..=255.
    #[allow(clippy::cast_possible_truncation)]
    Color32::from_rgb(
        (rgb >> 16) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

/// Brand tokens (brand-tokens.md).
#[allow(clippy::unreadable_literal)] // Written exactly as in the brand file (#RRGGBB).
pub mod token {
    use super::{Color32, hex};
    /// Window background.
    pub const MIDNIGHT: Color32 = hex(0x050A14);
    /// Panels.
    pub const NAVY: Color32 = hex(0x0B1220);
    /// Cards and controls.
    pub const SURFACE: Color32 = hex(0x1E293B);
    /// Lines and outlines.
    pub const BORDER: Color32 = hex(0x334155);
    /// Primary accent.
    pub const BLUE: Color32 = hex(0x0066FF);
    /// Highlight accent.
    pub const ELECTRIC: Color32 = hex(0x00D5F8);
    /// Secondary accent.
    pub const SKY: Color32 = hex(0x38BDF8);
    /// Main text.
    pub const WHITE: Color32 = hex(0xF8FAFC);
    /// Secondary text.
    pub const MUTED: Color32 = hex(0x94A3B8);
    /// Passed.
    pub const GREEN: Color32 = hex(0x10B981);
    /// Attention.
    pub const YELLOW: Color32 = hex(0xF59E0B);
    /// Failed or unsafe.
    pub const RED: Color32 = hex(0xEF4444);
}

/// Family names for the bundled Inter weights (brand: Regular, Medium, `SemiBold`, Bold).
pub const MEDIUM: &str = "Inter Medium";
/// See [`MEDIUM`].
pub const SEMIBOLD: &str = "Inter SemiBold";
/// See [`MEDIUM`].
pub const BOLD: &str = "Inter Bold";

/// Inter Medium at `size` points.
#[must_use]
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

/// Inter `SemiBold` at `size` points.
#[must_use]
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Inter Bold at `size` points.
#[must_use]
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

/// The colour for a tone.
#[must_use]
pub fn tone(t: Tone) -> Color32 {
    match t {
        Tone::Neutral => token::MUTED,
        Tone::Good => token::GREEN,
        Tone::Caution => token::YELLOW,
        Tone::Danger => token::RED,
    }
}

/// Inter Regular, Medium, `SemiBold` and Bold (SIL Open Font License 1.1,
/// `assets/fonts/OFL.txt`).
#[must_use]
pub fn fonts() -> FontDefinitions {
    let mut f = FontDefinitions::empty();
    let faces: [(&str, &'static [u8]); 4] = [
        ("Inter", include_bytes!("../assets/fonts/Inter-Regular.ttf")),
        (MEDIUM, include_bytes!("../assets/fonts/Inter-Medium.ttf")),
        (
            SEMIBOLD,
            include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
        ),
        (BOLD, include_bytes!("../assets/fonts/Inter-Bold.ttf")),
    ];
    for (name, bytes) in faces {
        f.font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
        if name != "Inter" {
            f.families
                .insert(FontFamily::Name(name.into()), vec![name.into()]);
        }
    }
    f.families
        .insert(FontFamily::Proportional, vec!["Inter".into()]);
    // No monospace font is bundled: Inter stands in for the few code-like strings.
    f.families
        .insert(FontFamily::Monospace, vec!["Inter".into()]);
    f
}

/// Applies fonts, text sizes, colours and spacing.
pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, semibold(24.0)),
            (TextStyle::Body, FontId::proportional(16.0)),
            (TextStyle::Button, FontId::proportional(16.0)),
            (TextStyle::Small, FontId::proportional(13.0)),
            (TextStyle::Monospace, FontId::monospace(14.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.spacing.interact_size.y = 32.0;
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(token::WHITE);
        v.panel_fill = token::MIDNIGHT;
        v.window_fill = token::NAVY;
        v.faint_bg_color = token::NAVY;
        v.extreme_bg_color = token::MIDNIGHT;
        v.hyperlink_color = token::SKY;
        v.selection.bg_fill = token::BLUE;
        v.selection.stroke = Stroke::new(1.0, token::WHITE);
        let radius = CornerRadius::same(8);
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = radius;
            w.fg_stroke = Stroke::new(1.0, token::WHITE);
        }
        v.widgets.noninteractive.bg_fill = token::NAVY;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, token::BORDER);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, token::WHITE);
        v.widgets.inactive.bg_fill = token::SURFACE;
        v.widgets.inactive.weak_bg_fill = token::SURFACE;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, token::BORDER);
        v.widgets.hovered.bg_fill = token::BORDER;
        v.widgets.hovered.weak_bg_fill = token::BORDER;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, token::SKY);
        v.widgets.active.bg_fill = token::BLUE;
        v.widgets.active.weak_bg_fill = token::BLUE;
        v.widgets.active.bg_stroke = Stroke::new(1.0, token::ELECTRIC);
        // Keyboard focus must always be visible.
        v.widgets.hovered.expansion = 1.0;
    });
}

/// WCAG contrast ratio between two colours (1.0 to 21.0).
#[must_use]
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn lum(c: Color32) -> f32 {
        let ch = |v: u8| {
            let s = f32::from(v) / 255.0;
            if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_match_the_brand_file() {
        assert_eq!(token::MIDNIGHT, Color32::from_rgb(0x05, 0x0A, 0x14));
        assert_eq!(token::ELECTRIC, Color32::from_rgb(0x00, 0xD5, 0xF8));
        assert_eq!(token::RED, Color32::from_rgb(0xEF, 0x44, 0x44));
    }

    #[test]
    fn text_colours_have_enough_contrast() {
        // WCAG AA: 4.5:1 for body text, on every background text is drawn on.
        for bg in [token::MIDNIGHT, token::NAVY, token::SURFACE] {
            for fg in [
                token::WHITE,
                token::MUTED,
                token::GREEN,
                token::YELLOW,
                token::SKY,
                token::ELECTRIC,
            ] {
                assert!(
                    contrast(fg, bg) >= 4.5,
                    "{fg:?} on {bg:?}: {:.2}",
                    contrast(fg, bg)
                );
            }
        }
        // Red is used for words on the darkest backgrounds only, and for borders.
        assert!(contrast(token::RED, token::NAVY) >= 4.5);
        assert!(contrast(token::RED, token::MIDNIGHT) >= 4.5);
    }
}
