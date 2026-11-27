# The `tpt-css` command line

The crate ships a small binary. It has no dependencies.

```text
tpt-css check <file.css | ->            diagnostics for a stylesheet
tpt-css compute <tree.json> [--all] [--css <file.css>]...   computed styles for a fixture tree
tpt-css inspect <tree.json>                                 the winning declaration of each property
```

## `check`

Parses a stylesheet and prints one line per diagnostic:

```text
error InvalidDeclaration at 2:6: declaration has no value
info UnknownProperty at 1:5: property `colr` is not implemented by this engine; the declaration is ignored
```

Exit status: `0` when there are no errors (infos and warnings are printed),
`1` when there is at least one error, `2` for a usage or read error. Use `-`
to read from standard input.

## `compute`

Resolves a JSON fixture and prints, for each element in document order, its
path (child indexes, `0.1` is the second child of the first child of the root)
and its name, `id`, and classes. Under each element it prints the properties
that an author style changed, with their computed values.

```json
{
  "stylesheets": ["p { color: red } .note { color: blue }"],
  "viewport": { "width": 800, "height": 600 },
  "root": {
    "element": "div",
    "id": "main",
    "children": [
      { "element": "p", "class": "note", "style": "margin-top: 3px",
        "children": [ { "text": "hi" } ] },
      { "element": "p" }
    ]
  }
}
```

- `stylesheets` are CSS strings, in source order, at author origin.
- `viewport` is optional. Its defaults are the engine's defaults.
- A node is an element (`element`, with optional `id`, `class` as a
  space-separated string, `style` as inline declarations, and `attributes` as a
  map of string values) or a text node (`text`). Elements may have `children`.

A property is printed when a stylesheet or inline style changed it. The tool
compares each element with the same tree resolved without any author style,
so `currentcolor` and the initial values do not appear on every element. Use
`--all` to print every property. `--css <file>` adds a stylesheet after the
fixture's own, at author origin. The option can repeat.

Pseudo-elements are not printed. Their styles are available through the
library (`StyleMap::get_pseudo`).

## `inspect`

Takes the same JSON fixture as `compute`. For each element it prints the
winning declaration of each property: its origin, its cascade layer, and its
specificity. Use it to find out which rule set a value. The playground's
Inspect button shows the same report.

## `inspect`

Resolves the same fixture as `compute`, and for each element prints the winning
declaration of each property it has. Each line has the value, then, in square
brackets, the origin (`author`, `user-agent`, and so on), `important` when the
declaration is `!important`, the layer rank, `inline` for a `style` attribute,
the specificity as `a,b,c`, and where it was declared (`sheet`, `rule`, and
`declaration` indexes, or `style attribute`).

```text
0.0 p.note
  color: rgb(0, 0, 255) [author; important; layer 0; specificity 1,1,1; sheet 0 rule 1 declaration 0]
```

The layer rank is the engine's cascade rank (`cascade::layer_rank`). Unlayered
normal declarations use `unlayered` in place of a rank. Unlike `compute`, this
lists every property that has a winning declaration at any origin, including the
user-agent styles that `compute` leaves out. A property with no declaration has no line.

Exit status: `0` on success, `2` for a usage error, a read error, a JSON error,
or a fixture without a `root`.
