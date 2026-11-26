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
    /// `text-decoration-line`: `none`, or one or more of `underline`, `overline`,
    /// `line-through`, and `blink`, in any order, each at most once.
    TextDecorationLine,
    /// A length without percentages, with extra keywords (`letter-spacing`).
    Length {
        /// Extra keywords accepted in place of a length.
        keywords: &'static [&'static str],
    },
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
    /// A corner radius: one or two non-negative lengths or percentages
    /// (horizontal, then vertical). Two equal values are stored as one.
    BorderRadius,
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
const CSS_UI: &str = "https://www.w3.org/TR/css-ui-4/";
const CSS_IMAGES: &str = "https://www.w3.org/TR/css-images-3/";
const CSS_COMPOSITING: &str = "https://www.w3.org/TR/compositing-1/";
const CSS_TRANSFORMS: &str = "https://www.w3.org/TR/css-transforms-2/";
const CSS_CSSOM_VIEW: &str = "https://www.w3.org/TR/cssom-view-1/";
const CSS_OVERSCROLL: &str = "https://www.w3.org/TR/css-overscroll-1/";
const CSS_LISTS: &str = "https://www.w3.org/TR/css-lists-3/";
const CSS_TEXT_DECOR: &str = "https://www.w3.org/TR/css-text-decor-3/";
const AUTO: &[&str] = &["auto"];
const NONE: &[&str] = &["none"];
const NO_KEYWORDS: &[&str] = &[];

const CURSOR_KEYWORDS: &[&str] = &[
    "auto",
    "default",
    "none",
    "context-menu",
    "help",
    "pointer",
    "progress",
    "wait",
    "cell",
    "crosshair",
    "text",
    "vertical-text",
    "alias",
    "copy",
    "move",
    "no-drop",
    "not-allowed",
    "grab",
    "grabbing",
    "e-resize",
    "n-resize",
    "ne-resize",
    "nw-resize",
    "s-resize",
    "se-resize",
    "sw-resize",
    "w-resize",
    "ew-resize",
    "ns-resize",
    "nesw-resize",
    "nwse-resize",
    "col-resize",
    "row-resize",
    "all-scroll",
    "zoom-in",
    "zoom-out",
];
const POINTER_EVENTS_KEYWORDS: &[&str] = &[
    "auto",
    "none",
    "visiblepainted",
    "visiblefill",
    "visiblestroke",
    "visible",
    "painted",
    "fill",
    "stroke",
    "all",
];
const USER_SELECT_KEYWORDS: &[&str] = &["auto", "none", "text", "all", "contain"];
const APPEARANCE_KEYWORDS: &[&str] = &["none", "auto"];
const RESIZE_KEYWORDS: &[&str] = &["none", "both", "horizontal", "vertical", "block", "inline"];
const OUTLINE_STYLE_KEYWORDS: &[&str] = &[
    "auto", "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset",
    "outset",
];
const TEXT_TRANSFORM_KEYWORDS: &[&str] = &[
    "none",
    "capitalize",
    "uppercase",
    "lowercase",
    "full-width",
    "full-size-kana",
];
const OVERFLOW_WRAP_KEYWORDS: &[&str] = &["normal", "break-word", "anywhere"];
const WORD_BREAK_KEYWORDS: &[&str] = &["normal", "break-all", "keep-all"];
const HYPHENS_KEYWORDS: &[&str] = &["none", "manual", "auto"];
const TEXT_RENDERING_KEYWORDS: &[&str] = &[
    "auto",
    "optimizespeed",
    "optimizelegibility",
    "geometricprecision",
];
const TEXT_OVERFLOW_KEYWORDS: &[&str] = &["clip", "ellipsis"];
const FLOAT_KEYWORDS: &[&str] = &["none", "left", "right", "inline-start", "inline-end"];
const CLEAR_KEYWORDS: &[&str] = &[
    "none",
    "left",
    "right",
    "both",
    "inline-start",
    "inline-end",
];
const OBJECT_FIT_KEYWORDS: &[&str] = &["fill", "contain", "cover", "none", "scale-down"];
const IMAGE_RENDERING_KEYWORDS: &[&str] =
    &["auto", "smooth", "high-quality", "crisp-edges", "pixelated"];
const BLEND_MODE_KEYWORDS: &[&str] = &[
    "normal",
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion",
    "hue",
    "saturation",
    "color",
    "luminosity",
];
const ISOLATION_KEYWORDS: &[&str] = &["auto", "isolate"];
const BACKFACE_KEYWORDS: &[&str] = &["visible", "hidden"];
const SCROLL_BEHAVIOR_KEYWORDS: &[&str] = &["auto", "smooth"];
const OVERSCROLL_KEYWORDS: &[&str] = &["auto", "contain", "none"];

const LIST_STYLE_TYPE_KEYWORDS: &[&str] = &[
    "none",
    "disc",
    "circle",
    "square",
    "decimal",
    "decimal-leading-zero",
    "lower-roman",
    "upper-roman",
    "lower-alpha",
    "upper-alpha",
    "lower-latin",
    "upper-latin",
    "lower-greek",
];
const LIST_STYLE_POSITION_KEYWORDS: &[&str] = &["inside", "outside"];

const TEXT_DECORATION_STYLE_KEYWORDS: &[&str] = &["solid", "double", "dotted", "dashed", "wavy"];

/// The `text-decoration-line` values, indexed by the bits `underline` (1),
/// `overline` (2), `line-through` (4), and `blink` (8). Each combination is
/// spelled in the order the specification serializes it.
pub(crate) const TEXT_DECORATION_LINE_VALUES: [&str; 16] = [
    "none",
    "underline",
    "overline",
    "underline overline",
    "line-through",
    "underline line-through",
    "overline line-through",
    "underline overline line-through",
    "blink",
    "underline blink",
    "overline blink",
    "underline overline blink",
    "line-through blink",
    "underline line-through blink",
    "overline line-through blink",
    "underline overline line-through blink",
];

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
    // 50
    PropertySpec {
        name: "cursor",
        inherited: true,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(CURSOR_KEYWORDS),
        spec_url: CSS_UI,
    },
    // 51
    PropertySpec {
        name: "pointer-events",
        inherited: true,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(POINTER_EVENTS_KEYWORDS),
        spec_url: CSS_UI,
    },
    // 52
    PropertySpec {
        name: "user-select",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(USER_SELECT_KEYWORDS),
        spec_url: CSS_UI,
    },
    // 53
    PropertySpec {
        name: "appearance",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(APPEARANCE_KEYWORDS),
        spec_url: CSS_UI,
    },
    // 54
    PropertySpec {
        name: "resize",
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::Keywords(RESIZE_KEYWORDS),
        spec_url: CSS_UI,
    },
    // 55
    PropertySpec {
        name: "outline-style",
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::Keywords(OUTLINE_STYLE_KEYWORDS),
        spec_url: CSS_UI,
    },
    // 56
    PropertySpec {
        name: "text-transform",
        inherited: true,
        initial: Initial::Keyword("none"),
        grammar: Grammar::Keywords(TEXT_TRANSFORM_KEYWORDS),
        spec_url: CSS_TEXT,
    },
    // 57
    PropertySpec {
        name: "text-indent",
        inherited: true,
        initial: Initial::Px(0.0),
        grammar: Grammar::LengthPercentage {
            non_negative: false,
            keywords: NO_KEYWORDS,
        },
        spec_url: CSS_TEXT,
    },
    // 58
    PropertySpec {
        name: "overflow-wrap",
        inherited: true,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Keywords(OVERFLOW_WRAP_KEYWORDS),
        spec_url: CSS_TEXT,
    },
    // 59
    PropertySpec {
        name: "word-break",
        inherited: true,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Keywords(WORD_BREAK_KEYWORDS),
        spec_url: CSS_TEXT,
    },
    // 60
    PropertySpec {
        name: "hyphens",
        inherited: true,
        initial: Initial::Keyword("manual"),
        grammar: Grammar::Keywords(HYPHENS_KEYWORDS),
        spec_url: CSS_TEXT,
    },
    // 61
    PropertySpec {
        name: "text-rendering",
        inherited: true,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(TEXT_RENDERING_KEYWORDS),
        spec_url: CSS_TEXT,
    },
    // 62
    PropertySpec {
        name: "text-overflow",
        inherited: false,
        initial: Initial::Keyword("clip"),
        grammar: Grammar::Keywords(TEXT_OVERFLOW_KEYWORDS),
        spec_url: CSS_OVERFLOW,
    },
    // 63
    PropertySpec {
        name: "float",
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::Keywords(FLOAT_KEYWORDS),
        spec_url: CSS_DISPLAY,
    },
    // 64
    PropertySpec {
        name: "clear",
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::Keywords(CLEAR_KEYWORDS),
        spec_url: CSS_DISPLAY,
    },
    // 65
    PropertySpec {
        name: "object-fit",
        inherited: false,
        initial: Initial::Keyword("fill"),
        grammar: Grammar::Keywords(OBJECT_FIT_KEYWORDS),
        spec_url: CSS_IMAGES,
    },
    // 66
    PropertySpec {
        name: "image-rendering",
        inherited: true,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(IMAGE_RENDERING_KEYWORDS),
        spec_url: CSS_IMAGES,
    },
    // 67
    PropertySpec {
        name: "mix-blend-mode",
        inherited: false,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Keywords(BLEND_MODE_KEYWORDS),
        spec_url: CSS_COMPOSITING,
    },
    // 68
    PropertySpec {
        name: "isolation",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(ISOLATION_KEYWORDS),
        spec_url: CSS_COMPOSITING,
    },
    // 69
    PropertySpec {
        name: "backface-visibility",
        inherited: false,
        initial: Initial::Keyword("visible"),
        grammar: Grammar::Keywords(BACKFACE_KEYWORDS),
        spec_url: CSS_TRANSFORMS,
    },
    // 70
    PropertySpec {
        name: "scroll-behavior",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(SCROLL_BEHAVIOR_KEYWORDS),
        spec_url: CSS_CSSOM_VIEW,
    },
    // 71
    PropertySpec {
        name: "overscroll-behavior",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(OVERSCROLL_KEYWORDS),
        spec_url: CSS_OVERSCROLL,
    },
    // 72
    PropertySpec {
        name: "list-style-type",
        inherited: true,
        initial: Initial::Keyword("disc"),
        grammar: Grammar::Keywords(LIST_STYLE_TYPE_KEYWORDS),
        spec_url: CSS_LISTS,
    },
    // 73
    PropertySpec {
        name: "list-style-position",
        inherited: true,
        initial: Initial::Keyword("outside"),
        grammar: Grammar::Keywords(LIST_STYLE_POSITION_KEYWORDS),
        spec_url: CSS_LISTS,
    },
    // 74
    PropertySpec {
        name: "list-style-image",
        inherited: true,
        initial: Initial::Keyword("none"),
        grammar: Grammar::Keywords(NONE),
        spec_url: CSS_LISTS,
    },
    // 75
    PropertySpec {
        name: "text-decoration-line",
        inherited: false,
        initial: Initial::Keyword("none"),
        grammar: Grammar::TextDecorationLine,
        spec_url: CSS_TEXT_DECOR,
    },
    // 76
    PropertySpec {
        name: "text-decoration-style",
        inherited: false,
        initial: Initial::Keyword("solid"),
        grammar: Grammar::Keywords(TEXT_DECORATION_STYLE_KEYWORDS),
        spec_url: CSS_TEXT_DECOR,
    },
    // 77
    PropertySpec {
        name: "text-decoration-color",
        inherited: false,
        initial: Initial::CurrentColor,
        grammar: Grammar::Color,
        spec_url: CSS_TEXT_DECOR,
    },
    // 78
    PropertySpec {
        name: "letter-spacing",
        inherited: true,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Length {
            keywords: &["normal"],
        },
        spec_url: CSS_TEXT,
    },
    // 79
    PropertySpec {
        name: "word-spacing",
        inherited: true,
        initial: Initial::Keyword("normal"),
        grammar: Grammar::Length {
            keywords: &["normal"],
        },
        spec_url: CSS_TEXT,
    },
    // 80
    PropertySpec {
        name: "border-top-left-radius",
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::BorderRadius,
        spec_url: CSS_BACKGROUNDS,
    },
    // 81
    PropertySpec {
        name: "border-top-right-radius",
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::BorderRadius,
        spec_url: CSS_BACKGROUNDS,
    },
    // 82
    PropertySpec {
        name: "border-bottom-right-radius",
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::BorderRadius,
        spec_url: CSS_BACKGROUNDS,
    },
    // 83
    PropertySpec {
        name: "border-bottom-left-radius",
        inherited: false,
        initial: Initial::Px(0.0),
        grammar: Grammar::BorderRadius,
        spec_url: CSS_BACKGROUNDS,
    },
    // 84
    PropertySpec {
        name: "text-decoration-thickness",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::LengthPercentage {
            non_negative: true,
            keywords: &["auto", "from-font"],
        },
        spec_url: CSS_TEXT_DECOR,
    },
    // 85
    PropertySpec {
        name: "text-decoration-skip-ink",
        inherited: false,
        initial: Initial::Keyword("auto"),
        grammar: Grammar::Keywords(&["auto", "all", "none"]),
        spec_url: CSS_TEXT_DECOR,
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
    let name = super::aliases::canonical_name(name);
    PROPERTIES
        .iter()
        .position(|p| p.name == name)
        .map(|i| PropertyId(i as u16))
}

/// The metadata row for an id.
pub fn spec(id: PropertyId) -> &'static PropertySpec {
    &PROPERTIES[id.index()]
}
