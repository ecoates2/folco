//! Data-transfer objects for the Tauri IPC boundary.
//!
//! These types define the JSON shapes that cross between the Rust backend and
//! the frontend via Tauri commands. They are intentionally separate from the
//! domain / wasm types: the Tauri IPC boundary is *always* JSON (serde_json),
//! so these DTOs use plain serde with no wasm/tsify concerns.
//!
//! Each DTO converts *from* the corresponding `folco-core` type via `From`,
//! keeping the domain type as the single source of truth. Field names mirror
//! the existing wire shape so the frontend is unaffected.
//!
//! TODO: generate matching TypeScript for these via tauri-specta.

use folco_core::{FolderIconBase, IconSizeSpec, PlatformSizeSpec, SurfaceColor};
use serde::{Deserialize, Serialize};

/// RGB surface color of a folder icon's base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceColorDto {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<SurfaceColor> for SurfaceColorDto {
    fn from(c: SurfaceColor) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}

/// A pixel rectangle describing the content bounds within an icon image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A single PNG-encoded icon image at a given size/scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconImageDto {
    pub png_data: Vec<u8>,
    pub scale: f32,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub content_bounds: Option<RectDto>,
}

/// A folder icon base (images + surface color) delivered to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderIconBaseDto {
    pub images: Vec<IconImageDto>,
    pub surface_color: SurfaceColorDto,
}

impl TryFrom<&FolderIconBase> for FolderIconBaseDto {
    type Error = String;

    fn try_from(base: &FolderIconBase) -> Result<Self, Self::Error> {
        let mut images = Vec::with_capacity(base.icons.len());
        for img in base.icons.iter() {
            let dims = img.dimensions();
            images.push(IconImageDto {
                png_data: img.to_png_bytes().map_err(|e| e.to_string())?,
                scale: img.scale,
                width: dims.width,
                height: dims.height,
                content_bounds: Some(RectDto {
                    x: img.content_bounds.x,
                    y: img.content_bounds.y,
                    width: img.content_bounds.width,
                    height: img.content_bounds.height,
                }),
            });
        }

        Ok(Self {
            images,
            surface_color: base.surface_color.into(),
        })
    }
}

/// A single target icon size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconSizeSpecDto {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

impl From<IconSizeSpec> for IconSizeSpecDto {
    fn from(s: IconSizeSpec) -> Self {
        Self {
            width: s.width,
            height: s.height,
            scale: s.scale,
        }
    }
}

/// The set of icon sizes required by the host platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSizeSpecDto {
    pub sizes: Vec<IconSizeSpecDto>,
}

impl From<PlatformSizeSpec> for PlatformSizeSpecDto {
    fn from(spec: PlatformSizeSpec) -> Self {
        Self {
            sizes: spec
                .sizes()
                .iter()
                .copied()
                .map(IconSizeSpecDto::from)
                .collect(),
        }
    }
}
