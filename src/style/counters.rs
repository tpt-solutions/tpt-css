// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! CSS counters (CSS 2.1 section 12.4.1, CSS Lists 3 for the styles). The
//! style pass visits elements in document order, so counters are a stack that
//! is updated as the pass goes.
//!
//! A counter instance records the depth of the element that created it. Its
//! scope is that element and its following siblings and their descendants:
//! it ends when the pass reaches an element shallower than the creator. A
//! reset on an element replaces an instance of the same name created by a
//! previous sibling, so siblings do not nest.

/// One counter instance in scope.
#[derive(Debug, Clone, PartialEq)]
struct Instance {
    name: String,
    value: i64,
    /// Depth of the element that created the instance.
    level: usize,
}

/// The counter instances in scope, outermost first.
#[derive(Debug, Default, Clone)]
pub(crate) struct CounterScope {
    instances: Vec<Instance>,
}

impl CounterScope {
    /// Called before an element at `depth` is styled. Drops the instances of
    /// deeper elements, whose scope has ended.
    pub(crate) fn enter(&mut self, depth: usize) {
        self.instances.retain(|i| i.level <= depth);
    }

    /// `counter-reset`: a new instance on the element at `depth`, replacing a
    /// previous sibling's instance of the same name.
    pub(crate) fn reset(&mut self, name: &str, value: i64, depth: usize) {
        self.instances
            .retain(|i| !(i.name == name && i.level == depth));
        self.instances.push(Instance {
            name: name.to_string(),
            value,
            level: depth,
        });
    }

    /// `counter-set`: sets the innermost instance, or creates one on the
    /// element at `depth`.
    pub(crate) fn set(&mut self, name: &str, value: i64, depth: usize) {
        match self.innermost_mut(name) {
            Some(instance) => instance.value = value,
            None => self.reset(name, value, depth),
        }
    }

    /// `counter-increment`: adds `by` to the innermost instance, or creates one
    /// on the element at `depth` with the value 0 and then adds `by`.
    pub(crate) fn increment(&mut self, name: &str, by: i64, depth: usize) {
        match self.innermost_mut(name) {
            Some(instance) => instance.value = instance.value.wrapping_add(by),
            None => self.reset(name, by, depth),
        }
    }

    /// The innermost value of `name`, for `counter()`. A counter that is not in
    /// scope reads as 0 (CSS 2.1 instantiates it on the element that uses it,
    /// and that instance has the value 0).
    pub(crate) fn value(&self, name: &str) -> i64 {
        self.instances
            .iter()
            .rev()
            .find(|i| i.name == name)
            .map_or(0, |i| i.value)
    }

    /// Every value of `name` in scope, outermost first, for `counters()`.
    pub(crate) fn values(&self, name: &str) -> Vec<i64> {
        self.instances
            .iter()
            .filter(|i| i.name == name)
            .map(|i| i.value)
            .collect()
    }

    fn innermost_mut(&mut self, name: &str) -> Option<&mut Instance> {
        self.instances.iter_mut().rev().find(|i| i.name == name)
    }
}

/// Formats a counter value in a list style (CSS Counter Styles 3). Styles that
/// cannot represent the value fall back to decimal, as the spec requires.
/// Returns `None` for an unknown style.
pub(crate) fn format_counter(value: i64, style: &str) -> Option<String> {
    Some(match style {
        "decimal" => value.to_string(),
        "none" => String::new(),
        "lower-alpha" | "lower-latin" => {
            alphabetic(value, b'a').unwrap_or_else(|| value.to_string())
        }
        "upper-alpha" | "upper-latin" => {
            alphabetic(value, b'A').unwrap_or_else(|| value.to_string())
        }
        "lower-roman" => roman(value, false).unwrap_or_else(|| value.to_string()),
        "upper-roman" => roman(value, true).unwrap_or_else(|| value.to_string()),
        _ => return None,
    })
}

/// Bijective base-26: 1 is `a`, 26 is `z`, 27 is `aa`. Values below 1 have no
/// alphabetic form.
fn alphabetic(value: i64, base: u8) -> Option<String> {
    if value < 1 {
        return None;
    }
    let mut n = value;
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(base + (n % 26) as u8);
        n /= 26;
    }
    out.reverse();
    String::from_utf8(out).ok()
}

/// Roman numerals for 1 to 3999. Other values have no roman form.
fn roman(value: i64, upper: bool) -> Option<String> {
    if !(1..=3999).contains(&value) {
        return None;
    }
    const TABLE: [(i64, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut n = value;
    let mut out = String::new();
    for (unit, symbol) in TABLE {
        while n >= unit {
            out.push_str(symbol);
            n -= unit;
        }
    }
    Some(if upper { out.to_ascii_uppercase() } else { out })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_and_sibling_scopes() {
        let mut scope = CounterScope::default();
        // h1 at depth 0 resets; its children are at depth 1.
        scope.reset("sec", 0, 0);
        scope.increment("sec", 1, 1);
        assert_eq!(scope.value("sec"), 1);
        // A reset on a deeper element nests inside the outer instance.
        scope.reset("sec", 0, 1);
        assert_eq!(scope.values("sec"), vec![1, 0]);
        // A second reset at the same depth replaces the first, not nests.
        scope.reset("sec", 5, 1);
        assert_eq!(scope.values("sec"), vec![1, 5]);
        // Leaving the depth-1 subtree drops the instances created there.
        scope.enter(0);
        assert_eq!(scope.values("sec"), vec![1]);
    }

    #[test]
    fn increment_without_an_instance_creates_one() {
        let mut scope = CounterScope::default();
        scope.increment("n", 1, 2);
        assert_eq!(scope.value("n"), 1);
        scope.enter(1);
        assert_eq!(scope.value("n"), 0);
    }

    #[test]
    fn styles_format_values() {
        assert_eq!(format_counter(27, "lower-alpha").as_deref(), Some("aa"));
        assert_eq!(format_counter(3, "upper-alpha").as_deref(), Some("C"));
        assert_eq!(format_counter(0, "lower-alpha").as_deref(), Some("0"));
        assert_eq!(
            format_counter(1994, "upper-roman").as_deref(),
            Some("MCMXCIV")
        );
        assert_eq!(format_counter(0, "lower-roman").as_deref(), Some("0"));
        assert_eq!(format_counter(7, "none").as_deref(), Some(""));
        assert_eq!(format_counter(7, "circle"), None);
    }
}
