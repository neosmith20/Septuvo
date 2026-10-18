//! The application: window, navigation and pages.

use eframe::egui::{self, ColorImage, Margin, TextureHandle, TextureOptions, Ui};
use std::sync::Arc;

use crate::pages::{self, NARROW, Page};
use crate::theme::{self, token};
use crate::view::StatusView;
use crate::{APP_ID, ICON_RGBA, ICON_SIZE};

/// Where the status page gets its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusSource {
    /// Live status is not wired in yet (N4-02): say so.
    Pending,
    /// A fixed sample, for the design preview and tests (always labelled).
    Sample(StatusView),
}

/// The Septuvo VPN app.
pub struct SeptuvoApp {
    /// The page shown.
    pub page: Page,
    /// What the status page shows.
    pub status: StatusSource,
    icon: Option<TextureHandle>,
    styled: bool,
}

impl SeptuvoApp {
    /// A new app showing the status page. The theme and icon are set up on
    /// the first frame.
    #[must_use]
    pub fn new(status: StatusSource) -> SeptuvoApp {
        SeptuvoApp {
            page: Page::Status,
            status,
            icon: None,
            styled: false,
        }
    }

    /// Applies the theme and loads the icon. Returns false on the first
    /// frame: new fonts take effect from the next frame, so nothing is drawn
    /// until then.
    fn setup(&mut self, ctx: &egui::Context) -> bool {
        if self.styled {
            return true;
        }
        self.styled = true;
        ctx.request_repaint();
        theme::apply(ctx);
        let size = usize::try_from(ICON_SIZE).unwrap_or(0);
        self.icon = (ICON_RGBA.len() == size * size * 4).then(|| {
            ctx.load_texture(
                "septuvo-vpn-icon",
                ColorImage::from_rgba_unmultiplied([size, size], ICON_RGBA),
                TextureOptions::LINEAR,
            )
        });
        false
    }

    /// Draws one frame into `ui` (the whole window).
    pub fn draw(&mut self, ui: &mut Ui) {
        if !self.setup(ui.ctx()) {
            return;
        }
        let wide = ui.available_width() >= NARROW;
        pages::header(ui, self.icon.as_ref());
        egui::Frame::new()
            .inner_margin(Margin::symmetric(18, 14))
            .show(ui, |ui| {
                if wide {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(170.0);
                            pages::navigation(ui, &mut self.page, true);
                        });
                        ui.add_space(8.0);
                        ui.vertical(|ui| self.body(ui));
                    });
                } else {
                    pages::navigation(ui, &mut self.page, false);
                    ui.add_space(6.0);
                    self.body(ui);
                }
            });
    }

    fn body(&self, ui: &mut Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                pages::preview_notice(ui);
                ui.add_space(10.0);
                match (self.page, &self.status) {
                    (Page::Status, StatusSource::Pending) => pages::status_pending(ui),
                    (Page::Status, StatusSource::Sample(v)) => pages::status(ui, v, true),
                    (Page::About, _) => pages::about(ui, self.icon.as_ref()),
                }
            });
    }
}

impl eframe::App for SeptuvoApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::Frame::new().fill(token::MIDNIGHT).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            self.draw(ui);
        });
    }
}

/// Window options: title, app id, icon and a size that works from 360 px.
#[must_use]
pub fn native_options(size: [f32; 2]) -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_title("Septuvo VPN")
        .with_app_id(APP_ID)
        .with_inner_size(size)
        .with_min_inner_size([360.0, 480.0])
        .with_icon(Arc::new(egui::IconData {
            rgba: ICON_RGBA.to_vec(),
            width: ICON_SIZE,
            height: ICON_SIZE,
        }));
    eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    }
}
