//! Error types for folco-renderer.

use thiserror::Error;

/// Errors that can occur during icon rendering.
#[derive(Debug, Error)]
pub enum RenderError {
    /// The SVG data could not be parsed.
    #[error("failed to parse SVG: {source}")]
    SvgParse {
        /// The underlying usvg parse error.
        #[from]
        source: resvg::usvg::Error,
    },

    /// The emoji character is not supported by twemoji.
    #[error("unsupported emoji character: {emoji:?}")]
    InvalidEmoji {
        /// The emoji string that failed to resolve.
        emoji: String,
    },

    /// The emoji name is not recognized by twemoji.
    #[error("unsupported emoji name: {name:?}")]
    InvalidEmojiName {
        /// The name that failed to resolve.
        name: String,
    },

    /// An emoji source was used but the `twemoji` feature is not enabled.
    #[error("emoji support requires the \"twemoji\" feature")]
    TwemojiNotAvailable,

    /// Failed to create a pixel buffer for rendering.
    #[error("failed to create render target ({width}x{height})")]
    PixmapCreation {
        /// Requested width.
        width: u32,
        /// Requested height.
        height: u32,
    },

    /// No base icon was found at the requested logical size.
    #[error("no base icon available for logical size {logical_size}")]
    NoBaseIcon {
        /// The logical size that was requested.
        logical_size: u32,
    },

    /// Failed to decode a raster image (e.g., invalid PNG data).
    #[error("failed to decode image: {0}")]
    ImageDecode(image::ImageError),

    /// Failed to encode an image (e.g., to PNG for `ImageSource`).
    #[error("failed to encode image: {source}")]
    ImageEncode {
        /// The underlying image encoding error.
        source: image::ImageError,
    },
}
