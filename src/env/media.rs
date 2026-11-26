// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Media query evaluation (spec 16.3). Evaluation is separate from selector
//! matching: the resolver evaluates each `@media` condition once per resolve
//! call, then matches selectors only for the rules whose conditions hold.

use super::{Environment, MediaKind};
use crate::ast::{Comparison, MediaFeature, MediaQuery, MediaType};
use crate::values::LengthUnit;

/// True if a media query list matches: any one query must match.
pub fn list_matches(queries: &[MediaQuery], env: &Environment) -> bool {
    queries.iter().any(|q| query_matches(q, env))
}

/// True if one media query matches. `not` inverts the whole result.
pub fn query_matches(query: &MediaQuery, env: &Environment) -> bool {
    let type_ok = match &query.media_type {
        MediaType::All => true,
        MediaType::Screen => env.media.kind == MediaKind::Screen,
        MediaType::Print => env.media.kind == MediaKind::Print,
        MediaType::Unknown(_) => false,
    };
    let features_ok = query.features.iter().all(|f| feature_matches(f, env));
    let matched = type_ok && features_ok;
    if query.negated {
        !matched
    } else {
        matched
    }
}

/// Length in px for media features. Viewport units are not valid in media
/// queries, so they fall back to the raw value.
fn length_px(value: f64, unit: LengthUnit, env: &Environment) -> f64 {
    match unit {
        LengthUnit::Em | LengthUnit::Rem => value * env.root_font_size,
        _ => value,
    }
}

fn compare(actual: f64, expected: f64, cmp: Comparison) -> bool {
    match cmp {
        Comparison::Equal => (actual - expected).abs() < 1e-9,
        Comparison::Min => actual >= expected,
        Comparison::Max => actual <= expected,
    }
}

fn feature_matches(feature: &MediaFeature, env: &Environment) -> bool {
    let (w, h) = (env.viewport.width, env.viewport.height);
    match feature {
        MediaFeature::Width(cmp, value, unit) => compare(w, length_px(*value, *unit, env), *cmp),
        MediaFeature::Height(cmp, value, unit) => compare(h, length_px(*value, *unit, env), *cmp),
        MediaFeature::AspectRatio(cmp, ratio) => {
            // A zero-height viewport has no ratio, so no ratio query matches it.
            h > 0.0 && compare(w / h, *ratio, *cmp)
        }
        MediaFeature::Landscape(want) => (w > h) == *want,
        MediaFeature::PrefersDark(want) => env.media.prefers_dark == *want,
        MediaFeature::PrefersReducedMotion(want) => env.media.reduced_motion == *want,
        MediaFeature::Hover(want) => env.media.hover == *want,
        MediaFeature::Pointer(kind) => env.media.pointer == *kind,
        MediaFeature::PrefersContrast(kind) => env.media.contrast == *kind,
        MediaFeature::DynamicRange(high) => env.media.high_dynamic_range == *high,
        MediaFeature::Update(kind) => env.media.update == *kind,
        MediaFeature::Resolution(cmp, dppx) => compare(env.media.device_pixel_ratio, *dppx, *cmp),
        MediaFeature::Unknown(_) => false,
    }
}
