//! Relational Kernel Language types, bounds, canonical form, and debug parser.
//!
//! RKL is a low-level bounded language. It deliberately contains no task-family
//! transformation tags. This crate validates syntax and totality obligations;
//! execution belongs to WP05.

mod ast;
mod canonical;
mod checker;
mod cost;
mod error;
mod execution;
mod parser;
mod theory_execution;

pub use ast::*;
pub use canonical::{canonical_runtime_value, CanonicalForm, Canonicalizer, DefaultCanonicalizer};
pub use checker::{BoundChecker, DefinitionSignature, TheoryChecker, TypeChecker, TypeEnvironment};
pub use cost::{Cost, CostError, CostModel, StructuralCostModel};
pub use error::{CheckError, CheckErrorKind, ExprPath, PathSegment};
pub use execution::*;
pub use parser::{parse_debug_expr, ParseError};
pub use theory_execution::*;

/// Executable semantics established by this first RKL contract.
pub const RKL_SEMANTICS_VERSION: &str = "1.1.0";
