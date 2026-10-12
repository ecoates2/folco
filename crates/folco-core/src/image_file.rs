//! Reading user image files as icon sources.

use std::path::Path;

use folco_renderer::{ImageSource, SvgSource};
use serde::Serialize;

use crate::error::{Error, Result};

/// Raster formats accepted as image files, by extension.
const RASTER_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff", "tif"];

/// Whether an image is scalable markup or pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Svg,
    Raster,
}

impl ImageFormat {
    /// The format of an extension [`ImageFile::read`] accepts, case-insensitively.
    pub fn from_extension(ext: &str) -> Option<Self> {
        if ext.eq_ignore_ascii_case("svg") {
            Some(Self::Svg)
        } else if RASTER_EXTENSIONS.iter().any(|r| ext.eq_ignore_ascii_case(r)) {
            Some(Self::Raster)
        } else {
            None
        }
    }

    /// The format of `path`, by extension.
    pub fn of_path(path: &Path) -> Option<Self> {
        Self::from_extension(path.extension()?.to_str()?)
    }

    /// The format of an image source; emoji count as SVG.
    pub fn of_source(source: &ImageSource) -> Self {
        if source.is_svg() { Self::Svg } else { Self::Raster }
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Svg => "SVG",
            Self::Raster => "raster",
        })
    }
}

/// An image file read from disk, classified by extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFile {
    /// SVG markup.
    Svg(String),
    /// Encoded raster bytes (PNG, JPEG, WebP, ...).
    Raster(Vec<u8>),
}

impl ImageFile {
    /// Every extension [`read`](Self::read) accepts, lowercase and without the dot.
    pub fn supported_extensions() -> impl Iterator<Item = &'static str> {
        std::iter::once("svg").chain(RASTER_EXTENSIONS.iter().copied())
    }

    /// Whether `path` has an extension [`read`](Self::read) accepts.
    pub fn is_supported(path: &Path) -> bool {
        ImageFormat::of_path(path).is_some()
    }

    /// Reads `path` as SVG markup or raster bytes, based on its extension.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedImageFile`] for unknown extensions and
    /// [`Error::ReadImageFile`] if the file can't be read.
    pub fn read(path: &Path) -> Result<Self> {
        let format = ImageFormat::of_path(path)
            .ok_or_else(|| Error::UnsupportedImageFile(path.to_path_buf()))?;

        let read_err = |source| Error::ReadImageFile {
            path: path.to_path_buf(),
            source,
        };

        match format {
            ImageFormat::Svg => std::fs::read_to_string(path).map(Self::Svg).map_err(read_err),
            ImageFormat::Raster => std::fs::read(path).map(Self::Raster).map_err(read_err),
        }
    }

    /// Whether the file is SVG markup or raster bytes.
    pub fn format(&self) -> ImageFormat {
        match self {
            Self::Svg(_) => ImageFormat::Svg,
            Self::Raster(_) => ImageFormat::Raster,
        }
    }

    /// The file's contents: UTF-8 markup for SVG, encoded bytes for raster.
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Svg(svg) => svg.as_bytes(),
            Self::Raster(bytes) => bytes,
        }
    }
}

impl From<ImageFile> for ImageSource {
    fn from(file: ImageFile) -> Self {
        match file {
            ImageFile::Svg(svg) => ImageSource::from(SvgSource::Raw(svg)),
            ImageFile::Raster(bytes) => ImageSource::raster(bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_extension_case_insensitively() {
        assert!(ImageFile::is_supported(Path::new("a.SVG")));
        assert!(ImageFile::is_supported(Path::new("a.Jpeg")));
        assert!(!ImageFile::is_supported(Path::new("a.txt")));
        assert!(!ImageFile::is_supported(Path::new("noext")));
    }

    #[test]
    fn reads_svg_as_markup_and_raster_as_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let svg = dir.path().join("icon.svg");
        let png = dir.path().join("icon.png");
        std::fs::write(&svg, "<svg/>").unwrap();
        std::fs::write(&png, [1u8, 2, 3]).unwrap();

        assert_eq!(ImageFile::read(&svg).unwrap(), ImageFile::Svg("<svg/>".into()));
        assert_eq!(ImageFile::read(&png).unwrap(), ImageFile::Raster(vec![1, 2, 3]));
    }

    #[test]
    fn rejects_unsupported_extension_without_reading() {
        let err = ImageFile::read(Path::new("/does/not/exist.txt")).unwrap_err();
        assert!(matches!(err, Error::UnsupportedImageFile(_)));
    }
}
