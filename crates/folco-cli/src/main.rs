use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};

use folco_core::{
    CustomIconProfile, CustomizationContextBuilder, CustomizationProfile, DecalConfig,
    ImageOverlayConfig, ImageSource, OverlayAnchorMode, OverlayPosition, SvgSource,
    folder_color::FolderColor,
    progress::{Progress, progress_channel},
};

#[derive(Parser)]
#[command(name = "folco")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Show full error chains instead of just the root cause
    #[arg(long, short, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Customize folder icons
    #[command(group(
        clap::ArgGroup::new("mode")
            .required(true)
            .args(["folder", "custom_icon"])
    ))]
    Customize {
        /// Directories to customize
        #[arg(required = true)]
        directories: Vec<PathBuf>,

        // === Mode Selection ===
        /// Customize using the system folder icon as the base image
        #[arg(long)]
        folder: bool,

        /// Customize using a custom image as the base icon.
        /// Accepts an SVG file path, raw SVG markup, raster image path,
        /// emoji character, or emoji name.
        #[arg(long, value_name = "SOURCE")]
        custom_icon: Option<String>,

        // === Folder-mode JSON profile ===
        /// JSON-serialized CustomizationProfile for folder mode
        /// (alternative to individual --color / --decal / --overlay options)
        #[arg(long, value_name = "JSON", requires = "folder")]
        folder_customization_profile: Option<String>,

        // === Custom-icon JSON profile ===
        /// JSON-serialized CustomIconProfile for custom-icon mode
        /// (alternative to individual --overlay options)
        #[arg(long, value_name = "JSON", requires = "custom_icon")]
        custom_icon_profile: Option<String>,

        // === Color Target Options (folder-mode only) ===
        /// Folder color
        #[arg(long, value_enum, value_name = "COLOR", requires = "folder")]
        color: Option<FolderColor>,

        // === Decal Options (folder-mode only) ===
        /// Decal source: an SVG file path or raw SVG markup.
        /// This gets centered on the folder and tinted to a slightly darker color.
        #[arg(long, value_name = "SOURCE", requires = "folder")]
        decal: Option<String>,

        /// Decal scale factor (0.0-1.0)
        #[arg(
            long,
            value_name = "SCALE",
            default_value = "0.70",
            requires = "folder"
        )]
        decal_scale: f32,

        // === Overlay Options (both modes) ===
        /// Overlay source: an SVG file path, raw SVG markup, emoji character, or
        /// emoji name (e.g. "duck").
        /// See https://emojibase.dev/emojis for accepted emoji names
        #[arg(long, value_name = "SOURCE")]
        overlay: Option<String>,

        /// Overlay position
        #[arg(long, value_name = "POSITION", default_value = "center")]
        overlay_position: PositionArg,

        /// Overlay anchor mode
        #[arg(long, value_name = "MODE", default_value = "inset")]
        overlay_anchor_mode: AnchorModeArg,

        /// Overlay scale factor (0.0-1.0)
        #[arg(long, value_name = "SCALE", default_value = "0.70")]
        overlay_scale: f32,
    },

    /// Reset folder icons to system default
    Reset {
        /// Directories to reset
        #[arg(required = true)]
        directories: Vec<PathBuf>,
    },

    /// Print the JSON Schema for CustomizationProfile
    Schema,
}

/// Resolve an SVG source string (for decals — only SVG file paths and raw markup).
fn resolve_svg_source(input: &str) -> Result<String> {
    let trimmed = input.trim();

    // Raw SVG markup
    if trimmed.starts_with('<') {
        return Ok(trimmed.to_string());
    }

    // File path — must be an .svg file
    let path = Path::new(trimmed);
    if path.exists() {
        match path.extension() {
            Some(ext) if ext.eq_ignore_ascii_case("svg") => {
                let contents = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read SVG file: {}", path.display()))?;
                return Ok(contents);
            }
            Some(ext) => bail!(
                "Decal source must be an SVG file, got .{} file: {}\n\
                 Hint: only SVG files are supported for decals. \
                 Use --overlay for raster images (PNG, JPG, etc.).",
                ext.to_string_lossy(),
                path.display()
            ),
            None => bail!(
                "Decal source file has no extension: {}\n\
                 Hint: decals require an .svg file.",
                path.display()
            ),
        }
    }

    bail!(
        "Could not resolve decal source {:?}: not a file path or SVG markup. \
         Raw SVG should start with '<'.",
        input
    )
}

/// Returns true if the string contains at least one emoji character.
fn looks_like_emoji(s: &str) -> bool {
    s.chars().any(|c| {
        // Common emoji ranges (supplementary symbols, emoticons, dingbats, etc.)
        matches!(c,
            '\u{200D}'              // ZWJ
            | '\u{FE0F}'           // variation selector
            | '\u{20E3}'           // combining enclosing keycap
            | '\u{2600}'..='\u{27BF}'   // misc symbols & dingbats
            | '\u{2B50}'..='\u{2B55}'   // stars, circles
            | '\u{1F000}'..='\u{1FAFF}' // all major emoji blocks
        )
    })
}

/// Image file extensions recognised as raster overlays.
const RASTER_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff", "tif"];

/// Returns true if a file extension is a supported raster image format.
fn is_raster_extension(ext: &std::ffi::OsStr) -> bool {
    RASTER_EXTENSIONS
        .iter()
        .any(|r| ext.eq_ignore_ascii_case(r))
}

/// Resolve an overlay source string to an [`ImageSource`].
///
/// Accepted inputs (checked in order):
///
/// 1. Raw SVG markup (starts with `<`)
/// 2. File path with `.svg` extension → read as SVG text
/// 3. File path with a raster extension (PNG, JPG, WEBP, …) → read as bytes
/// 4. Emoji character (contains emoji codepoints)
/// 5. Fallback: treated as an emoji name (e.g. "duck", "star", "heart")
fn resolve_overlay_source(input: &str) -> Result<ImageSource> {
    let trimmed = input.trim();

    // Raw SVG markup
    if trimmed.starts_with('<') {
        return Ok(ImageSource::from(SvgSource::Raw(trimmed.to_string())));
    }

    // File path
    let path = Path::new(trimmed);
    if let Some(ext) = path.extension() {
        if ext.eq_ignore_ascii_case("svg") && path.exists() {
            let svg = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read overlay SVG file: {}", path.display()))?;
            return Ok(ImageSource::from(SvgSource::Raw(svg)));
        }

        if is_raster_extension(ext) && path.exists() {
            let bytes = std::fs::read(path).with_context(|| {
                format!("Failed to read overlay image file: {}", path.display())
            })?;
            return Ok(ImageSource::raster(bytes));
        }
    }

    // Emoji character (contains actual emoji codepoints)
    if looks_like_emoji(trimmed) {
        return Ok(ImageSource::from(SvgSource::Emoji(trimmed.to_string())));
    }

    // Fallback: treat as an emoji name (e.g. "duck", "star", "heart")
    Ok(ImageSource::from(SvgSource::EmojiName(trimmed.to_string())))
}

#[derive(Clone, ValueEnum, Default)]
enum PositionArg {
    BottomLeft,
    #[default]
    BottomRight,
    TopLeft,
    TopRight,
    Center,
}

impl From<PositionArg> for OverlayPosition {
    fn from(pos: PositionArg) -> Self {
        match pos {
            PositionArg::BottomLeft => OverlayPosition::BottomLeft,
            PositionArg::BottomRight => OverlayPosition::BottomRight,
            PositionArg::TopLeft => OverlayPosition::TopLeft,
            PositionArg::TopRight => OverlayPosition::TopRight,
            PositionArg::Center => OverlayPosition::Center,
        }
    }
}

#[derive(Clone, ValueEnum, Default)]
enum AnchorModeArg {
    #[default]
    Inset,
    Centered,
}

impl From<AnchorModeArg> for OverlayAnchorMode {
    fn from(mode: AnchorModeArg) -> Self {
        match mode {
            AnchorModeArg::Inset => OverlayAnchorMode::Inset,
            AnchorModeArg::Centered => OverlayAnchorMode::Centered,
        }
    }
}

fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{wide_bar} {pos}/{len} {msg}")
            .expect("invalid progress bar template"),
    );
    pb
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Customize {
            directories,
            folder,
            custom_icon,
            folder_customization_profile,
            custom_icon_profile,
            color,
            decal,
            decal_scale,
            overlay,
            overlay_position,
            overlay_anchor_mode,
            overlay_scale,
        } => {
            if folder {
                // ── Folder mode ────────────────────────────────────
                let profile = if let Some(json) = folder_customization_profile {
                    CustomizationProfile::from_json(&json)
                        .context("Failed to parse CustomizationProfile JSON")?
                } else {
                    let mut p = CustomizationProfile::new();

                    if let Some(color) = color {
                        p = p.with_folder_color_target(color.to_folder_color_target_config());
                    }

                    if let Some(ref source) = decal {
                        let svg = resolve_svg_source(source)?;
                        p = p.with_decal(DecalConfig::new(svg, decal_scale));
                    }

                    if let Some(ref source) = overlay {
                        let source = resolve_overlay_source(source)?;
                        p = p.with_overlay(ImageOverlayConfig::new(
                            source,
                            overlay_position.into(),
                            overlay_anchor_mode.clone().into(),
                            overlay_scale,
                        ));
                    }

                    // Require at least one layer when not using a JSON profile
                    if p.folder_color_target.is_none() && p.decal.is_none() && p.overlay.is_none() {
                        bail!(
                            "--folder mode requires at least one of --color, --decal, \
                             --overlay, or --folder-customization-profile"
                        );
                    }

                    p
                };

                customize_folders(directories, profile, cli.verbose).await?;
            } else if let Some(ref source) = custom_icon {
                // ── Custom-icon mode ───────────────────────────────
                let image_source = resolve_overlay_source(source)
                    .context("Failed to resolve --custom-icon source")?;

                let profile = if let Some(json) = custom_icon_profile {
                    CustomIconProfile::from_json(&json)
                        .context("Failed to parse CustomIconProfile JSON")?
                } else {
                    let mut p = CustomIconProfile::new();

                    if let Some(ref source) = overlay {
                        let source = resolve_overlay_source(source)?;
                        p = p.with_overlay(ImageOverlayConfig::new(
                            source,
                            overlay_position.into(),
                            overlay_anchor_mode.clone().into(),
                            overlay_scale,
                        ));
                    }

                    p
                };

                customize_custom_icon(directories, image_source, profile, cli.verbose).await?;
            }
        }

        Commands::Reset { directories } => {
            reset_folders(directories, cli.verbose).await?;
        }

        Commands::Schema => {
            let schema = CustomizationProfile::json_schema_string()
                .context("Failed to generate JSON schema")?;
            println!("{schema}");
        }
    }

    Ok(())
}

async fn customize_folders(
    directories: Vec<PathBuf>,
    profile: CustomizationProfile,
    verbose: bool,
) -> Result<()> {
    println!("Initializing...");

    let mut ctx = CustomizationContextBuilder::new()
        .build()
        .context("Failed to initialize customization context")?;

    let (tx, mut rx) = progress_channel(32);

    let total = directories.len() as u64;
    let pb = create_progress_bar(total);

    // Spawn progress handler
    let progress_handle = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            match progress {
                Progress::Started { total } => {
                    pb.set_length(total as u64);
                    pb.set_message("Starting...");
                }
                Progress::Rendering => {
                    pb.set_message("Rendering icons...");
                }
                Progress::RenderFailed { error } => {
                    pb.suspend(|| {
                        if verbose {
                            eprintln!("Render failed: {}", error);
                        } else {
                            eprintln!("Render failed");
                        }
                    });
                }
                Progress::Processing { path, .. } => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    pb.set_message(format!("Processing: {}", name));
                }
                Progress::FolderComplete { .. } => {
                    pb.inc(1);
                }
                Progress::FolderFailed { path, error, .. } => {
                    pb.inc(1);
                    pb.suspend(|| {
                        if verbose {
                            eprintln!("Failed {}: {}", path.display(), error);
                        } else {
                            eprintln!("Failed {}", path.display());
                        }
                    });
                }
                Progress::Completed { succeeded, failed } => {
                    pb.finish_with_message(format!(
                        "Completed: {} succeeded, {} failed",
                        succeeded, failed
                    ));
                }
            }
        }
    });

    // Run customization
    ctx.customize_folders_async(directories, &profile, tx).await;

    // Wait for progress handler to finish
    progress_handle.await?;

    Ok(())
}

async fn customize_custom_icon(
    directories: Vec<PathBuf>,
    image_source: ImageSource,
    profile: CustomIconProfile,
    verbose: bool,
) -> Result<()> {
    println!("Initializing...");

    let ctx = CustomizationContextBuilder::new()
        .build()
        .context("Failed to initialize customization context")?;

    let mut customizer = ctx
        .create_custom_icon_customizer(&image_source)
        .context("Failed to create custom icon customizer")?;

    customizer.apply_profile(&profile);

    let (tx, mut rx) = progress_channel(32);

    let total = directories.len() as u64;
    let pb = create_progress_bar(total);

    // Spawn progress handler
    let progress_handle = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            match progress {
                Progress::Started { total } => {
                    pb.set_length(total as u64);
                    pb.set_message("Starting...");
                }
                Progress::Rendering => {
                    pb.set_message("Rendering icons...");
                }
                Progress::RenderFailed { error } => {
                    pb.suspend(|| {
                        if verbose {
                            eprintln!("Render failed: {}", error);
                        } else {
                            eprintln!("Render failed");
                        }
                    });
                }
                Progress::Processing { path, .. } => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    pb.set_message(format!("Processing: {}", name));
                }
                Progress::FolderComplete { .. } => {
                    pb.inc(1);
                }
                Progress::FolderFailed { path, error, .. } => {
                    pb.inc(1);
                    pb.suspend(|| {
                        if verbose {
                            eprintln!("Failed {}: {}", path.display(), error);
                        } else {
                            eprintln!("Failed {}", path.display());
                        }
                    });
                }
                Progress::Completed { succeeded, failed } => {
                    pb.finish_with_message(format!(
                        "Completed: {} succeeded, {} failed",
                        succeeded, failed
                    ));
                }
            }
        }
    });

    // Run customization
    ctx.customize_folders_custom_async(directories, &mut customizer, tx)
        .await;

    // Wait for progress handler to finish
    progress_handle.await?;

    Ok(())
}

async fn reset_folders(directories: Vec<PathBuf>, verbose: bool) -> Result<()> {
    println!("Initializing...");

    let ctx = CustomizationContextBuilder::new()
        .build()
        .context("Failed to initialize customization context")?;

    let (tx, mut rx) = progress_channel(32);

    let total = directories.len() as u64;
    let pb = create_progress_bar(total);

    // Spawn progress handler
    let progress_handle = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            match progress {
                Progress::Started { total } => {
                    pb.set_length(total as u64);
                    pb.set_message("Starting...");
                }
                Progress::Processing { path, .. } => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    pb.set_message(format!("Resetting: {}", name));
                }
                Progress::FolderComplete { .. } => {
                    pb.inc(1);
                }
                Progress::FolderFailed { path, error, .. } => {
                    pb.inc(1);
                    pb.suspend(|| {
                        if verbose {
                            eprintln!("Failed {}: {}", path.display(), error);
                        } else {
                            eprintln!("Failed {}", path.display());
                        }
                    });
                }
                Progress::Completed { succeeded, failed } => {
                    pb.finish_with_message(format!(
                        "Completed: {} succeeded, {} failed",
                        succeeded, failed
                    ));
                }
                _ => {}
            }
        }
    });

    // Run reset
    ctx.reset_folders_async(directories, tx).await;

    // Wait for progress handler to finish
    progress_handle.await?;

    Ok(())
}
