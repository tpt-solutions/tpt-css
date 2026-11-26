// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `tpt-css`: a clean-room CSS engine.
//!
//! Pipeline (spec 1):
//!
//! ```text
//! CSS text -> tokens -> component values -> stylesheet AST
//!          -> selector matching -> cascade -> specified values
//!          -> computed styles
//! ```
//!
//! The core has no third-party dependencies and no DOM assumptions. It
//! styles any tree that implements [`tree::StyleTree`].

#![forbid(unsafe_code)]

pub mod ast;
pub mod cascade;
#[cfg(feature = "cssom")]
pub mod cssom;
pub mod env;
pub mod limits;
pub mod parse;
pub mod properties;
pub mod selectors;
pub mod style;
pub mod syntax;
pub mod tree;
pub mod values;

pub use cascade::origin::Origin;
pub use env::container::{ContainerInfo, ContainerSizeProvider};
pub use env::{Environment, MediaEnvironment, MediaKind, ViewportInfo};
pub use limits::ParseLimits;
pub use parse::{
    parse_declaration_list, parse_stylesheet, Diagnostic, DiagnosticCode, ParseOptions,
    ParsedStylesheet, Severity,
};
pub use style::computed::{ComputedStyle, StyleMap};
pub use style::custom::CustomPropertyValue;
pub use style::custom::RegisteredProperty;
pub use style::inspect::Inspection;
pub use style::invalidation::StyleChange;
pub use style::resolver::{
    ResolverConfig, SheetId, StyleResolver, StylesheetLimitExceeded, VisitedPolicy,
};
pub use tree::contract::StyleTree;
