# CLI Reference

## Usage

```bash
asciinema-to-svg <input.cast> [options]
```

## Options

### Output

- `-o, --output <path>`: output SVG path. Defaults to `output.svg`.
- `--theme <macos|linux|powershell|path>`: built-in theme name or custom theme JSON path. Defaults to `macos`.

### Sizing

- `--size <small|medium|large>`: output size preset. Scales font size, line height, chrome dimensions, and statusline proportionally. Defaults to `medium`.
- `--size-config <path>`: path to a custom sizes JSON file. When provided, presets are loaded from this file instead of the built-in `config/sizes.json`.
- `--width <px>`: explicit SVG width in pixels.
- `--height <px>`: explicit SVG height in pixels.

### Title

- `--title <text>`: override the title bar text. Highest priority. If omitted, the renderer falls back to (in order): the OSC 0/2 title set by the cast, then the input file stem, then `Terminal`.

### Statusline

- `--no-statusline`: disable statusline prompt remapping.
- `--statusline <path>`: path to a standalone statusline config JSON that overrides the theme's `prompt` section. Uses the same shape as the `prompt` object in a theme file.

### Timing

- `--speed <factor>`: playback speed multiplier. Defaults to `1.0` (real time). `2.0` halves the animation duration.
- `--idle-time-limit <secs>`: cap any inter-frame pause at this many seconds. Useful for compressing long quiet stretches in a recording without affecting the active sections.
- `--start <secs>` / `--end <secs>`: trim the cast to the `[start, end]` window. Surviving frames are rebased so the animation begins at 0.
- `--at <secs>`: render a single static SVG of the buffer at this time. Implies no animation: the output contains no `@keyframes` and no `.frame` groups.
- `--no-loop`: play the animation once instead of looping forever (sets `animation-iteration-count: 1`).

### Other

- `--verbose`: print warnings about unhandled control sequences to stderr. Currently a placeholder (logging hooks land in a follow-up).

## Examples

```bash
asciinema-to-svg demo.cast --output demo.svg
asciinema-to-svg demo.cast --theme linux --output demo.svg
asciinema-to-svg demo.cast --theme ./themes/custom.json --width 1440 --title "Deploy" --output demo.svg
asciinema-to-svg demo.cast --statusline custom-prompt.json --output demo.svg
asciinema-to-svg demo.cast --size large --output demo.svg

# Timing
asciinema-to-svg demo.cast --speed 2 --output fast.svg
asciinema-to-svg demo.cast --idle-time-limit 0.5 --output tight.svg
asciinema-to-svg demo.cast --start 5 --end 15 --output clip.svg
asciinema-to-svg demo.cast --at 8 --output thumbnail.svg
asciinema-to-svg demo.cast --no-loop --output once.svg
```
