# Powerline / Statusline Rendering

Powerline/starship prompt rendering is enabled by default.

## Behavior

- The renderer scans each row for powerline separator glyphs (`` U+E0B0, `` U+E0B2, `` U+E0B4).
- When a separator is found, the row is treated as a **statusline row**.
- Text between separators is extracted into segments.
- Each segment is drawn as a colored `<rect>` with an arrow `<polygon>` separator, using the theme's `prompt.palette` colors (cycling by index).
- Segment text is rendered centered inside each bar.
- The final arrow fades into the terminal background color.

## Statusline Override

Use `--statusline <path>` to supply a standalone JSON config that overrides the theme's `prompt` section:

```bash
asciinema-to-svg demo.cast -o demo.svg --statusline custom-prompt.json
```

The JSON file uses the same shape as the `prompt` section in a theme file. See [Theme Format](theme-format.md) for field descriptions.

## Disable

Use `--no-powerline` to render the original prompt text without remapping.

## Detection Heuristic

- Detection is row-based: any row containing a powerline separator glyph is rendered as a statusline.
- Non-prompt rows are rendered as plain terminal text.
- Statusline rows use `prompt.segment_height` for their height, which may differ from the normal `line_height`. Subsequent rows are offset accordingly.
