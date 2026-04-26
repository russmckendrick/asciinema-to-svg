# Rendering

The renderer replays the cast into an in-memory terminal and captures a frame after each output event. Identical consecutive buffers are deduplicated. Each surviving frame becomes an SVG `<g class="frame">` with step-based opacity keyframes.

## Pipeline

```
cast → emulator.replay() → dedup → apply_timing → static-frame select → normalize → SVG
```

The `apply_timing` step runs the CLI flags `--start`, `--end`, `--idle-time-limit`, and `--speed` (in that order). The static-frame step (`--at`) picks the buffer at the requested time and shortcuts the renderer into a single-frame, no-animation output.

## Window Themes

- `macos`: rounded chrome and traffic-light controls
- `linux`: Ubuntu-style top bar and terminal colors
- `powershell`: Windows PowerShell style chrome and palette

## Size Rules

- If neither `--width` nor `--height` is given, the SVG uses its natural size from font metrics, terminal rows and columns, and chrome padding.
- If only one dimension is set, the renderer preserves aspect ratio.
- If both are set, the renderer uses them directly.

## Pixel Snapping

Every coordinate emitted inside `<g class="frame">` is snapped to a whole pixel at emit time (rect/text/line/polygon `x`, `y`, `width`, `height`, points). This is essential on Safari, where each frame's `<g>` becomes a GPU compositor layer and fractional sub-pixel coords get re-rasterized with slight variance per cycle, producing visible shimmer.

Cell left/right edges are snapped independently (`round(frame_x + col * cell_width)`), so cell widths can vary by ±1px while the row total still matches `terminal_width` exactly. The `.frame` rule also sets `will-change: opacity; transform: translateZ(0);` to keep each frame on a stable GPU layer.

The chrome (rounded title bar, traffic-light circles, divider line) lives outside `<g class="frame">`. Its title text and circles are also snapped to integer pixels; rounded corners and the 1px divider intentionally use fractional positioning.

## Animation

- Each frame is an `<g class="frame" style="animation-name: frame-N">` with a per-frame `@keyframes` block toggling `opacity` between `0` and `1` with `steps(1, end)` timing.
- `--no-loop` switches `animation-iteration-count` from `infinite` to `1`.
- `--at` produces a static SVG with no keyframes and no `.frame` wrapper.

## Terminal Rendering

- ANSI foreground and background colors are preserved (16-color, 256-color, truecolor).
- Bold, italic, underline, strikethrough, overline, reverse video, faint, cursor movement, clears, scroll regions, alt-screen, and basic deletion commands are supported.
- Cursor visibility (`?25h/l`) is tracked but not yet rendered.
- OSC 0/2 window titles are captured and used when `--title` isn't set.
- Wide characters and combining marks are handled in the screen buffer.
