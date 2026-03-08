# Cast Support

## Supported Versions

- asciicast v2
- asciicast v3

## Import Rules

- Only output events (`"o"`) are rendered.
- v3 event timings are accumulated because asciicast v3 stores relative delays.
- Empty and malformed event lines are skipped.
- The cast header determines the initial terminal grid size.

## Current Limits

- Input events are ignored.
- Terminal metadata beyond rows and columns is not currently rendered.
