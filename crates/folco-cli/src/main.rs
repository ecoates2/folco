use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};

use folco_core::{
    CustomizationContextBuilder, CustomizationProfile, DecalConfig, DefaultPlacements, ImageFile,
    ImageFormat, ImageOverlayConfig, ImageSource, LayerKind, OutputMedium, OverlayAnchorMode,
    OverlayPosition, Placement, SvgSource, WriteCapabilities,
    folder_color::{FolderColor, FolderColorExt},
    progress::{Progress, ProgressSender, progress_channel},
};

// ============================================================================
// Command line
// ============================================================================

#[derive(Parser)]
#[command(name = "folco")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Show full error chains instead of just the root cause
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Enable experimental options (also enabled by FOLCO_EXPERIMENTAL=1).
    /// Currently, this includes changing overlay/color dot size/position/corner anchoring settings
    #[arg(long, global = true)]
    experimental: bool,

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
        /// Customize a folder's appearance using the system or active DE theme's folder icon as the base image
        #[arg(long)]
        folder: bool,

        /// Customize a folder's appearance using a custom image as the base icon.
        /// Accepts an SVG file path, raw SVG markup, or raster image path.
        /// Emoji characters/emoji names are also supported, but untested
        #[arg(long, value_name = "SOURCE")]
        custom_icon: Option<String>,

        // === Folder-mode JSON profile ===
        /// JSON-serialized CustomizationProfile for folder mode
        /// (alternative to individual --color / --decal / --overlay options)
        #[arg(long, value_name = "JSON", conflicts_with = "custom_icon")]
        folder_customization_profile: Option<String>,

        // === Custom-icon JSON profile ===
        /// JSON-serialized CustomizationProfile for custom-icon mode
        /// (alternative to individual --overlay options). The solidColor and
        /// decal entries are ignored
        #[arg(long, value_name = "JSON", requires = "custom_icon")]
        custom_icon_profile: Option<String>,

        #[command(flatten)]
        layers: LayerArgs,
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

/// The per-layer flags shared by both customize modes.
///
/// Turned into a [`CustomizationProfile`] by [`into_profile`](Self::into_profile),
/// which is where the mode-specific rules live.
///
/// Folder-only flags are `conflicts_with = "custom_icon"` rather than
/// `requires = "folder"`: `--folder` is a `SetTrue` flag, which clap counts as
/// always present through its default, so `requires` on it never fires.
#[derive(Args, Debug, Default)]
struct LayerArgs {
    // === Solid Color Options (folder-mode only) ===
    /// Recolor the whole folder icon to this color by performing an HSL shift against computed averages.
    /// Only supported on Windows
    #[arg(long, value_enum, value_name = "COLOR", conflicts_with = "custom_icon")]
    color: Option<FolderColor>,

    // === Color Dot Options (both modes) ===
    /// Add a colored dot badge. Centered on Windows and placed at the bottom right on Linux
    #[arg(long, value_enum, value_name = "COLOR")]
    color_dot: Option<FolderColor>,

    /// Color dot position [experimental; default depends on platform]
    #[arg(long, value_name = "POSITION", hide = true)]
    color_dot_position: Option<PositionArg>,

    /// Color dot anchor mode [experimental; default: inset]
    #[arg(long, value_name = "MODE", hide = true)]
    color_dot_anchor_mode: Option<AnchorModeArg>,

    /// Color dot scale factor (0.0-1.0) [experimental; default depends on platform]
    #[arg(long, value_name = "SCALE", hide = true)]
    color_dot_scale: Option<f32>,

    // === Decal Options (folder-mode only) ===
    /// Decal source: an SVG file path or raw SVG markup.
    /// This gets centered on the folder and tinted to a slightly darker color, making it appear imprinted.
    /// This is only supported on Windows
    #[arg(long, value_name = "SOURCE", conflicts_with = "custom_icon")]
    decal: Option<String>,

    /// Decal scale factor (0.0-1.0)
    #[arg(
        long,
        value_name = "SCALE",
        default_value = "0.70",
        conflicts_with = "custom_icon"
    )]
    decal_scale: f32,

    // === Overlay Options (both modes) ===
    /// Overlay source: an SVG file path, raw SVG markup, emoji character, or
    /// emoji name (e.g. "duck").
    /// See https://emojibase.dev/emojis for accepted emoji names
    #[arg(long, value_name = "SOURCE")]
    overlay: Option<String>,

    /// Overlay position [experimental; default depends on the icon medium]
    #[arg(long, value_name = "POSITION", hide = true)]
    overlay_position: Option<PositionArg>,

    /// Overlay anchor mode [experimental; default: inset]
    #[arg(long, value_name = "MODE", hide = true)]
    overlay_anchor_mode: Option<AnchorModeArg>,

    /// Overlay scale factor (0.0-1.0) [experimental; default depends on the icon medium]
    #[arg(long, value_name = "SCALE", hide = true)]
    overlay_scale: Option<f32>,
}

/// Which base the customization targets, which fixes the placement defaults
/// and which flags are meaningful.
#[derive(Debug, Clone, Copy)]
enum Mode {
    /// The system folder icon.
    Folder,
    /// A user-supplied image, written in `medium`.
    CustomIcon { medium: OutputMedium },
}

impl Mode {
    fn default_placements(self) -> DefaultPlacements {
        match self {
            Mode::Folder => OutputMedium::system_folder_icon().default_placements(),
            Mode::CustomIcon { medium } => medium.default_placements(),
        }
    }
}

impl LayerArgs {
    /// Flags that are hidden behind `--experimental`, with whether each was set.
    fn experimental_flags(&self) -> [(&'static str, bool); 6] {
        [
            ("--overlay-position", self.overlay_position.is_some()),
            ("--overlay-anchor-mode", self.overlay_anchor_mode.is_some()),
            ("--overlay-scale", self.overlay_scale.is_some()),
            ("--color-dot-position", self.color_dot_position.is_some()),
            (
                "--color-dot-anchor-mode",
                self.color_dot_anchor_mode.is_some(),
            ),
            ("--color-dot-scale", self.color_dot_scale.is_some()),
        ]
    }

    /// Builds the profile these flags describe.
    ///
    /// Unset placement flags fall back to the defaults for `mode`'s medium.
    /// Folder mode requires at least one layer; custom-icon mode allows none
    /// (the image itself is the customization). Surface-relative flags
    /// conflict with `--custom-icon` at the parser, so they are always `None`
    /// here in custom-icon mode.
    fn into_profile(self, mode: Mode, experimental: bool) -> Result<CustomizationProfile> {
        for (flag, set) in self.experimental_flags() {
            if set && !experimental {
                bail!(
                    "{flag} is experimental; pass --experimental or set {}=1",
                    folco_core::experimental::ENV_VAR
                );
            }
        }

        let defaults = mode.default_placements();
        let mut profile = CustomizationProfile::new();

        if let Some(color) = self.color {
            profile = profile.with_solid_color(color.to_solid_color_config());
        }

        if let Some(color) = self.color_dot {
            let placement = Placement::new(
                self.color_dot_position
                    .map_or(defaults.color_dot.position, Into::into),
                self.color_dot_anchor_mode.unwrap_or_default().into(),
                self.color_dot_scale.unwrap_or(defaults.color_dot.scale),
            );
            profile = profile.with_color_dot(color.to_color_dot_config().with_placement(placement));
        }

        if let Some(source) = &self.decal {
            let svg = resolve_svg_source(source)?;
            profile = profile.with_decal(DecalConfig::new(svg, self.decal_scale));
        }

        if let Some(source) = &self.overlay {
            let source = resolve_overlay_source(source)?;
            let placement = Placement::new(
                self.overlay_position
                    .map_or(defaults.overlay.position, Into::into),
                self.overlay_anchor_mode.unwrap_or_default().into(),
                self.overlay_scale.unwrap_or(defaults.overlay.scale),
            );
            profile = profile.with_overlay(ImageOverlayConfig::new(source, placement));
        }

        if matches!(mode, Mode::Folder) && LayerKind::ALL.iter().all(|&l| !profile.carries(l)) {
            bail!(
                "--folder mode requires at least one of --color, --color-dot, \
                 --decal, --overlay, or --folder-customization-profile"
            );
        }

        Ok(profile)
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, Default)]
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

#[derive(Clone, Copy, Debug, ValueEnum, Default)]
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

// ============================================================================
// Source resolution
// ============================================================================

/// The flag that gets closest to `layer` on every icon medium.
fn alternative_flag(layer: LayerKind) -> &'static str {
    match layer {
        LayerKind::SolidColor => "--color-dot",
        LayerKind::Decal => "--overlay",
        LayerKind::ColorDot | LayerKind::Overlay => "--color-dot",
    }
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

/// Resolve an overlay source string to an [`ImageSource`].
///
/// Accepted inputs (checked in order):
///
/// 1. Raw SVG markup (starts with `<`)
/// 2. Existing SVG or raster image file (see [`ImageFile`])
/// 3. Emoji character (contains emoji codepoints)
/// 4. Fallback: treated as an emoji name (e.g. "duck", "star", "heart")
fn resolve_overlay_source(input: &str) -> Result<ImageSource> {
    let trimmed = input.trim();

    // Raw SVG markup
    if trimmed.starts_with('<') {
        return Ok(ImageSource::from(SvgSource::Raw(trimmed.to_string())));
    }

    let path = Path::new(trimmed);
    if ImageFile::is_supported(path) && path.exists() {
        return Ok(ImageFile::read(path)?.into());
    }

    // Emoji character (contains actual emoji codepoints)
    if looks_like_emoji(trimmed) {
        return Ok(ImageSource::from(SvgSource::Emoji(trimmed.to_string())));
    }

    // Fallback: treat as an emoji name (e.g. "duck", "star", "heart")
    Ok(ImageSource::from(SvgSource::EmojiName(trimmed.to_string())))
}

// ============================================================================
// Progress display
// ============================================================================

/// Runs `op` while mirroring its [`Progress`] stream onto a terminal bar.
///
/// `processing` is the verb shown per folder ("Processing", "Resetting").
/// The bar is finished by the stream's `Completed` event, which core always
/// sends — including after a render failure — so the handler drains cleanly.
async fn run_with_progress_bar<F, Fut>(
    total: usize,
    verbose: bool,
    processing: &'static str,
    op: F,
) -> Result<()>
where
    F: FnOnce(ProgressSender) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let (tx, mut rx) = progress_channel(32);
    let pb = create_progress_bar(total as u64);

    let handler = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            update_bar(&pb, progress, processing, verbose);
        }
    });

    let result = op(tx).await;
    handler.await?;
    result
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

fn update_bar(pb: &ProgressBar, progress: Progress, processing: &str, verbose: bool) {
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
                    eprintln!("Render failed: {error}");
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
            pb.set_message(format!("{processing}: {name}"));
        }
        Progress::FolderComplete { .. } => {
            pb.inc(1);
        }
        Progress::FolderFailed { path, error, .. } => {
            pb.inc(1);
            pb.suspend(|| {
                if verbose {
                    eprintln!("Failed {}: {error}", path.display());
                } else {
                    eprintln!("Failed {}", path.display());
                }
            });
        }
        Progress::Completed { succeeded, failed } => {
            pb.finish_with_message(format!("Completed: {succeeded} succeeded, {failed} failed"));
        }
    }
}

// ============================================================================
// Entry point
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let experimental = cli.experimental || folco_core::experimental::enabled_from_env();

    match cli.command {
        Commands::Customize {
            directories,
            folder,
            custom_icon,
            folder_customization_profile,
            custom_icon_profile,
            layers,
        } => {
            if folder {
                let profile = match folder_customization_profile {
                    Some(json) => CustomizationProfile::from_json(&json)
                        .context("Failed to parse CustomizationProfile JSON")?,
                    None => layers.into_profile(Mode::Folder, experimental)?,
                };
                customize_folders(directories, profile, cli.verbose).await?;
            } else if let Some(source) = custom_icon {
                let image_source = resolve_overlay_source(&source)
                    .context("Failed to resolve --custom-icon source")?;

                let format = ImageFormat::of_source(&image_source);
                let caps = WriteCapabilities::current_platform();
                let Some(medium) = caps.custom_image_medium(format) else {
                    bail!(
                        "--custom-icon: {format} images can't be used as folder icons on this system \
                         (supported file types: {})",
                        caps.custom_image_extensions().join(", ")
                    );
                };

                let profile = match custom_icon_profile {
                    Some(json) => CustomizationProfile::from_json(&json)
                        .context("Failed to parse CustomizationProfile JSON")?,
                    None => layers.into_profile(Mode::CustomIcon { medium }, experimental)?,
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

    // Applying anyway would write a stock-looking icon and report success.
    let unsupported = ctx.unsupported_layers(&profile);
    if !unsupported.is_empty() {
        bail!(
            "This system can't apply every layer in the profile:\n{}",
            unsupported
                .iter()
                .map(|entry| format!(
                    "  - {} ({}) — use {} instead",
                    entry.layer,
                    entry.reason,
                    alternative_flag(entry.layer)
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    run_with_progress_bar(directories.len(), verbose, "Processing", |tx| async {
        ctx.customize_folders_async(directories, profile, tx).await;
        Ok(())
    })
    .await
}

async fn customize_custom_icon(
    directories: Vec<PathBuf>,
    image_source: ImageSource,
    profile: CustomizationProfile,
    verbose: bool,
) -> Result<()> {
    println!("Initializing...");

    let ctx = CustomizationContextBuilder::new()
        .build()
        .context("Failed to initialize customization context")?;

    run_with_progress_bar(directories.len(), verbose, "Processing", |tx| async {
        ctx.customize_folders_with_image_async(directories, &image_source, profile, tx)
            .await
            .context("Failed to apply custom icon")
    })
    .await
}

async fn reset_folders(directories: Vec<PathBuf>, verbose: bool) -> Result<()> {
    println!("Initializing...");

    let ctx = CustomizationContextBuilder::new()
        .build()
        .context("Failed to initialize customization context")?;

    run_with_progress_bar(directories.len(), verbose, "Resetting", |tx| async {
        ctx.reset_folders_async(directories, tx).await;
        Ok(())
    })
    .await
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses a full `folco customize ...` command line and returns the layer flags.
    fn layer_args(args: &[&str]) -> LayerArgs {
        let cli = Cli::try_parse_from(["folco", "customize"].iter().chain(args))
            .unwrap_or_else(|e| panic!("{e}"));
        match cli.command {
            Commands::Customize { layers, .. } => layers,
            _ => unreachable!(),
        }
    }

    fn parse_error(args: &[&str]) -> String {
        Cli::try_parse_from(["folco", "customize"].iter().chain(args))
            .err()
            .expect("expected a parse error")
            .to_string()
    }

    const RASTER_CUSTOM: Mode = Mode::CustomIcon {
        medium: OutputMedium::Raster,
    };

    #[test]
    fn color_maps_to_solid_color() {
        let profile = layer_args(&["--folder", "--color", "red", "d"])
            .into_profile(Mode::Folder, false)
            .unwrap();
        let solid = profile.solid_color.unwrap();
        assert_eq!(
            (solid.target_r, solid.target_g, solid.target_b),
            FolderColor::Red.rgb()
        );
        assert!(profile.color_dot.is_none());
    }

    #[test]
    fn color_dot_uses_the_medium_defaults_when_unset() {
        let profile = layer_args(&["--folder", "--color-dot", "blue", "d"])
            .into_profile(Mode::Folder, false)
            .unwrap();
        let expected = OutputMedium::system_folder_icon()
            .default_placements()
            .color_dot;
        assert_eq!(profile.color_dot.unwrap().placement, expected);
    }

    #[test]
    fn decal_accepts_raw_markup_and_scale() {
        let profile = layer_args(&["--folder", "--decal", "<svg/>", "--decal-scale", "0.5", "d"])
            .into_profile(Mode::Folder, false)
            .unwrap();
        let decal = profile.decal.unwrap();
        assert_eq!(decal.source, SvgSource::Raw("<svg/>".into()));
        assert_eq!(decal.scale, 0.5);
    }

    #[test]
    fn overlay_accepts_an_emoji_name() {
        let profile = layer_args(&["--folder", "--overlay", "duck", "d"])
            .into_profile(Mode::Folder, false)
            .unwrap();
        assert_eq!(
            profile.overlay.unwrap().source,
            ImageSource::Svg(SvgSource::EmojiName("duck".into()))
        );
    }

    #[test]
    fn placement_flags_need_experimental() {
        let args = || {
            layer_args(&[
                "--folder",
                "--overlay",
                "duck",
                "--overlay-scale",
                "0.3",
                "d",
            ])
        };

        let err = args().into_profile(Mode::Folder, false).unwrap_err();
        assert!(err.to_string().contains("--overlay-scale is experimental"));

        let profile = args().into_profile(Mode::Folder, true).unwrap();
        assert_eq!(profile.overlay.unwrap().placement.scale, 0.3);
    }

    #[test]
    fn explicit_placement_overrides_the_defaults() {
        let profile = layer_args(&[
            "--folder",
            "--color-dot",
            "red",
            "--color-dot-position",
            "top-left",
            "--color-dot-anchor-mode",
            "centered",
            "d",
        ])
        .into_profile(Mode::Folder, true)
        .unwrap();
        let placement = profile.color_dot.unwrap().placement;
        assert_eq!(placement.position, OverlayPosition::TopLeft);
        assert_eq!(placement.anchor_mode, OverlayAnchorMode::Centered);
    }

    #[test]
    fn folder_mode_requires_a_layer() {
        let err = layer_args(&["--folder", "d"])
            .into_profile(Mode::Folder, false)
            .unwrap_err();
        assert!(err.to_string().contains("requires at least one of"));
    }

    #[test]
    fn custom_icon_mode_allows_no_layers() {
        let profile = layer_args(&["--custom-icon", "<svg/>", "d"])
            .into_profile(RASTER_CUSTOM, false)
            .unwrap();
        assert!(LayerKind::ALL.iter().all(|&l| !profile.carries(l)));
    }

    #[test]
    fn custom_icon_mode_uses_its_medium_defaults() {
        let raster = layer_args(&["--custom-icon", "<svg/>", "--color-dot", "red", "d"])
            .into_profile(RASTER_CUSTOM, false)
            .unwrap();
        let vector = layer_args(&["--custom-icon", "<svg/>", "--color-dot", "red", "d"])
            .into_profile(
                Mode::CustomIcon {
                    medium: OutputMedium::Vector,
                },
                false,
            )
            .unwrap();

        assert_eq!(
            raster.color_dot.unwrap().placement,
            DefaultPlacements::RASTER.color_dot
        );
        assert_eq!(
            vector.color_dot.unwrap().placement,
            DefaultPlacements::VECTOR.color_dot
        );
    }

    #[test]
    fn surface_relative_flags_are_rejected_without_folder() {
        // Regression: `requires = "folder"` silently passed because `--folder`
        // is a SetTrue flag with an implicit default.
        assert!(
            parse_error(&["--custom-icon", "<svg/>", "--color", "red", "d"])
                .contains("cannot be used with")
        );
        assert!(
            parse_error(&["--custom-icon", "<svg/>", "--decal", "<svg/>", "d"])
                .contains("cannot be used with")
        );
        assert!(
            parse_error(&["--custom-icon", "<svg/>", "--decal-scale", "0.5", "d"])
                .contains("cannot be used with")
        );
        // The default decal scale must not count as "using" the flag.
        layer_args(&["--custom-icon", "<svg/>", "d"]);
    }

    #[test]
    fn exactly_one_mode_is_required() {
        assert!(parse_error(&["d"]).contains("required"));
        assert!(
            parse_error(&["--folder", "--custom-icon", "<svg/>", "d"])
                .contains("cannot be used with")
        );
    }
}
