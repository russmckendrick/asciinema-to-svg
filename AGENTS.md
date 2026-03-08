# asciinema-to-svg

Rust CLI for converting asciinema v2/v3 cast files into themed animated SVG output.

## Quick Reference

- Build: `cargo build`
- Test: `cargo test`
- Format: `cargo fmt`
- Run: `cargo run -- <input.cast> --output output.svg`

## Working Rules

- Keep the project conversion-only. Do not add recording or shell-spawning features.
- Treat `themes/*.json` as the primary customization surface for visuals and prompt styling.
- Update the relevant file in `docs/` whenever the CLI, theme schema, or rendering behavior changes.
- Keep behavior-changing tests close to the touched module.

## Entry Points

- Docs index: [docs/README.md](/Users/russ.mckendrick/Code/asciinema-to-svg/docs/README.md)
- CLI: [src/cli.rs](/Users/russ.mckendrick/Code/asciinema-to-svg/src/cli.rs)
- Cast parsing: [src/cast.rs](/Users/russ.mckendrick/Code/asciinema-to-svg/src/cast.rs)
- Themes: [src/theme.rs](/Users/russ.mckendrick/Code/asciinema-to-svg/src/theme.rs)
- Rendering: [src/render/mod.rs](/Users/russ.mckendrick/Code/asciinema-to-svg/src/render/mod.rs)
