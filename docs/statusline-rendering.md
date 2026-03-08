# Statusline Rendering

Statusline prompt rendering is enabled by default.

## Behavior

- The renderer scans each row for statusline separator glyphs (U+E0B0–U+E0BF range).
- When a separator is found, the row is treated as a **statusline row** and is **skipped** — its raw content is not rendered.
- Instead, a **bespoke statusline** is drawn using the `prompt.segments` array from the theme (or `--statusline` override).
- Each segment is drawn as a colored `<rect>` with a right-pointing arrow `<polygon>` separator, using the theme's `prompt.palette` colors (cycling by index).
- Only the **first** statusline row per frame triggers the statusline; subsequent statusline rows are silently skipped.
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
