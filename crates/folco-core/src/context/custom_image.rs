//! Customizing a user-supplied image instead of the system folder icon.
//!
//! The platform decides the medium: an SVG stays vector where the platform
//! can write SVG icons, and is rasterized to the platform's custom-image sizes
//! otherwise. Raster sources are always rasterized.

use std::path::Path;

use folco_renderer::{
    CustomizationProfile, Customizer, ImageSource, RasterCustomizer, SvgCustomizer,
};

use super::CustomizationContext;
use super::operations::render_and_write;
use crate::convert::SystemFormat;
use crate::error::{Error, Result};
use crate::image_file::ImageFormat;
use crate::progress::ProgressSender;
use crate::sys::{OutputMedium, PlatformSizeSpec, WriteCapabilities};

impl CustomizationContext {
    /// The medium `source` is customized and written in when used as a custom image.
    ///
    /// Decided by the platform's [`WriteCapabilities`]: SVG stays vector where
    /// possible and is rasterized to [`custom_image_size_spec`](Self::custom_image_size_spec)
    /// otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedCustomImage`] if the platform can't write it in any medium.
    fn custom_image_medium(&self, source: &ImageSource) -> Result<OutputMedium> {
        let format = ImageFormat::of_source(source);
        WriteCapabilities::current_platform()
            .custom_image_medium(format)
            .ok_or(Error::UnsupportedCustomImage(format))
    }

    /// Returns the sizes a user-supplied image is rasterized to on this platform.
    ///
    /// Pass the returned specs to [`RasterCustomizer::from_image()`] or
    /// [`IconSet::from_image_source()`](folco_renderer::IconSet::from_image_source)
    /// to generate platform-compatible sizes.
    pub fn custom_image_size_spec(&self) -> PlatformSizeSpec {
        PlatformSizeSpec::custom_image()
    }

    /// Builds a customizer over `source` in the medium this platform writes it in.
    ///
    /// # Errors
    ///
    /// Returns an error if `source` can't be written on this platform or can't
    /// be decoded.
    fn custom_image_customizer(&self, source: &ImageSource) -> Result<Customizer> {
        let medium = self.custom_image_medium(source)?;
        Ok(match (medium, source) {
            (OutputMedium::Vector, ImageSource::Svg(svg)) => {
                SvgCustomizer::from_svg(svg.resolve()?)?.into()
            }
            _ => {
                let specs = self.custom_image_size_spec();
                RasterCustomizer::from_image(source, specs.sizes())?.into()
            }
        })
    }

    /// Uses `source` as the folder icon, decorated per `profile`, on every folder.
    ///
    /// Written in the medium [`WriteCapabilities::custom_image_medium`] picks
    /// for `source`'s format: SVG stays SVG where the platform allows it.
    /// Surface-relative intents (`solid_color`, `decal`) are ignored: a custom
    /// image has no surface color.
    ///
    /// # Errors
    ///
    /// Returns an error, before any folder is touched, if `source` can't be
    /// written on this platform or can't be decoded.
    pub async fn customize_folders_with_image_async<P: AsRef<Path>>(
        &self,
        folders: Vec<P>,
        source: &ImageSource,
        profile: CustomizationProfile,
        progress: ProgressSender,
    ) -> Result<()> {
        let mut customizer = self.custom_image_customizer(source)?;
        customizer.apply_profile(profile);
        render_and_write(
            &self.folder_provider,
            folders,
            || Ok(customizer.render_output()?.into_sys()),
            progress,
        )
        .await;
        Ok(())
    }
}
