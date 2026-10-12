//! Reading user image files as icon sources.

use std::path::Path;

use folco_renderer::{ImageSource, SvgSource};

use crate::error::{Error, Result};

/// Raster formats accepted as image files, by extension.
const RASTER_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff", "tif"];

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
        path.extension().is_some_and(|ext| {
            Self::supported_extensions().any(|supported| ext.eq_ignore_ascii_case(supported))
        })
    }

    /// Reads `path` as SVG markup or raster bytes, based on its extension.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedImageFile`] for unknown extensions and
    /// [`Error::ReadImageFile`] if the file can't be read.
    pub fn read(path: &Path) -> Result<Self> {
        if !Self::is_supported(path) {
            return Err(Error::UnsupportedImageFile(path.to_path_buf()));
        }

        let read_err = |source| Error::ReadImageFile {
            path: path.to_path_buf(),
            source,
        };
        let is_svg = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));

        if is_svg {
            std::fs::read_to_string(path).map(Self::Svg).map_err(read_err)
        } else {
            std::fs::read(path).map(Self::Raster).map_err(read_err)
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
