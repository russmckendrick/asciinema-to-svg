# Theme Format

Theme files are JSON documents. The tool ships with built-in themes under [`themes/`](../themes).

## Top-Level Shape

```json
{
  "name": "custom",
  "font_family": "Cascadia Code, monospace",
  "font_size": 18.0,
  "line_height": 28.0,
  "terminal": { "...": "..." },
  "chrome": { "...": "..." },
  "prompt": { "...": "..." }
}
```

## `terminal`

- `background`: terminal background color
- `foreground`: default text color
- `selection`: reserved for selection styling
- `ansi_palette`: 16 ANSI colors in order

## `chrome`

- `kind`: `macos`, `linux`, or `powershell`
- `background`
- `border_color`
- `title_color`
- `subtitle_color`
- `radius`
- `padding`
- `title_bar_height`
- `content_top_gap` (optional, default `8.0`): vertical gap between the title bar and terminal content area

## `prompt`

Controls how powerline/statusline rows are rendered. Can also be supplied standalone via `--statusline`.

- `font_family`: font used for segment text
- `font_size`: text size inside segments
- `row_padding_x`: horizontal padding inside each segment before/after text
- `segment_height`: reserved height value (defaults to `28.0` to match `line_height`)
- `text_color`: color of text inside segments
- `edge_fill`: reserved for edge decoration color
- `separator_fill`: reserved for separator decoration color
- `leading_symbol`: reserved for future use
- `trailing_symbol`: reserved for future use
- `segments`: array of strings defining the bespoke statusline content (e.g. `["user", "~"]`). Each entry becomes one colored segment.
- `palette`: array of one or more segment background colors; segments cycle through these by index
- `segment_padding_x` (optional): override horizontal text padding within segments

## Notes

- Built-in themes are regular JSON and can be used as a starting point for custom themes.
- The `prompt` section is used whenever a powerline row is detected and `--no-powerline` is not set.
- The `segments` array defines what text appears in the bespoke statusline. The actual cast prompt content is skipped — only the theme-defined segments are rendered.
- A standalone `--statusline` JSON file uses the same shape as the `prompt` section and overrides it when provided.
