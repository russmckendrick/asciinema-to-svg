# Architecture

`asciinema-to-svg` is a small Rust binary with five core modules:

- `cli`: parses the single conversion command
- `cast`: imports asciicast v2 and v3 into one in-memory session model
- `terminal`: replays ANSI output into a screen buffer
- `theme`: loads built-in or custom theme JSON files
- `render`: turns replayed frames into animated SVG

## Data Flow

1. Read the cast header and events from disk.
2. Normalize v3 relative delays into absolute timestamps.
3. Replay each output event through the ANSI parser and screen buffer.
4. Capture a frame after each event.
5. Render the frames inside the selected themed terminal window.

## Scope

- Conversion only
- No recording pipeline
- No shell execution
- No non-SVG outputs
