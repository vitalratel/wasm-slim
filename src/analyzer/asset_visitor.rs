//! AST visitor implementation for asset detection
//!
//! Provides a `syn::Visit` implementation to traverse Rust AST and detect
//! embedded asset inclusions (`include_bytes!`, `include_str!`).
//!
//! # Overview
//!
//! This module implements the low-level AST traversal logic for asset detection.
//! It uses the `syn` crate to parse Rust source and identify macro invocations
//! that embed assets into the binary.
//!
//! # Implementation Details
//!
//! The visitor pattern walks the AST tree and:
//! 1. Visits every expression node
//! 2. Checks for macro expressions
//! 3. Identifies `include_bytes!` and `include_str!` macros
//! 4. Extracts the file path from macro arguments
//!
//! # Examples
//!
//! ```no_run
//! use wasm_slim::analyzer::asset_visitor::AssetVisitor;
//! use syn::{File, visit::Visit};
//! use std::path::Path;
//!
//! let source_code = r#"
//!     const FONT: &[u8] = include_bytes!("font.woff2");
//! "#;
//!
//! let syntax_tree: File = syn::parse_str(source_code).unwrap();
//! let mut visitor = AssetVisitor::new(Path::new("src/main.rs"));
//! visitor.visit_file(&syntax_tree);
//!
//! assert_eq!(visitor.assets().len(), 1);
//! assert_eq!(visitor.assets()[0].0, "font.woff2");
//! ```

use std::path::Path;
use syn::visit::{self, Visit};
use syn::{Expr, Item};

/// Visitor for traversing the AST to find asset inclusions
///
/// Walks the Rust syntax tree to detect `include_bytes!` and `include_str!`
/// macro invocations, collecting file paths and detection metadata.
pub struct AssetVisitor<'a> {
    assets: Vec<(String, usize, String)>, // (path, line, method)
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> AssetVisitor<'a> {
    /// Create a new AST visitor for the given source file
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the source file being visited (for context)
    pub fn new(_source_file: &'a Path) -> Self {
        Self {
            assets: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the collected assets from the visitor
    ///
    /// Returns a vector of tuples containing:
    /// - File path (relative path from macro argument)
    /// - Line number (currently always 0, future enhancement)
    /// - Detection method ("include_bytes!" or "include_str!")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wasm_slim::analyzer::asset_visitor::AssetVisitor;
    /// # use std::path::Path;
    /// let mut visitor = AssetVisitor::new(Path::new("src/main.rs"));
    /// // ... visit AST ...
    /// for (path, line, method) in visitor.assets() {
    ///     println!("Found {} at line {} via {}", path, line, method);
    /// }
    /// ```
    pub fn assets(&self) -> &[(String, usize, String)] {
        &self.assets
    }

    /// Check if a macro is an asset inclusion macro
    ///
    /// Examines the macro path to determine if it's `include_bytes!` or `include_str!`,
    /// and if so, extracts the file path from the macro arguments.
    fn check_macro(&mut self, mac: &syn::Macro) {
        let path_str = quote::quote!(#mac.path).to_string();

        if path_str.contains("include_bytes") || path_str.contains("include_str") {
            // Extract the string literal from macro tokens
            let tokens = mac.tokens.to_string();
            if let Some(file_path) = Self::extract_string_literal(&tokens) {
                let method = if path_str.contains("include_bytes") {
                    "include_bytes!"
                } else {
                    "include_str!"
                };
                self.assets.push((file_path, 0, method.to_string())); // Line number not easily available
            }
        }
    }

    /// Extract a string literal from macro token stream
    ///
    /// Parses the token stream to find and extract the quoted string argument.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The macro token stream as a string
    ///
    /// # Returns
    ///
    /// `Some(String)` containing the extracted path, or `None` if parsing fails
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::analyzer::asset_visitor::AssetVisitor;
    ///
    /// assert_eq!(
    ///     AssetVisitor::extract_string_literal(r#""path/to/file.png""#),
    ///     Some("path/to/file.png".to_string())
    /// );
    ///
    /// assert_eq!(
    ///     AssetVisitor::extract_string_literal(r#"  "file.txt"  "#),
    ///     Some("file.txt".to_string())
    /// );
    ///
    /// assert_eq!(
    ///     AssetVisitor::extract_string_literal(""),
    ///     None
    /// );
    /// ```
    pub fn extract_string_literal(tokens: &str) -> Option<String> {
        // Remove quotes and whitespace
        let cleaned = tokens.trim().trim_matches('"').trim();
        if !cleaned.is_empty() {
            Some(cleaned.to_string())
        } else {
            None
        }
    }
}

impl<'a> Visit<'a> for AssetVisitor<'a> {
    /// Visit an expression node
    ///
    /// Checks if the expression is a macro invocation, and if so,
    /// delegates to `check_macro` for asset detection.
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Macro(macro_expr) = expr {
            self.check_macro(&macro_expr.mac);
        }
        visit::visit_expr(self, expr);
    }

    /// Visit an item node
    ///
    /// Continues traversal to nested items within the AST.
    fn visit_item(&mut self, item: &'a Item) {
        // Visit all nested items
        visit::visit_item(self, item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_string_literal_valid_strings_extracts_content() {
        assert_eq!(
            AssetVisitor::extract_string_literal(r#""path/to/file.png""#),
            Some("path/to/file.png".to_string())
        );
        assert_eq!(
            AssetVisitor::extract_string_literal(r#"  "file.txt"  "#),
            Some("file.txt".to_string())
        );
    }

    #[test]
    fn test_extract_string_literal_invalid_formats_returns_none() {
        assert_eq!(AssetVisitor::extract_string_literal(""), None);
        assert_eq!(AssetVisitor::extract_string_literal(r#""""#), None);
        assert_eq!(AssetVisitor::extract_string_literal("   "), None);
    }
}
