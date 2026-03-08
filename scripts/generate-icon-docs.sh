#!/usr/bin/env bash
# Generate Markdown icon reference pages under docs/icons/.
# Requires: jq
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAGS_FILE="$REPO_ROOT/tags.json"
ICONS_DIR="$REPO_ROOT/icons"
DOCS_DIR="$REPO_ROOT/docs/icons"

if ! command -v jq &>/dev/null; then
  echo "error: jq is required but not found" >&2
  exit 1
fi

if [ ! -f "$TAGS_FILE" ]; then
  echo "error: tags.json not found at $TAGS_FILE" >&2
  exit 1
fi

rm -rf "$DOCS_DIR"
mkdir -p "$DOCS_DIR"

slug_for() {
  echo "$1" | tr '[:upper:]' '[:lower:]' | tr ' &' '-' | tr -s '-'
}

# --- Index page ---
cat > "$DOCS_DIR/README.md" <<'HEADER'
# Icon Reference

All icons are from [Remix Icon](https://remixicon.com/) and are licensed under the [Remix Icon License v1.0](https://github.com/Remix-Design/RemixIcon/blob/master/License).

Use icons in statusline segments by specifying the icon name:

```json
"segments": [{"icon": "apple-fill", "text": "user"}, {"icon": "folder-fill", "text": "~"}]
```

## Categories

| Category | Icons | Link |
|----------|------:|------|
HEADER

page_count=0

jq -r 'keys[] | select(. == "_comment" | not)' "$TAGS_FILE" | while IFS= read -r category; do
  count=$(jq -r --arg c "$category" '.[$c] | keys | length' "$TAGS_FILE")
  slug=$(slug_for "$category")
  echo "| $category | $count | [Browse](${slug}.md) |" >> "$DOCS_DIR/README.md"
done

# --- Per-category pages ---
jq -r 'keys[] | select(. == "_comment" | not)' "$TAGS_FILE" | while IFS= read -r category; do
  slug=$(slug_for "$category")
  outfile="$DOCS_DIR/${slug}.md"

  {
    echo "# $category Icons"
    echo ""
    echo "[Back to icon index](README.md)"
    echo ""
    echo "| Icon | Name | Tags | Usage |"
    echo "|------|------|------|-------|"
  } > "$outfile"

  jq -r --arg c "$category" '.[$c] | to_entries[] | "\(.key)\t\(.value)"' "$TAGS_FILE" | while IFS=$'\t' read -r name tags; do
    svg_path="../../icons/$category/$name.svg"
    if [ -f "$ICONS_DIR/$category/${name}.svg" ]; then
      icon_cell="<img src=\"${svg_path}\" alt=\"${name}\" width=\"24\" height=\"24\">"
    else
      icon_cell="-"
    fi
    safe_tags=$(echo "$tags" | tr ',' ', ')
    echo "| ${icon_cell} | \`${name}\` | ${safe_tags} | \`{\"icon\": \"${name}\"}\` |" >> "$outfile"
  done

  echo "Generated $outfile"
  page_count=$((page_count + 1))
done

echo "Done. Icon docs generated in $DOCS_DIR/"
