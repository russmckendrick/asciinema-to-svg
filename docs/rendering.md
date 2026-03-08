# Rendering

The renderer replays the cast into an in-memory terminal and captures a frame after each output event. Each frame becomes an SVG group with step-based animation timing.

## Window Themes

- `macos`: rounded chrome and traffic-light controls
- `linux`: Ubuntu-style top bar and terminal colors
- `powershell`: Windows PowerShell style chrome and palette

## Size Rules

- If neither `--width` nor `--height` is given, the SVG uses its natural size from font metrics, terminal rows and columns, and chrome padding.
- If only one dimension is set, the renderer preserves aspect ratio.
- If both are set, the renderer uses them directly.

## Terminal Rendering

- ANSI foreground and background colors are preserved.
- Bold, italic, underline, reverse video, cursor movement, clears, and basic deletion commands are supported.
- Wide characters are handled in the screen buffer.
