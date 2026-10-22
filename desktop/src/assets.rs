//! Bundled brand images, as raw RGBA so the app needs no image decoder.
//!
//! Converted from the official brand kit (`brand/`), unmodified apart from
//! cropping and scaling (`docs/build/desktop.md` shows the commands):
//! - the app icon from `brand/03 - App Icons/VPN/VPN_Dark_128.png`;
//! - the S mark and the "Septuvo" wordmark from
//!   `brand/02 - Logos/PNG_Transparent/Septuvo_Horizontal_Dark_Transparent.png`.
//!
//! The brand kit has no one-line "Septuvo VPN" lockup like the approved
//! header, so the header places the official S mark and wordmark side by
//! side and sets "VPN" in Inter, brand blue, as the official product-family
//! lockups do (owner sign-off in the N4-01 pull request).

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};

/// A raw RGBA image with its size.
pub struct Raw {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// RGBA bytes, `width * height * 4`.
    pub rgba: &'static [u8],
}

/// The app icon, 128 × 128.
pub const ICON: Raw = Raw {
    width: 128,
    height: 128,
    rgba: include_bytes!("../assets/icons/septuvo-vpn-128.rgba"),
};
/// The S mark, 103 × 160.
pub const S_MARK: Raw = Raw {
    width: 103,
    height: 160,
    rgba: include_bytes!("../assets/brand/septuvo-s-103x160.rgba"),
};
/// The "Septuvo" wordmark (white, for dark backgrounds), 385 × 72.
pub const WORDMARK: Raw = Raw {
    width: 385,
    height: 72,
    rgba: include_bytes!("../assets/brand/septuvo-wordmark-385x72.rgba"),
};

impl Raw {
    /// True if the byte count matches the size.
    #[must_use]
    pub fn complete(&self) -> bool {
        self.rgba.len() == self.width * self.height * 4
    }

    /// Uploads it as a texture (`None` if the data is incomplete).
    #[must_use]
    pub fn texture(&self, ctx: &Context, name: &str) -> Option<TextureHandle> {
        self.complete().then(|| {
            ctx.load_texture(
                name,
                ColorImage::from_rgba_unmultiplied([self.width, self.height], self.rgba),
                TextureOptions::LINEAR,
            )
        })
    }
}

/// The loaded brand textures.
pub struct Textures {
    /// The S mark.
    pub s_mark: Option<TextureHandle>,
    /// The wordmark.
    pub wordmark: Option<TextureHandle>,
}

impl Textures {
    /// Uploads all brand images.
    #[must_use]
    pub fn load(ctx: &Context) -> Textures {
        Textures {
            s_mark: S_MARK.texture(ctx, "septuvo-s-mark"),
            wordmark: WORDMARK.texture(ctx, "septuvo-wordmark"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_image_is_complete() {
        for r in [&ICON, &S_MARK, &WORDMARK] {
            assert!(r.complete(), "{}x{}", r.width, r.height);
        }
    }
}
