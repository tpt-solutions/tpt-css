// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Data-driven property registry (spec 15.1, 15.2).
//!
//! Each supported property is one row in [`PROPERTIES`]. Parsing is driven by
//! [`Grammar`]; initial values by [`Initial`]; inheritance by `inherited`.
//! Adding a property means adding a row, not changing the cascade.
//!
//! Deviations from the spec 15.1 sketch, recorded in DESIGN.md: rows have no
//! `id` field (the id is the row index, see [`ids`]), and there are no
//! `animatable` or `layout_effect` fields yet (M2 and M3).

use crate::values::{Color, Value};

/// Index of a property in [`PROPERTIES`]. Compact storage (spec 21.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PropertyId(pub u16);

impl PropertyId {
    /// The row index as `usize`.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// How a property's value is parsed (spec 15.1 `parse`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grammar {
    /// One of a fixed set of keywords.
    Keywords(&'static [&'static str]),
    /// A color, or `currentcolor`.
    Color,
    /// A length or percentage, optionally with keywords (for example `auto`).
    /// Unitless zero is accepted as a length.
    LengthPercentage {
        /// Reject negative values.
        non_negative: bool,
        /// Extra keywords accepted in place of a length.
        keywords: &'static [&'static str],
    },
    /// A border width: `thin`, `medium`, `thick`, or a length.
    BorderWidth,
    /// `line-height`: `normal`, a non-negative number, or a length or percentage.
    LineHeight,
    /// `font-weight`: `normal`, `bold`, `bolder`, `lighter`, or a number from 1 to 1000.
    FontWeight,
    /// A comma-separated font family list.
    FontFamily,
    /// `content`: `normal`, `none`, or one or more strings.
    Content,
    /// `opacity`: a number, clamped to `[0, 1]`. Percentages are accepted.
    Opacity,
    /// An integer, or one of the keywords (`z-index`).
    Integer {
        /// Extra keywords accepted in place of an integer.
        keywords: &'static [&'static str],
    },
}

/// The initial value (spec 15.1 `initial`). A small enum so the table is
/// const-constructible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Initial {
    /// A keyword.
    Keyword(&'static str),
    /// A px length.
    Px(f64),
    /// A unitless number.
    Number(f64),
    /// Opaque black.
    Black,
    /// Fully transparent.
    Transparent,
    /// `currentcolor`, resolved during computation.
    CurrentColor,
    /// The generic `serif` family.
    SerifFamily,
}

impl Initial {
    /// The initial value as a [`Value`].
    pub fn to_value(self) -> Value {
        match self {
            Initial::Keyword(k) => Value::Keyword(k),
            Initial::Px(v) => Value::px(v),
            Initial::Number(n) => Value::Number(n),
            Initial::Black => Value::Color(Color::BLACK),
            Initial::Transparent => Value::Color(Color::TRANSPARENT),
            Initial::CurrentColor => Value::Keyword("currentcolor"),
            Initial::SerifFamily => Value::FontFamily(vec!["serif".to_string()]),
        }
    }
}

/// Metadata for one property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertySpec {
    /// Property name, lowercase ASCII.
    pub name: &'static str,
    /// Whether the property inherits by default (CSS Cascade, inheritance).
    pub inherited: bool,
    /// Initial value.
    pub initial: Initial,
    /// Value grammar.
    pub grammar: Grammar,
    /// Link to the defining specification.
    pub spec_url: &'static str,
}

const CSS_BOX: &str = "https://www.w3.org/TR/css-box-4/";
const CSS_TEXT: &str = "https://www.w3.org/TR/css-text-3/";
const CSS_FONTS: &str = "https://www.w3.org/TR/css-fonts-4/";
const CSS_DISPLAY: &str = "https://www.w3.org/TR/css-display-3/";
const CSS_BACKGROUNDS: &str = "https://www.w3.org/TR/css-backgrounds-3/";
const CSS_COLOR: &str = "https://www.w3.org/TR/css-color-4/";
const CSS_SIZING: &str = "https://www.w3.org/TR/css-sizing-3/";

const DISPLAY_KEYWORDS: &[&str] = &[
    "block",
    "inline",
    "inline-block",
    "list-item",
    "none",
    "contents",
    "flow-root",
    "flex",
    "inline-flex",
    "grid",
    "inline-grid",
    "table",
    "table-row",
    "table-cell",
];
const VISIBILITY_KEYWORDS: &[&str] = &["visible", "hidden", "collapse"];
const FONT_STYLE_KEYWORDS: &[&str] = &["normal", "italic", "oblique"];
const TEXT_ALIGN_KEYWORDS: &[&str] = &["start", "end", "left", "right", "center", "justify"];
const BORDER_STYLE_KEYWORDS: &[&str] = &[
    "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];
const BOX_SIZING_KEYWORDS: &[&str] = &["content-box", "border-box"];
const OVERFLOW_KEYWORDS: &[&str] = &["visible", "hidden", "clip", "scroll", "auto"];
const POSITION_KEYWORDS: &[&str] = &["static", "relative", "absolute", "fixed", "sticky"];
const VERTICAL_ALIGN_KEYWORDS: &[&str] = &[
    "baseline",
    "sub",
    "super",
    "text-top",
    "text-bottom",
    "middle",
    "top",
    "bottom",
];
const WHITE_SPACE_KEYWORDS: &[&str] = &[
    "normal",
    "nowrap",
    "pre",
    "pre-wrap",
    "pre-line",
    "break-spaces",
];
const DIRECTION_KEYWORDS: &[&str] = &["ltr", "rtl"];
const UNICODE_BIDI_KEYWORDS: &[&str] = &[
    "normal",
    "embed",
    "isolate",
    "bidi-override",
    "isolate-override",
    "plaintext",
];
const CSS_POSITION: &str = "https://www.w3.org/TR/css-position-3/";
const CSS_OVERFLOW: &str = "https://www.w3.org/TR/css-overflow-3/";
const CSS_CONTENT: &str = "https://www.w3.org/TR/css-content-3/";
const CSS_INLINE: &str = "https://www.w3.org/TR/css-inline-3/";
const CSS_WRITING: &str = "https://www.w3.org/TR/css-writing-modes-4/";
const AUTO: &[&str] = &["auto"];
const NONE: &[&str] = &["none"];
const NO_KEYWORDS: &[&str] = &[];

/// Every property supported in M0 (spec 15.2, 37 properties).
///
/// The order is significant: [`ids`] holds the row indices, and a unit test
/// checks that they agree.
pub const PROPERTIES: &[PropertySpec] = &[
    // 0
    PropertySpec {
        name: "display",
        inherited: false,
        initial: Initial::Keyword("inline"),
        grammar: Grammar::Keywords(DISPLAY_KEYWORDS),
        spec_url: CSS_DISPLAY,
    },
    // 1
    PropertySpec {
        name: "visibility",
        inherited: true,
        initial: Initial::Keyword("visible"),
        grammar: Grammar::Keywords(VISIBILITY_KEYWORDS),
        spec_url: CSS_BOX,
    },
    // 2
    PropertySpec {
        name: "color",
        inherited: true,
        initial: Initial::Black,
        grammar: Grammar::Color,
        spec_url: CSS_COLOR,
    },
    // 3
    PropertySpec {
        name: "background-color",
        inherited: false,
        initial: Initial::Transparent,
        grammar: Grammar::Color,
        spec_url: CSS_BACKGROUNDS,
    },
    // 4
    PropertySpec {
        name: "font-family",
        inherited: true,
        initial: Initial::SerifFamily,
        grammar: Grammar::FontFamily,
        spec_url: CSS_FONTS,
    },
    // 5
    PropertySpec {
        name: "font-size",
        inherited: true,
        initial: Initial::Px(16.0),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: NO_KEYWORDS,
        },
        spec_url: CSS_FONTS,
    },
    // 6
    PropertySpec {
        name: "font-style",
        inherited: true,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Keywords(FONT_STYLE_KEYWORDS),
        spec_url: CSS_FONTS,
    },
    // 7
    PropertySpec {
        name: "font-weight",
        inherited: true,
        initial: Initial::Number(400.0),
        grammar: Grammar::FontWeight,
        spec_url: CSS_FONTS,
    },
    // 8
    PropertySpec {
        name: "line-height",
        inherited: true,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::LineHeight,
        spec_url: CSS_TEXT,
    },
    // 9
    PropertySpec {
        name: "text-align",
        inherited: true,
        initial: Initial::Keyword("start"),
        grammar: Grammar::Keywords(TEXT_ALIGN_KEYWORDS),
        spec_url: CSS_TEXT,
    },
    // 10
    PropertySpec {
        name: "width",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: AUTO,
        },
        spec_url: CSS_SIZING,
    },
    // 11
    PropertySpec {
        name: "height",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: AUTO,
        },
        spec_url: CSS_SIZING,
    },
    // 12
    PropertySpec {
        name: "min-width",
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: NO_KEYWORDS,
        },
        spec_url: CSS_SIZING,
    },
    // 13
    PropertySpec {
        name: "min-height",
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: NO_KEYWORDS,
        },
        spec_url: CSS_SIZING,
    },
    // 14
    PropertySpec {
        name: "max-width",
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: NONE,
        },
        spec_url: CSS_SIZING,
    },
    // 15
    PropertySpec {
        name: "max-height",
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: NONE,
        },
        spec_url: CSS_SIZING,
    },
    // 16-19
    margin("margin-top"),
    margin("margin-right"),
    margin("margin-bottom"),
    margin("margin-left"),
    // 20-23
    padding("padding-top"),
    padding("padding-right"),
    padding("padding-bottom"),
    padding("padding-left"),
    // 24-27
    border_width("border-top-width"),
    border_width("border-right-width"),
    border_width("border-bottom-width"),
    border_width("border-left-width"),
    // 28-31
    border_color("border-top-color"),
    border_color("border-right-color"),
    border_color("border-bottom-color"),
    border_color("border-left-color"),
    // 32-35
    border_style("border-top-style"),
    border_style("border-right-style"),
    border_style("border-bottom-style"),
    border_style("border-left-style"),
    // 36
    PropertySpec {
        name: "box-sizing",
        inherited: false,
        initial: Initial::Keyword("content-box"),
        grammar: Grammar::Keywords(BOX_SIZING_KEYWORDS),
        spec_url: CSS_SIZING,
    },
    // 37
    PropertySpec {
        name: "overflow",
        inherited: false,
        initial: Initial::Keyword("visible"),
        grammar: Grammar::Keywords(OVERFLOW_KEYWORDS),
        spec_url: CSS_OVERFLOW,
    },
    // 38
    PropertySpec {
        name: "position",
        inherited: false,
        initial: Initial::Keyword("static"),
        grammar: Grammar::Keywords(POSITION_KEYWORDS),
        spec_url: CSS_POSITION,
    },
    // 39-42
    offset("top"),
    offset("right"),
    offset("bottom"),
    offset("left"),
    // 43
    PropertySpec {
        name: "z-index",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Integer { keywords: AUTO },
        spec_url: CSS_POSITION,
    },
    // 44
    PropertySpec {
        name: "opacity",
        inherited: false,
        initial: Initial::Number(1.0),
        grammar: Grammar::Opacity,
        spec_url: CSS_COLOR,
    },
    // 45
    PropertySpec {
        name: "content",
        inherited: false,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Content,
        spec_url: CSS_CONTENT,
    },
    // 46
    PropertySpec {
        name: "vertical-align",
        inherited: false,
        initial: Initial::Keyword("baseline"),
        grammar: Grammar::LengthPercentage {
            non_negative: false,
            keywords: VERTICAL_ALIGN_KEYWORDS,
        },
        spec_url: CSS_INLINE,
    },
    // 47
    PropertySpec {
        name: "white-space",
        inherited: true,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Keywords(WHITE_SPACE_KEYWORDS),
        spec_url: CSS_TEXT,
    },
    // 48
    PropertySpec {
        name: "direction",
        inherited: true,
        initial: Initial::Keyword("ltr"),
        grammar: Grammar::Keywords(DIRECTION_KEYWORDS),
        spec_url: CSS_WRITING,
    },
    // 49
    PropertySpec {
        name: "unicode-bidi",
        inherited: false,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Keywords(UNICODE_BIDI_KEYWORDS),
        spec_url: CSS_WRITING,
    },
];

const fn offset(name: &'static str) -> PropertySpec {
    PropertySpec {
        name,
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::LengthPercentage {
            non_negative: false,
            keywords: AUTO,
        },
        spec_url: CSS_POSITION,
    }
}

const fn margin(name: &'static str) -> PropertySpec {
    PropertySpec {
        name,
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::LengthPercentage {
            non_negative: false,
            keywords: AUTO,
        },
        spec_url: CSS_BOX,
    }
}

const fn padding(name: &'static str) -> PropertySpec {
    PropertySpec {
        name,
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: NO_KEYWORDS,
        },
        spec_url: CSS_BOX,
    }
}

const fn border_width(name: &'static str) -> PropertySpec {
    PropertySpec {
        name,
        inherited: false,
        initial: Initial::Px(3.0),
        grammar: Grammar::BorderWidth,
        spec_url: CSS_BACKGROUNDS,
    }
}

const fn border_color(name: &'static str) -> PropertySpec {
    PropertySpec {
        name,
        inherited: false,
        initial: Initial::CurrentColor,
        grammar: Grammar::Color,
        spec_url: CSS_BACKGROUNDS,
    }
}

const fn border_style(name: &'static str) -> PropertySpec {
    PropertySpec {
        name,
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::Keywords(BORDER_STYLE_KEYWORDS),
        spec_url: CSS_BACKGROUNDS,
    }
}

/// Named ids for properties that the engine itself refers to.
///
/// Each constant equals the row index of the property in [`PROPERTIES`].
/// The unit test `ids_match_table` checks this.
pub mod ids {
    use super::PropertyId;

    /// `display`
    pub const DISPLAY: PropertyId = PropertyId(0);
    /// `visibility`
    pub const VISIBILITY: PropertyId = PropertyId(1);
    /// `color`
    pub const COLOR: PropertyId = PropertyId(2);
    /// `font-family`
    pub const FONT_FAMILY: PropertyId = PropertyId(4);
    /// `font-size`
    pub const FONT_SIZE: PropertyId = PropertyId(5);
    /// `font-weight`
    pub const FONT_WEIGHT: PropertyId = PropertyId(7);
    /// `content`, which decides whether `::before` and `::after` are generated.
    pub const CONTENT: PropertyId = PropertyId(45);
    /// `border-top-width`, first of the four border widths
    pub const BORDER_TOP_WIDTH: PropertyId = PropertyId(24);
    /// `border-top-style`, first of the four border styles
    pub const BORDER_TOP_STYLE: PropertyId = PropertyId(32);
}

/// Looks up a property by its (already lowercased) name.
pub fn lookup(name: &str) -> Option<PropertyId> {
    PROPERTIES
        .iter()
        .position(|p| p.name == name)
        .map(|i| PropertyId(i as u16))
}

/// The metadata row for an id.
pub fn spec(id: PropertyId) -> &'static PropertySpec {
    &PROPERTIES[id.index()]
}
