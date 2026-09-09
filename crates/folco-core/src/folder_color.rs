//! Named folder color presets with target RGB colors.
//!
//! This module re-exports [FolderColor] and [FolderColorMetadata] from
//! `folco_renderer::folder_color`.
//!
//! # Usage
//!
//! ```
//! use folco_core::folder_color::{FolderColor, FolderColorExt};
//!
//! let color = FolderColor::Red;
//! let solid = color.to_solid_color_config();
//! let dot = color.to_color_dot_config();
//! // Either can be embedded in a CustomizationProfile
//! ```

pub use folco_renderer::folder_color::{FolderColor, FolderColorExt, FolderColorMetadata};
