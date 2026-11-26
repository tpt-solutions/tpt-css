// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Container size provider (spec 16.5).
//!
//! Container queries need the size of a container, which layout computes. The
//! engine does not compute layout, so the host supplies the sizes through
//! [`ContainerSizeProvider`]. Only size containers should report a size.
//!
//! The host must compute the sizes before resolving styles, and call
//! [`crate::StyleResolver::resolve`] again when they change. Restyling with a
//! provider set always recomputes everything, because a size change is not
//! tracked by the invalidation system yet.

use crate::ast::MediaFeature;
use crate::tree::NodeId;
use crate::values::LengthUnit;

/// The size of a container and its optional name.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerInfo {
    /// Content-box width in px.
    pub width: f64,
    /// Content-box height in px.
    pub height: f64,
    /// The `container-name`, if set.
    pub name: Option<String>,
}

/// Supplies container sizes to the style engine (spec 16.5).
pub trait ContainerSizeProvider {
    /// The size of `node` if it is a size container, or `None` if it is not.
    fn container(&self, node: NodeId) -> Option<ContainerInfo>;
}

/// Evaluates size features against a container. Non-size features (for example
/// `orientation`) are false in a container query, and so are unknown features.
pub fn features_match(
    features: &[MediaFeature],
    info: &ContainerInfo,
    root_font_size: f64,
) -> bool {
    features.iter().all(|feature| match feature {
        MediaFeature::Width(cmp, value, unit) => {
            compare(info.width, length_px(*value, *unit, root_font_size), *cmp)
        }
        MediaFeature::Height(cmp, value, unit) => {
            compare(info.height, length_px(*value, *unit, root_font_size), *cmp)
        }
        _ => false,
    })
}

fn length_px(value: f64, unit: LengthUnit, root_font_size: f64) -> f64 {
    match unit {
        LengthUnit::Em | LengthUnit::Rem => value * root_font_size,
        _ => value,
    }
}

fn compare(actual: f64, expected: f64, cmp: crate::ast::Comparison) -> bool {
    use crate::ast::Comparison;
    match cmp {
        Comparison::Equal => (actual - expected).abs() < 1e-9,
        Comparison::Min => actual >= expected,
        Comparison::Max => actual <= expected,
    }
}
