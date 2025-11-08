use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

/// Example with embedded assets that could be externalized
/// This demonstrates wasm-slim's asset detection capabilities

/// Embedded font data (simulates a real font file)
/// In production, this would be a .ttf or .woff2 file
const FONT_DATA: &[u8] = include_bytes!("../assets/font.txt");

/// Embedded JSON data
const CONFIG_DATA: &str = include_str!("../assets/data.json");

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

/// Load configuration from embedded JSON
#[wasm_bindgen]
pub fn load_config() -> Result<JsValue, JsValue> {
    let config: AppConfig = serde_json::from_str(CONFIG_DATA)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?;
    
    serde_wasm_bindgen::to_value(&config)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get font data size
#[wasm_bindgen]
pub fn get_font_size() -> usize {
    FONT_DATA.len()
}

/// Check if font is loaded
#[wasm_bindgen]
pub fn has_font() -> bool {
    !FONT_DATA.is_empty()
}

/// Get app info
#[wasm_bindgen]
pub fn get_app_info() -> String {
    format!(
        "App with {} bytes of embedded assets\n\
         - Font: {} bytes\n\
         - Config: {} bytes",
        FONT_DATA.len() + CONFIG_DATA.len(),
        FONT_DATA.len(),
        CONFIG_DATA.len()
    )
}

// NOTE: In a real application, you would externalize these assets:
//
// Instead of include_bytes!(), use:
// 1. Fetch API to load font at runtime
// 2. IndexedDB for caching
// 3. CDN for asset delivery
//
// Example (JavaScript):
// ```javascript
// const fontResponse = await fetch('/assets/font.woff2');
// const fontData = await fontResponse.arrayBuffer();
// // Use fontData...
// ```
//
// This approach would reduce the WASM bundle size from ~2MB to ~200KB!
