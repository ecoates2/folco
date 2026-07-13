mod canvas;
mod serializable;

pub use canvas::CanvasRenderer;
pub use serializable::{SerializableFolderIconBase, SerializableIconImage};

use wasm_bindgen::prelude::*;

// Re-export shared model/renderer types so their tsify-generated TypeScript
// definitions are included in the wasm-pack output `.d.ts` file.
pub use folco_renderer::{FolderColor, FolderColorMetadata, SurfaceColor};

/// Returns all available folder color presets with their metadata.
#[wasm_bindgen(js_name = "getAvailableColors")]
pub fn get_available_colors() -> Result<JsValue, JsError> {
    let colors = FolderColor::all_with_metadata();
    serde_wasm_bindgen::to_value(&colors).map_err(|e| JsError::new(&e.to_string()))
}
