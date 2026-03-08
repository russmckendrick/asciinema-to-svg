# Powerline Remapping

Powerline/starship prompt remapping is enabled by default.

## Behavior

- The renderer looks for prompt rows containing common powerline separator glyphs such as ``, ``, ``, or ``.
- When detected, the prompt is redrawn with SVG shapes owned by the renderer rather than relying on terminal font glyphs.
- The selected theme controls the prompt font, colors, separators, leading symbol, and trailing symbol.

## Disable

Use `--no-powerline` to render the original prompt text without remapping.

## Current Heuristic

- Detection is intentionally conservative and row-based.
- Non-prompt rows are rendered as plain terminal text.
- The default `macos` theme uses the supplied personal prompt look as the reference direction.
