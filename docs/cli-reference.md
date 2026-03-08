# CLI Reference

## Usage

```bash
asciinema-to-svg <input.cast> [options]
```

## Options

- `-o, --output <path>`: output SVG path. Defaults to `output.svg`.
- `--theme <macos|linux|powershell|path>`: built-in theme name or custom theme JSON path. Defaults to `macos`.
- `--width <px>`: explicit SVG width in pixels.
- `--height <px>`: explicit SVG height in pixels.
- `--title <text>`: override the title bar text.
- `--no-powerline`: disable powerline/starship prompt remapping.

## Examples

```bash
asciinema-to-svg demo.cast --output demo.svg
asciinema-to-svg demo.cast --theme linux --output demo.svg
asciinema-to-svg demo.cast --theme ./themes/custom.json --width 1440 --title "Deploy" --output demo.svg
```
