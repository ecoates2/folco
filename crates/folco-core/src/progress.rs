//! Progress reporting for async operations.
//!
//! This module provides types for tracking progress of long-running operations
//! like folder customization. Progress is reported via tokio channels.

use std::path::PathBuf;

/// Progress event for folder customization operations.
#[derive(Debug, Clone)]
pub enum Progress {
    /// Operation has started.
    Started {
        /// Total number of items to process.
        total: usize,
    },

    /// Rendering icons (happens once before processing folders).
    Rendering,

    /// Icon rendering failed (e.g., invalid SVG or emoji).
    RenderFailed {
        /// Error message describing why rendering failed.
        error: String,
    },

    /// Processing a specific folder.
    Processing {
        /// Current item index (0-based).
        current: usize,
        /// Path of the folder being processed.
        path: PathBuf,
    },

    /// A folder was processed successfully.
    FolderComplete {
        /// Index of the completed folder.
        index: usize,
        /// Path of the folder.
        path: PathBuf,
    },

    /// A folder failed to process.
    FolderFailed {
        /// Index of the failed folder.
        index: usize,
        /// Path of the folder.
        path: PathBuf,
        /// Error message.
        error: String,
    },

    /// All operations completed.
    Completed {
        /// Number of successful operations.
        succeeded: usize,
        /// Number of failed operations.
        failed: usize,
    },
}

/// A sender for progress updates.
///
/// This is a re-export of `tokio::sync::mpsc::Sender<Progress>` for convenience.
pub type ProgressSender = tokio::sync::mpsc::Sender<Progress>;

/// A receiver for progress updates.
///
/// This is a re-export of `tokio::sync::mpsc::Receiver<Progress>` for convenience.
pub type ProgressReceiver = tokio::sync::mpsc::Receiver<Progress>;

/// Creates a new progress channel with the given buffer size.
///
/// # Arguments
///
/// * `buffer` - The channel buffer size. A reasonable default is 32.
///
/// # Returns
///
/// A tuple of (sender, receiver) for progress events.
///
/// # Example
///
/// ```ignore
/// use folco_core::progress::{progress_channel, Progress};
///
/// let (tx, mut rx) = progress_channel(32);
///
/// // Spawn a task to handle progress
/// tokio::spawn(async move {
///     while let Some(progress) = rx.recv().await {
///         match progress {
///             Progress::Processing { current, path } => {
///                 println!("Processing {}/{}: {:?}", current + 1, total, path);
///             }
///             Progress::Completed { succeeded, failed } => {
///                 println!("Done! {} succeeded, {} failed", succeeded, failed);
///             }
///             _ => {}
///         }
///     }
/// });
/// ```
pub fn progress_channel(buffer: usize) -> (ProgressSender, ProgressReceiver) {
    tokio::sync::mpsc::channel(buffer)
}
