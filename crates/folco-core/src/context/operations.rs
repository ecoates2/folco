//! Applying and resetting folder icons, with progress reporting.
//!
//! Every operation streams [`Progress`] events: `Started`, then (for renders)
//! `Rendering` or `RenderFailed`, then `Processing` / `FolderComplete` /
//! `FolderFailed` per folder, then `Completed` with the tally. Consumers end a
//! run on `Completed`, not on the future resolving.

use std::fmt::Display;
use std::path::Path;

use folco_renderer::CustomizationProfile;
use icon_sys::IconSet as SysIconSet;
use icon_sys::folder_settings::{FolderSettingsProvider, PlatformFolderSettingsProvider};

use super::CustomizationContext;
use crate::convert::SystemFormat;
use crate::error::Result;
use crate::progress::{Progress, ProgressSender};

impl CustomizationContext {
    /// Resets the icons for the specified folders to system default with progress reporting.
    ///
    /// Per-folder outcomes are reported through `progress`; the final
    /// [`Progress::Completed`] carries the success/failure counts.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use folco_core::{CustomizationContextBuilder, progress::progress_channel};
    ///
    /// let ctx = CustomizationContextBuilder::new().build()?;
    /// let (tx, mut rx) = progress_channel(32);
    ///
    /// ctx.reset_folders_async(folders, tx).await;
    /// ```
    pub async fn reset_folders_async<P: AsRef<Path>>(
        &self,
        folders: Vec<P>,
        progress: ProgressSender,
    ) {
        let _ = progress
            .send(Progress::Started {
                total: folders.len(),
            })
            .await;

        write_each(&folders, &progress, |folder| {
            self.folder_provider.reset_icon_for_folder(folder)
        })
        .await;
    }

    /// Customizes the icons for the specified folders with progress reporting.
    ///
    /// Applies the profile, renders once, then writes the result to each
    /// folder. Progress is reported through a tokio channel for GUI progress
    /// bars or CLI progress indicators.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use folco_core::{CustomizationContextBuilder, progress::progress_channel};
    ///
    /// let mut ctx = CustomizationContextBuilder::new().build()?;
    /// let (tx, mut rx) = progress_channel(32);
    ///
    /// // Handle progress in a separate task
    /// let handle = tokio::spawn(async move {
    ///     while let Some(p) = rx.recv().await {
    ///         println!("{:?}", p);
    ///     }
    /// });
    ///
    /// // Run customization
    /// ctx.customize_folders_async(folders, profile, tx).await;
    /// handle.await?;
    /// ```
    pub async fn customize_folders_async<P: AsRef<Path>>(
        &mut self,
        folders: Vec<P>,
        profile: CustomizationProfile,
        progress: ProgressSender,
    ) {
        self.apply_profile(profile);
        let customizer = &mut self.customizer;
        render_and_write(
            &self.folder_provider,
            folders,
            || Ok(customizer.render_output()?.into_sys()),
            progress,
        )
        .await;
    }
}

/// Renders once, then writes the result to each folder, reporting progress.
///
/// A render failure short-circuits: every folder is counted as failed and no
/// folder is touched.
pub(super) async fn render_and_write<P: AsRef<Path>>(
    provider: &PlatformFolderSettingsProvider,
    folders: Vec<P>,
    render: impl FnOnce() -> Result<SysIconSet>,
    progress: ProgressSender,
) {
    let total = folders.len();
    let _ = progress.send(Progress::Started { total }).await;

    let _ = progress.send(Progress::Rendering).await;
    let sys_icons = match render() {
        Ok(icons) => icons,
        Err(e) => {
            let _ = progress
                .send(Progress::RenderFailed {
                    error: e.to_string(),
                })
                .await;
            let _ = progress
                .send(Progress::Completed {
                    succeeded: 0,
                    failed: total,
                })
                .await;
            return;
        }
    };

    write_each(&folders, &progress, |folder| {
        provider.set_icon_for_folder(folder, &sys_icons)
    })
    .await;
}

/// Applies `write` to each folder in turn, reporting per-folder outcomes and
/// the final tally. The caller has already sent `Started`.
async fn write_each<P: AsRef<Path>, E: Display>(
    folders: &[P],
    progress: &ProgressSender,
    mut write: impl FnMut(&Path) -> std::result::Result<(), E>,
) {
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (index, folder) in folders.iter().enumerate() {
        let path = folder.as_ref().to_path_buf();
        let _ = progress
            .send(Progress::Processing {
                current: index,
                path: path.clone(),
            })
            .await;

        match write(folder.as_ref()) {
            Ok(()) => {
                succeeded += 1;
                let _ = progress
                    .send(Progress::FolderComplete { index, path })
                    .await;
            }
            Err(e) => {
                failed += 1;
                let _ = progress
                    .send(Progress::FolderFailed {
                        index,
                        path,
                        error: e.to_string(),
                    })
                    .await;
            }
        }
    }

    let _ = progress
        .send(Progress::Completed { succeeded, failed })
        .await;
}
