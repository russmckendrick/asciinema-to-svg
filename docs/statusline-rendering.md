# Statusline Rendering

Statusline prompt rendering is enabled by default.

## Behavior

- A row is treated as a **statusline row** if it contains a powerline separator glyph (U+E0B0–U+E0BF) or has 3+ distinct non-default background-color segments.
- The **first** statusline row per frame is rendered in **bespoke mode**: the row's raw content is replaced with the theme's `prompt.segments` array (theme- or `--statusline`-supplied). Each segment becomes a colored `<rect>` with a right-pointing arrow `<polygon>` separator, palette colors cycling by index. Below it, on its own line, the renderer draws the theme's `trailing_symbol` followed by the typed command extracted from the row's gap area.
- **Subsequent** statusline rows in the same frame are rendered in **dynamic mode**: colored segments are extracted directly from the row's cell data and drawn with the same arrow separator style.
- The statusline height matches `line_height`, so it is the same height as regular text rows.

## Bespoke Segments

The statusline text is defined in the theme, not extracted from the cast. This makes rendering reliable regardless of the shell prompt configuration:

```json
"prompt": {
  "segments": ["user", "~"],
  "palette": ["#d96d0f", "#d7a126"]
}
```

Each entry in `segments` becomes one colored bar. Colors cycle through `palette`.

## Icon Segments

Segments can include icons from the bundled [Remix Icon](https://remixicon.com/) set (3,229 icons embedded at build time). Use an object with `icon` and optional `text` fields:

```json
"prompt": {
  "segments": [
    {"icon": "apple-fill", "text": "user"},
    {"icon": "folder-fill", "text": "~"}
  ],
  "palette": ["#d96d0f", "#d7a126"]
}
```

Icons are rendered as inline SVG elements (scaled to `line_height - 8px`) before the text label, with a 4px gap. If an icon name is not found, a warning is printed to stderr and the icon is skipped — the text still renders.

Icon-only segments (no `text` field) are also supported:

```json
{"icon": "git-branch-line"}
```

Plain string segments remain backward-compatible:

```json
"segments": ["user", "~"]
```

See the [Icon Reference](icons/README.md) for the full list of available icon names.

## Statusline Override

Use `--statusline <path>` to supply a standalone JSON config that overrides the theme's `prompt` section:

```bash
asciinema-to-svg demo.cast -o demo.svg --statusline custom-prompt.json
```

The JSON file uses the same shape as the `prompt` section in a theme file. See [Theme Format](theme-format.md) for field descriptions.

## Disable

Use `--no-statusline` to render the original prompt text without remapping.

## Detection Heuristic

- Detection is row-based: any row containing a statusline separator glyph is treated as a statusline row.
- Non-statusline rows are rendered as plain terminal text.
- Private Use Area glyphs in non-statusline rows are filtered out when statusline mode is enabled.
