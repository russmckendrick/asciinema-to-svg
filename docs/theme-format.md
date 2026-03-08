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

## `prompt`

- `font_family`
- `font_size`
- `row_padding_x`
- `segment_height`
- `text_color`
- `edge_fill`
- `separator_fill`
- `leading_symbol`
- `trailing_symbol`
- `palette`: one or more segment background colors

## Notes

- Built-in themes are regular JSON and can be used as a starting point for custom themes.
- Prompt remapping uses the selected theme’s `prompt` section when a compatible powerline/starship line is detected.
