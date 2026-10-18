//! Drawing the pages. Layout only: every value shown comes from the view
//! model ([`crate::view`]), and everything from outside the app is shown as
//! plain text (labels never interpret markup, links or commands).

use eframe::egui::{
    self, Align, Color32, CornerRadius, Layout, Margin, RichText, Stroke, TextureHandle, Ui, Vec2,
};

use crate::theme::{self, semibold, token};
use crate::view::{CheckRow, Detail, PREVIEW_NOTICE, StatusView, Tone};

/// Narrower than this, the navigation moves from the side to the top.
pub const NARROW: f32 = 720.0;

/// The pages of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// Connection status.
    Status,
    /// Version, licence and the development-preview notice.
    About,
}

impl Page {
    /// All pages, in navigation order.
    pub const ALL: [Page; 2] = [Page::Status, Page::About];

    /// The page's name in the navigation.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Page::Status => "Status",
            Page::About => "About",
        }
    }
}

/// A card: the main building block of every page.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(token::SURFACE)
        .stroke(Stroke::new(1.0, token::BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A card with a coloured outline, for warnings.
fn outlined<R>(ui: &mut Ui, color: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(token::SURFACE)
        .stroke(Stroke::new(1.5, translucent(color, 190)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn translucent(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// A small coloured label, for example "Passed". The word carries the
/// meaning; the colour only repeats it.
pub fn pill(ui: &mut Ui, text: &str, t: Tone) {
    let c = theme::tone(t);
    egui::Frame::new()
        .fill(translucent(c, 36))
        .stroke(Stroke::new(1.0, translucent(c, 150)))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            let label = egui::Label::new(RichText::new(text).font(semibold(13.0)).color(c));
            // Short labels never wrap; long ones (the sample notice) may.
            ui.add(if text.chars().count() <= 24 {
                label.extend()
            } else {
                label.wrap()
            });
        });
}

/// The bar at the top: icon, product name, preview label.
pub fn header(ui: &mut Ui, icon: Option<&TextureHandle>) {
    egui::Frame::new()
        .fill(token::NAVY)
        .inner_margin(Margin::symmetric(18, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(icon) = icon {
                    ui.add(
                        egui::Image::from_texture((icon.id(), Vec2::splat(30.0)))
                            .alt_text("Septuvo VPN logo"),
                    );
                }
                ui.label(RichText::new("Septuvo VPN").font(semibold(19.0)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    pill(ui, "DEVELOPMENT PREVIEW", Tone::Caution);
                });
            });
        });
}

/// The navigation: a column on wide windows, a row of tabs on narrow ones.
pub fn navigation(ui: &mut Ui, page: &mut Page, vertical: bool) {
    let mut item = |ui: &mut Ui, p: Page| {
        let selected = *page == p;
        let text = RichText::new(p.title()).font(if selected {
            semibold(15.0)
        } else {
            egui::FontId::proportional(15.0)
        });
        let b = egui::Button::selectable(selected, text)
            .min_size(Vec2::new(if vertical { 160.0 } else { 0.0 }, 34.0));
        if ui.add(b).clicked() {
            *page = p;
        }
    };
    if vertical {
        ui.vertical(|ui| {
            for p in Page::ALL {
                item(ui, p);
            }
        });
    } else {
        ui.horizontal(|ui| {
            for p in Page::ALL {
                item(ui, p);
            }
        });
    }
}

/// The development-preview notice, on every page until G1.
pub fn preview_notice(ui: &mut Ui) {
    egui::Frame::new()
        .fill(translucent(token::YELLOW, 22))
        .stroke(Stroke::new(1.0, translucent(token::YELLOW, 120)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(PREVIEW_NOTICE).color(token::WHITE));
        });
}

fn check_row(ui: &mut Ui, c: &CheckRow, narrow: bool) {
    let draw_text = |ui: &mut Ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(c.label).font(semibold(15.0)));
            ui.label(RichText::new(c.help).color(token::MUTED).small());
        });
    };
    if narrow {
        draw_text(ui);
        pill(ui, c.state, c.tone);
    } else {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new((ui.available_width() - 110.0).max(120.0), 0.0),
                Layout::top_down(Align::Min),
                draw_text,
            );
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add_space(2.0);
                pill(ui, c.state, c.tone);
            });
        });
    }
}

fn detail_row(ui: &mut Ui, d: &Detail) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{}:", d.label)).color(token::MUTED));
        let color = if d.tone == Tone::Neutral {
            token::WHITE
        } else {
            theme::tone(d.tone)
        };
        ui.label(RichText::new(&d.value).color(color));
    });
}

/// The status page for one view.
pub fn status(ui: &mut Ui, v: &StatusView, sample: bool) {
    let narrow = ui.available_width() < NARROW - 200.0;
    if sample {
        pill(
            ui,
            "SAMPLE DATA: design preview, not this computer's status",
            Tone::Neutral,
        );
        ui.add_space(4.0);
    }
    // Headline: the one line people must not misread.
    let c = theme::tone(v.headline.tone);
    egui::Frame::new()
        .fill(token::SURFACE)
        .stroke(Stroke::new(1.0, translucent(c, 170)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(&v.headline.title)
                    .font(semibold(30.0))
                    .color(c),
            );
            ui.label(RichText::new(&v.headline.detail).color(token::WHITE));
        });
    if let Some(why) = &v.service_lost {
        ui.add_space(6.0);
        outlined(ui, token::RED, |ui| {
            ui.label(
                RichText::new("SERVICE LOST")
                    .font(semibold(15.0))
                    .color(token::RED),
            );
            ui.label(format!("Reason: {why}"));
            ui.label(
                RichText::new("Nothing below is known until the service answers again.")
                    .color(token::MUTED),
            );
        });
    }
    if let Some(cp) = &v.captive {
        ui.add_space(6.0);
        outlined(ui, token::YELLOW, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Captive-portal window open")
                        .font(semibold(17.0))
                        .color(token::YELLOW),
                );
                pill(ui, &cp.left, Tone::Caution);
            });
            ui.label(RichText::new(format!("Warning: {}", cp.warning)).color(token::WHITE));
            if let Some(dns) = &cp.dns {
                ui.label(RichText::new(format!("DNS for sign-in: {dns}")).color(token::MUTED));
            }
        });
    }
    ui.add_space(6.0);
    card(ui, |ui| {
        ui.label(RichText::new("Connection checks").font(semibold(17.0)));
        ui.label(
            RichText::new("Septuvo shows \"Connected\" only when all four pass, in order.")
                .color(token::MUTED)
                .small(),
        );
        ui.add_space(6.0);
        for (i, c) in v.checks.iter().enumerate() {
            if i > 0 {
                ui.separator();
            }
            check_row(ui, c, narrow);
        }
    });
    if !v.details.is_empty() {
        ui.add_space(6.0);
        card(ui, |ui| {
            ui.label(RichText::new("Protection").font(semibold(17.0)));
            ui.add_space(4.0);
            for d in &v.details {
                detail_row(ui, d);
            }
        });
    }
}

/// The status page before live status is wired in (N4-02): says so plainly
/// instead of showing a state the app has not checked.
pub fn status_pending(ui: &mut Ui) {
    card(ui, |ui| {
        ui.label(RichText::new("Live status is coming in the next update").font(semibold(22.0)));
        ui.label(
            "This early build of Septuvo VPN does not read the service yet, so it shows no \
             connection state. Run `septuvo status` or `septuvo watch` in a terminal to see it.",
        );
    });
}

/// The About page.
pub fn about(ui: &mut Ui, icon: Option<&TextureHandle>) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some(icon) = icon {
                ui.add(
                    egui::Image::from_texture((icon.id(), Vec2::splat(64.0)))
                        .alt_text("Septuvo VPN logo"),
                );
            }
            ui.vertical(|ui| {
                ui.label(RichText::new("Septuvo VPN").font(semibold(26.0)));
                ui.label(
                    RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                        .color(token::MUTED),
                );
                ui.label(RichText::new("Security Without Compromise.").color(token::ELECTRIC));
            });
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Licensed under the PolyForm Noncommercial License 1.0.0. Commercial use requires a \
                 separate written agreement.",
            )
            .color(token::MUTED),
        );
        ui.label(
            RichText::new(
                "Typeface: Inter, SIL Open Font License 1.1. Built entirely in Rust with egui \
                 (MIT OR Apache-2.0).",
            )
            .color(token::MUTED),
        );
    });
}
