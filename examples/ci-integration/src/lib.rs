use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

/// CI Integration Example
/// Demonstrates size budget enforcement in CI/CD pipelines

#[derive(Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: String,
    pub optimized: bool,
    pub size_kb: f64,
}

/// Get build information
#[wasm_bindgen]
pub fn get_build_info() -> Result<JsValue, JsValue> {
    let info = BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        optimized: cfg!(not(debug_assertions)),
        size_kb: 0.0, // This would be populated by the build system
    };
    
    serde_wasm_bindgen::to_value(&info)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Process data (example computation)
#[wasm_bindgen]
pub fn process_data(input: &[u8]) -> Vec<u8> {
    // Simple transformation for demo purposes
    input.iter()
        .map(|&b| b.wrapping_add(1))
        .collect()
}

/// Health check endpoint
#[wasm_bindgen]
pub fn health_check() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_data() {
        let input = vec![1, 2, 3];
        let output = process_data(&input);
        assert_eq!(output, vec![2, 3, 4]);
    }

    #[test]
    fn test_health_check() {
        assert!(health_check());
    }
}
