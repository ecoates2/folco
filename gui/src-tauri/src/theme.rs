use std::fs;

use serde::Deserialize;
use tauri::utils::config::Color;
use tauri::{AppHandle, Manager};

/// Name of the cache file holding the last resolved UI theme.
const CACHE_FILE: &str = "startup-theme";

/// The UI theme that was in effect the last time the app ran.
///
/// The window's background colour is painted by the compositor before the webview
/// has rendered anything. If that colour doesn't match the app's own background,
/// the user sees a flash on every launch. The frontend resolves its theme from
/// `localStorage` plus the system preference, neither of which the Rust side can
/// read before a window exists, so the resolved value is cached here instead.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StartupTheme {
    Light,
    Dark,
}

impl StartupTheme {
    /// The window background colour, mirroring `--background` in `gui/src/app.css`.
    pub fn background_color(self) -> Color {
        match self {
            Self::Light => Color(255, 255, 255, 255),
            Self::Dark => Color(2, 6, 23, 255),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// Reads the cached theme, falling back to light on first run or any read failure.
pub fn read(app: &AppHandle) -> StartupTheme {
    let Ok(dir) = app.path().app_config_dir() else {
        return StartupTheme::Light;
    };

    match fs::read_to_string(dir.join(CACHE_FILE))
        .as_deref()
        .map(str::trim)
    {
        Ok("dark") => StartupTheme::Dark,
        _ => StartupTheme::Light,
    }
}

/// Caches the resolved theme for the next launch.
pub fn write(app: &AppHandle, theme: StartupTheme) -> Result<(), String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config directory: {e}"))?;

    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app config directory: {e}"))?;

    fs::write(dir.join(CACHE_FILE), theme.as_str())
        .map_err(|e| format!("Failed to cache startup theme: {e}"))
}
