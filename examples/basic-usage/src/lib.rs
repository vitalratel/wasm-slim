use wasm_bindgen::prelude::*;

/// A simple greeting function for WASM
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! This WASM module was optimized with wasm-slim.", name)
}

/// Calculate Fibonacci number (demonstrates code that benefits from LTO)
#[wasm_bindgen]
pub fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

/// Example struct to show serialization overhead
#[wasm_bindgen]
pub struct Calculator {
    value: f64,
}

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Calculator {
        Calculator { value: 0.0 }
    }

    pub fn add(&mut self, x: f64) {
        self.value += x;
    }

    pub fn multiply(&mut self, x: f64) {
        self.value *= x;
    }

    pub fn get_value(&self) -> f64 {
        self.value
    }
}
