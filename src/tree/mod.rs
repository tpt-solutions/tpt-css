// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The abstract tree contract (spec 9).

pub mod attributes;
pub mod contract;
pub mod node;
pub mod scope;
pub mod state;

pub use attributes::Attributes;
pub use contract::{Direction, StyleTree};
pub use node::{NamespaceId, NodeId, NodeKind, QualifiedName};
pub use scope::{ScopeInfo, ShadowInfo};
pub use state::{CustomStateSet, ElementState, StateBits};
