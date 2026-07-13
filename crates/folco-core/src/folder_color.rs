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
//! let config = color.to_folder_color_target_config();
//! // config can be embedded in a CustomizationProfile
//! ```

pub use folco_renderer::folder_color::{FolderColor, FolderColorExt, FolderColorMetadata};
