# CLI Reference

## Usage

```bash
asciinema-to-svg <input.cast> [options]
```

## Options

- `-o, --output <path>`: output SVG path. Defaults to `output.svg`.
- `--theme <macos|linux|powershell|path>`: built-in theme name or custom theme JSON path. Defaults to `macos`.
- `--size <small|medium|large>`: output size preset. Scales font size, line height, chrome dimensions, and statusline proportionally. Defaults to `medium`.
- `--size-config <path>`: path to a custom sizes JSON file. When provided, presets are loaded from this file instead of the built-in `config/sizes.json`.
- `--width <px>`: explicit SVG width in pixels.
- `--height <px>`: explicit SVG height in pixels.
- `--title <text>`: override the title bar text.
- `--no-statusline`: disable statusline prompt remapping.
- `--statusline <path>`: path to a standalone statusline config JSON that overrides the theme's `prompt` section. Uses the same shape as the `prompt` object in a theme file.

## Examples

```bash
asciinema-to-svg demo.cast --output demo.svg
asciinema-to-svg demo.cast --theme linux --output demo.svg
asciinema-to-svg demo.cast --theme ./themes/custom.json --width 1440 --title "Deploy" --output demo.svg
asciinema-to-svg demo.cast --statusline custom-prompt.json --output demo.svg
asciinema-to-svg demo.cast --size large --output demo.svg
asciinema-to-svg demo.cast --size small --output compact.svg
```
