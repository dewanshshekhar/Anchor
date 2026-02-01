//! AST-based API endpoint detection for multiple languages.
//!
//! Each language module walks the AST directly to find API definitions
//! and consumptions. This approach is more reliable than regex-based
//! pattern matching.

pub mod api;
pub mod python;
pub mod javascript;
pub mod go;
pub mod java;
pub mod csharp;
pub mod ruby;
// pub mod kotlin;  // Disabled: tree-sitter version conflict
