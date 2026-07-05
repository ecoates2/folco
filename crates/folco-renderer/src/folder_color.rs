//! Named folder color presets with target RGB colors.
//!
//! Each [`FolderColor`] variant maps to a target RGB color. When applied,
//! the renderer computes the necessary deltas from the base icon's surface
//! color to produce the target appearance.
//!
//! # Usage
//!
//! ```
//! use folco_renderer::folder_color::FolderColor;
//!
//! let color = FolderColor::Red;
//! let config = color.to_folder_color_target_config();
//! // config can be embedded in a CustomizationProfile
//! ```
//!
//! The full list of available colors, including their target RGB values,
//! can be serialized to JSON via [`FolderColor::all_with_metadata`] so that
//! a frontend can present a color picker.

use serde::{Deserialize, Serialize};

use crate::layer::FolderColorTargetConfig;

/// A named folder color preset.
///
/// Each variant maps to a target RGB color that recolors a standard
/// system folder icon to the desired color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "kebab-case")]
pub enum FolderColor {
    Red,
    Pink,
    Purple,
    DeepPurple,
    Indigo,
    Blue,
    LightBlue,
    Cyan,
    Teal,
    Green,
    LightGreen,
    Lime,
    Yellow,
    Amber,
    Orange,
    DeepOrange,
    Brown,
    Grey,
    BlueGrey,
    White,
    Black,
}

impl FolderColor {
    /// Returns all available folder color presets.
    pub fn all() -> &'static [FolderColor] {
        &[
            FolderColor::Red,
            FolderColor::Pink,
            FolderColor::Purple,
            FolderColor::DeepPurple,
            FolderColor::Indigo,
            FolderColor::Blue,
            FolderColor::LightBlue,
            FolderColor::Cyan,
            FolderColor::Teal,
            FolderColor::Green,
            FolderColor::LightGreen,
            FolderColor::Lime,
            FolderColor::Yellow,
            FolderColor::Amber,
            FolderColor::Orange,
            FolderColor::DeepOrange,
            FolderColor::Brown,
            FolderColor::Grey,
            FolderColor::BlueGrey,
            FolderColor::White,
            FolderColor::Black,
        ]
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            FolderColor::Red => "Red",
            FolderColor::Pink => "Pink",
            FolderColor::Purple => "Purple",
            FolderColor::DeepPurple => "Deep Purple",
            FolderColor::Indigo => "Indigo",
            FolderColor::Blue => "Blue",
            FolderColor::LightBlue => "Light Blue",
            FolderColor::Cyan => "Cyan",
            FolderColor::Teal => "Teal",
            FolderColor::Green => "Green",
            FolderColor::LightGreen => "Light Green",
            FolderColor::Lime => "Lime",
            FolderColor::Yellow => "Yellow",
            FolderColor::Amber => "Amber",
            FolderColor::Orange => "Orange",
            FolderColor::DeepOrange => "Deep Orange",
            FolderColor::Brown => "Brown",
            FolderColor::Grey => "Grey",
            FolderColor::BlueGrey => "Blue Grey",
            FolderColor::White => "White",
            FolderColor::Black => "Black",
        }
    }

    /// Converts this color preset to a color target config.
    ///
    /// The returned config contains the **target RGB** color, ready to
    /// embed in a [`CustomizationProfile`](crate::CustomizationProfile).
    /// The renderer computes the necessary deltas from the base icon's
    /// surface color.
    pub fn to_folder_color_target_config(&self) -> FolderColorTargetConfig {
        let (r, g, b) = self.rgb();
        FolderColorTargetConfig::new(r, g, b)
    }

    /// Returns the `(r, g, b)` tuple for this color preset.
    ///
    /// Values are in the range 0–255.
    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            FolderColor::Red =>              (244,  67,  54),
            FolderColor::Pink =>             (233,  30,  99),
            FolderColor::Purple =>           (156,  39, 176),
            FolderColor::DeepPurple =>       (103,  58, 183),
            FolderColor::Indigo =>           ( 63,  81, 181),
            FolderColor::Blue =>             ( 33, 150, 243),
            FolderColor::LightBlue =>        (  3, 169, 244),
            FolderColor::Cyan =>             (  0, 188, 212),
            FolderColor::Teal =>             (  0, 150, 136),
            FolderColor::Green =>            ( 76, 175,  80),
            FolderColor::LightGreen =>       (139, 195,  74),
            FolderColor::Lime =>             (205, 220,  57),
            FolderColor::Yellow =>           (255, 235,  59),
            FolderColor::Amber =>            (255, 193,   7),
            FolderColor::Orange =>           (255, 152,   0),
            FolderColor::DeepOrange =>       (255,  87,  34),
            FolderColor::Brown =>            (121,  85,  72),
            FolderColor::Grey =>             (158, 158, 158),
            FolderColor::BlueGrey =>         ( 96, 125, 139),
            FolderColor::White =>            (237, 237, 237),
            FolderColor::Black =>            ( 64,  64,  64),
        }
    }

    /// Returns all color presets with their metadata, suitable for
    /// serializing to JSON and sending to a frontend.
    pub fn all_with_metadata() -> Vec<FolderColorMetadata> {
        Self::all()
            .iter()
            .map(|c| {
                let (r, g, b) = c.rgb();
                FolderColorMetadata {
                    id: *c,
                    display_name: c.display_name().to_string(),
                    r,
                    g,
                    b,
                }
            })
            .collect()
    }

    /// Serializes all color presets (with metadata) to a JSON string.
    pub fn all_metadata_json() -> Result<String, serde_json::Error> {
        serde_json::to_string(&Self::all_with_metadata())
    }

    /// Pretty-printed variant of [`all_metadata_json`](Self::all_metadata_json).
    pub fn all_metadata_json_pretty() -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&Self::all_with_metadata())
    }
}

/// Metadata for a single folder color preset, including its RGB values.
///
/// Serialized to JSON as:
/// ```json
/// {
///   "id": "red",
///   "displayName": "Red",
///   "r": 244,
///   "g": 67,
///   "b": 54
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct FolderColorMetadata {
    /// Machine-readable color identifier (kebab-case).
    pub id: FolderColor,
    /// Human-readable display name.
    pub display_name: String,
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
}

#[cfg(feature = "clap")]
impl clap::ValueEnum for FolderColor {
    fn value_variants<'a>() -> &'a [Self] {
        Self::all()
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let name = match self {
            FolderColor::Red => "red",
            FolderColor::Pink => "pink",
            FolderColor::Purple => "purple",
            FolderColor::DeepPurple => "deep-purple",
            FolderColor::Indigo => "indigo",
            FolderColor::Blue => "blue",
            FolderColor::LightBlue => "light-blue",
            FolderColor::Cyan => "cyan",
            FolderColor::Teal => "teal",
            FolderColor::Green => "green",
            FolderColor::LightGreen => "light-green",
            FolderColor::Lime => "lime",
            FolderColor::Yellow => "yellow",
            FolderColor::Amber => "amber",
            FolderColor::Orange => "orange",
            FolderColor::DeepOrange => "deep-orange",
            FolderColor::Brown => "brown",
            FolderColor::Grey => "grey",
            FolderColor::BlueGrey => "blue-grey",
            FolderColor::White => "white",
            FolderColor::Black => "black",
        };

        let (r, g, b) = self.rgb();
        let help = format!(
            "\x1b[48;2;{r};{g};{b}m  \x1b[0m {}",
            self.display_name()
        );

        Some(clap::builder::PossibleValue::new(name).help(help))
    }
}

impl std::fmt::Display for FolderColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

impl std::str::FromStr for FolderColor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(' ', "").replace('-', "").replace('_', "").as_str() {
            "red" => Ok(FolderColor::Red),
            "pink" => Ok(FolderColor::Pink),
            "purple" => Ok(FolderColor::Purple),
            "deeppurple" => Ok(FolderColor::DeepPurple),
            "indigo" => Ok(FolderColor::Indigo),
            "blue" => Ok(FolderColor::Blue),
            "lightblue" => Ok(FolderColor::LightBlue),
            "cyan" => Ok(FolderColor::Cyan),
            "teal" => Ok(FolderColor::Teal),
            "green" => Ok(FolderColor::Green),
            "lightgreen" => Ok(FolderColor::LightGreen),
            "lime" => Ok(FolderColor::Lime),
            "yellow" => Ok(FolderColor::Yellow),
            "amber" => Ok(FolderColor::Amber),
            "orange" => Ok(FolderColor::Orange),
            "deeporange" => Ok(FolderColor::DeepOrange),
            "brown" => Ok(FolderColor::Brown),
            "grey" | "gray" => Ok(FolderColor::Grey),
            "bluegrey" | "bluegray" => Ok(FolderColor::BlueGrey),
            "white" => Ok(FolderColor::White),
            "black" => Ok(FolderColor::Black),
            _ => Err(format!("Unknown folder color: '{}'", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_colors_have_valid_params() {
        for color in FolderColor::all() {
            let (_r, _g, _b) = color.rgb();
        }
    }

    #[test]
    fn metadata_json_roundtrip() {
        let json = FolderColor::all_metadata_json().unwrap();
        let parsed: Vec<FolderColorMetadata> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), FolderColor::all().len());
    }

    #[test]
    fn parse_color_names() {
        assert_eq!("red".parse::<FolderColor>().unwrap(), FolderColor::Red);
        assert_eq!(
            "deep-purple".parse::<FolderColor>().unwrap(),
            FolderColor::DeepPurple
        );
        assert_eq!(
            "BlueGrey".parse::<FolderColor>().unwrap(),
            FolderColor::BlueGrey
        );
        assert_eq!(
            "light_blue".parse::<FolderColor>().unwrap(),
            FolderColor::LightBlue
        );
        assert!("invalid".parse::<FolderColor>().is_err());
    }

    #[test]
    fn to_folder_color_target_config() {
        let config = FolderColor::Red.to_folder_color_target_config();
        assert_eq!(config.target_r, 244);
        assert_eq!(config.target_g, 67);
        assert_eq!(config.target_b, 54);
    }
}
