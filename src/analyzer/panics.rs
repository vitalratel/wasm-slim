//! Panic pattern detection for WASM size optimization
//!
//! Identifies panic-inducing code patterns that bloat WASM binaries.
//! Each panic site adds 500-2000 bytes for formatting and unwinding infrastructure.
//!
//! Based on [Rust WASM book](https://rustwasm.github.io/docs/book/reference/code-size.html#avoid-panicking)

use crate::infra::{FileSystem, RealFileSystem};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use syn::{visit::Visit, BinOp, ExprBinary, ExprIndex, ExprMethodCall};
use thiserror::Error;

/// Errors that can occur during panic detection
#[derive(Error, Debug)]
pub enum PanicDetectionError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Regex compilation error
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Parse error
    #[error("Failed to parse {0}: {1}")]
    ParseError(PathBuf, String),
}

/// Type of panic pattern detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PanicPattern {
    /// .unwrap() call
    Unwrap,
    /// .expect("message") call
    Expect,
    /// Array indexing arr\[i\]
    Index,
    /// Division operator / or %
    Division,
    /// panic!() macro
    PanicMacro,
    /// assert!() macro (in release builds)
    AssertMacro,
}

impl PanicPattern {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            PanicPattern::Unwrap => "unwrap()",
            PanicPattern::Expect => "expect()",
            PanicPattern::Index => "array[index]",
            PanicPattern::Division => "division operator",
            PanicPattern::PanicMacro => "panic!()",
            PanicPattern::AssertMacro => "assert!()",
        }
    }

    /// Get recommended alternative
    pub fn alternative(&self) -> &'static str {
        match self {
            PanicPattern::Unwrap | PanicPattern::Expect => "match or if let",
            PanicPattern::Index => ".get(index)",
            PanicPattern::Division => ".checked_div() or .checked_rem()",
            PanicPattern::PanicMacro => "Result<T, E> or Option<T>",
            PanicPattern::AssertMacro => "debug_assert!() or runtime checks",
        }
    }

    /// Get estimated size per occurrence (bytes)
    pub fn size_per_occurrence(&self) -> u64 {
        match self {
            PanicPattern::Unwrap => 800,
            PanicPattern::Expect => 1200, // Higher due to custom message
            PanicPattern::Index => 1000,
            PanicPattern::Division => 600,
            PanicPattern::PanicMacro => 1500,
            PanicPattern::AssertMacro => 1000,
        }
    }
}

/// A detected panic site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPanic {
    /// File path
    pub file: PathBuf,
    /// Line number (0 if unknown)
    pub line: usize,
    /// Type of panic pattern
    pub pattern: PanicPattern,
    /// Code snippet (if available)
    pub snippet: Option<String>,
}

/// Complete panic analysis results
#[derive(Debug, Serialize, Deserialize)]
pub struct PanicResults {
    /// Total panics detected
    pub total_panics: usize,
    /// Panics by type
    pub by_pattern: Vec<(PanicPattern, usize)>,
    /// All detected panic sites
    pub panic_sites: Vec<DetectedPanic>,
    /// Estimated size impact in KB
    pub estimated_size_kb: u64,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Panic pattern detector
pub struct PanicDetector<FS: FileSystem + Sync + Send = RealFileSystem> {
    project_root: PathBuf,
    fs: FS,
}

impl PanicDetector<RealFileSystem> {
    /// Create a new panic detector with the real filesystem
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self::with_fs(project_root, RealFileSystem)
    }
}

impl<FS: FileSystem + Sync + Send> PanicDetector<FS> {
    /// Create a new panic detector with a custom filesystem implementation
    pub fn with_fs(project_root: impl Into<PathBuf>, fs: FS) -> Self {
        Self {
            project_root: project_root.into(),
            fs,
        }
    }

    /// Scan the project for panic patterns
    pub fn scan_project(&self) -> Result<PanicResults, PanicDetectionError> {
        // Find all Rust source files
        let rust_files = self.find_rust_files()?;

        // Parallel scan of all files
        let all_panics: Vec<DetectedPanic> = rust_files
            .par_iter()
            .flat_map(|source_file| {
                self.scan_file(source_file).unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to scan {}: {}", source_file.display(), e);
                    Vec::new()
                })
            })
            .collect();

        // Build results
        self.build_results(all_panics)
    }

    /// Scan a single source file
    fn scan_file(&self, source_file: &Path) -> Result<Vec<DetectedPanic>, PanicDetectionError> {
        let content = self.fs.read_to_string(source_file)?;

        let mut panics = Vec::new();

        // Try AST parsing first (most reliable)
        if let Ok(ast_panics) = self.scan_with_ast(&content, source_file) {
            panics.extend(ast_panics);
        }

        // Add regex fallback for patterns AST might miss
        let regex_panics = self.scan_with_regex(&content, source_file)?;
        panics.extend(regex_panics);

        Ok(panics)
    }

    /// Scan using AST parsing
    fn scan_with_ast(
        &self,
        content: &str,
        source_file: &Path,
    ) -> Result<Vec<DetectedPanic>, PanicDetectionError> {
        let syntax_tree: syn::File = syn::parse_str(content).map_err(|e| {
            PanicDetectionError::ParseError(source_file.to_path_buf(), e.to_string())
        })?;

        let mut visitor = PanicVisitor::new(source_file);
        visitor.visit_file(&syntax_tree);

        Ok(visitor.panics)
    }

    /// Scan using regex patterns (fallback)
    fn scan_with_regex(
        &self,
        content: &str,
        source_file: &Path,
    ) -> Result<Vec<DetectedPanic>, PanicDetectionError> {
        let mut panics = Vec::new();

        // Pattern: panic!
        let panic_re = Regex::new(r"panic!\s*\(")?;
        for (line_num, line) in content.lines().enumerate() {
            if panic_re.is_match(line) {
                panics.push(DetectedPanic {
                    file: source_file.to_path_buf(),
                    line: line_num + 1,
                    pattern: PanicPattern::PanicMacro,
                    snippet: Some(line.trim().to_string()),
                });
            }
        }

        // Pattern: assert!
        let assert_re = Regex::new(r"assert!\s*\(")?;
        for (line_num, line) in content.lines().enumerate() {
            if assert_re.is_match(line) && !line.contains("debug_assert!") {
                panics.push(DetectedPanic {
                    file: source_file.to_path_buf(),
                    line: line_num + 1,
                    pattern: PanicPattern::AssertMacro,
                    snippet: Some(line.trim().to_string()),
                });
            }
        }

        Ok(panics)
    }

    /// Find all Rust source files in the project
    fn find_rust_files(&self) -> Result<Vec<PathBuf>, PanicDetectionError> {
        let mut rust_files = Vec::new();

        // Search in src/ and tests/ directories
        for dir_name in &["src", "tests", "benches", "examples"] {
            let dir_path = self.project_root.join(dir_name);
            if dir_path.exists() {
                Self::collect_rust_files(&dir_path, &mut rust_files, &self.fs)?;
            }
        }

        Ok(rust_files)
    }

    /// Recursively collect .rs files
    fn collect_rust_files(
        dir: &Path,
        files: &mut Vec<PathBuf>,
        fs: &FS,
    ) -> Result<(), PanicDetectionError> {
        if dir.is_dir() {
            for entry in fs.read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    Self::collect_rust_files(&path, files, fs)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
        Ok(())
    }

    /// Build final results with statistics and recommendations
    fn build_results(
        &self,
        panic_sites: Vec<DetectedPanic>,
    ) -> Result<PanicResults, PanicDetectionError> {
        Ok(super::panic_advisor::build_results(panic_sites))
    }
}

/// AST visitor for detecting panic patterns
struct PanicVisitor<'a> {
    source_file: &'a Path,
    panics: Vec<DetectedPanic>,
}

impl<'a> PanicVisitor<'a> {
    fn new(source_file: &'a Path) -> Self {
        Self {
            source_file,
            panics: Vec::new(),
        }
    }

    fn add_panic(&mut self, pattern: PanicPattern, line: usize) {
        self.panics.push(DetectedPanic {
            file: self.source_file.to_path_buf(),
            line,
            pattern,
            snippet: None,
        });
    }
}

impl<'a> Visit<'a> for PanicVisitor<'a> {
    /// Visit method calls to detect unwrap(), expect()
    fn visit_expr_method_call(&mut self, node: &'a ExprMethodCall) {
        let method_name = node.method.to_string();

        match method_name.as_str() {
            "unwrap" => {
                self.add_panic(PanicPattern::Unwrap, 0);
            }
            "expect" => {
                self.add_panic(PanicPattern::Expect, 0);
            }
            _ => {}
        }

        // Continue visiting children
        syn::visit::visit_expr_method_call(self, node);
    }

    /// Visit binary operations to detect division
    fn visit_expr_binary(&mut self, node: &'a ExprBinary) {
        match node.op {
            BinOp::Div(_) | BinOp::Rem(_) => {
                self.add_panic(PanicPattern::Division, 0);
            }
            _ => {}
        }

        // Continue visiting children
        syn::visit::visit_expr_binary(self, node);
    }

    /// Visit index expressions to detect arr[i]
    fn visit_expr_index(&mut self, node: &'a ExprIndex) {
        self.add_panic(PanicPattern::Index, 0);

        // Continue visiting children
        syn::visit::visit_expr_index(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panic_pattern_name_returns_correct_names() {
        assert_eq!(PanicPattern::Unwrap.name(), "unwrap()");
        assert_eq!(PanicPattern::Expect.name(), "expect()");
        assert_eq!(PanicPattern::Index.name(), "array[index]");
        assert_eq!(PanicPattern::Division.name(), "division operator");
    }

    #[test]
    fn test_panic_pattern_alternative_returns_suggestions() {
        assert_eq!(PanicPattern::Unwrap.alternative(), "match or if let");
        assert_eq!(PanicPattern::Index.alternative(), ".get(index)");
        assert_eq!(
            PanicPattern::Division.alternative(),
            ".checked_div() or .checked_rem()"
        );
    }

    #[test]
    fn test_panic_pattern_size_per_occurrence_returns_nonzero() {
        assert!(PanicPattern::Unwrap.size_per_occurrence() > 0);
        assert!(
            PanicPattern::Expect.size_per_occurrence() > PanicPattern::Unwrap.size_per_occurrence()
        );
        assert!(PanicPattern::PanicMacro.size_per_occurrence() > 1000);
    }

    #[test]
    fn test_scan_with_ast_detects_unwrap() {
        let detector = PanicDetector::new(".");
        let code = r#"
            fn main() {
                let x = Some(5);
                x.unwrap();
            }
        "#;

        let result = detector.scan_with_ast(code, Path::new("test.rs"));
        assert!(result.is_ok());

        let panics = result.unwrap();
        assert_eq!(panics.len(), 1);
        assert_eq!(panics[0].pattern, PanicPattern::Unwrap);
    }

    #[test]
    fn test_scan_with_ast_detects_expect() {
        let detector = PanicDetector::new(".");
        let code = r#"
            fn main() {
                let x: Result<i32, String> = Ok(5);
                x.expect("failed");
            }
        "#;

        let result = detector.scan_with_ast(code, Path::new("test.rs"));
        assert!(result.is_ok());

        let panics = result.unwrap();
        assert_eq!(panics.len(), 1);
        assert_eq!(panics[0].pattern, PanicPattern::Expect);
    }

    #[test]
    fn test_scan_with_ast_detects_index() {
        let detector = PanicDetector::new(".");
        let code = r#"
            fn main() {
                let arr = [1, 2, 3];
                let x = arr[0];
            }
        "#;

        let result = detector.scan_with_ast(code, Path::new("test.rs"));
        assert!(result.is_ok());

        let panics = result.unwrap();
        assert_eq!(panics.len(), 1);
        assert_eq!(panics[0].pattern, PanicPattern::Index);
    }

    #[test]
    fn test_scan_with_ast_detects_division() {
        let detector = PanicDetector::new(".");
        let code = r#"
            fn main() {
                let x = 10 / 2;
                let y = 10 % 3;
            }
        "#;

        let result = detector.scan_with_ast(code, Path::new("test.rs"));
        assert!(result.is_ok());

        let panics = result.unwrap();
        assert_eq!(panics.len(), 2); // Both / and %
        assert!(panics.iter().all(|p| p.pattern == PanicPattern::Division));
    }

    #[test]
    fn test_scan_with_regex_detects_panic_macro() {
        let detector = PanicDetector::new(".");
        let code = r#"
            fn main() {
                panic!("error");
            }
        "#;

        let result = detector.scan_with_regex(code, Path::new("test.rs"));
        assert!(result.is_ok());

        let panics = result.unwrap();
        assert!(panics.iter().any(|p| p.pattern == PanicPattern::PanicMacro));
    }

    #[test]
    fn test_scan_with_regex_detects_assert_macro() {
        let detector = PanicDetector::new(".");
        let code = r#"
            fn main() {
                assert!(x > 0);
                debug_assert!(y > 0);  // Should NOT be detected
            }
        "#;

        let result = detector.scan_with_regex(code, Path::new("test.rs"));
        assert!(result.is_ok());

        let panics = result.unwrap();
        let assert_panics: Vec<_> = panics
            .iter()
            .filter(|p| p.pattern == PanicPattern::AssertMacro)
            .collect();
        assert_eq!(assert_panics.len(), 1); // Only assert!, not debug_assert!
    }

    #[test]
    fn test_generate_recommendations_critical_level() {
        use crate::analyzer::panic_advisor::generate_recommendations;
        let by_pattern = vec![(PanicPattern::Unwrap, 150)];

        let recs = generate_recommendations(150, &by_pattern, 120);

        assert!(!recs.is_empty());
        assert!(recs[0].contains("[P0]"));
        assert!(recs[0].contains("Critical"));
    }

    #[test]
    fn test_generate_recommendations_low_level() {
        use crate::analyzer::panic_advisor::generate_recommendations;
        let by_pattern = vec![(PanicPattern::Unwrap, 5)];

        let recs = generate_recommendations(5, &by_pattern, 4);

        assert!(!recs.is_empty());
        assert!(recs[0].contains("[P3]"));
        assert!(recs[0].contains("Low"));
    }
}
