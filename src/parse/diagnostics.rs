// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Diagnostics (spec 19.3). Diagnostics are optional for style resolution:
//! the engine recovers without them and they are only collected for tooling.

use crate::syntax::Span;

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Something was dropped or ignored because it is invalid.
    Error,
    /// Something was accepted but is suspicious.
    Warning,
    /// Informational.
    Info,
}

/// Stable diagnostic codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// A declaration was malformed and was dropped.
    InvalidDeclaration,
    /// A property name is not in the registry.
    UnknownProperty,
    /// A value did not match the property grammar and was dropped.
    InvalidValue,
    /// A selector was malformed and its rule was dropped.
    InvalidSelector,
    /// A valid selector or feature that this engine does not support yet.
    UnsupportedSelector,
    /// An at-rule that this engine does not support yet. It is preserved.
    UnsupportedAtRule,
    /// A feature that is valid CSS but not implemented yet.
    UnsupportedFeature,
    /// A configured limit (spec 19.2) was reached and input was cut off.
    LimitExceeded,
}

/// A diagnostic message with a source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Stable code.
    pub code: DiagnosticCode,
    /// Where the problem is.
    pub span: Span,
    /// Human-readable message.
    pub message: String,
    /// Link to the relevant public specification section, if any.
    pub spec_url: Option<&'static str>,
}

impl Diagnostic {
    /// Creates an error-severity diagnostic.
    pub fn error(
        code: DiagnosticCode,
        span: Span,
        message: impl Into<String>,
        spec_url: Option<&'static str>,
    ) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            code,
            span,
            message: message.into(),
            spec_url,
        }
    }
}
