#!/bin/bash
set -euo pipefail

echo "📦 Running: cargo run -- --help"
echo "----------------------------------------"
HELP_OUTPUT=$(cargo run -- --help)
echo "$HELP_OUTPUT"
echo "----------------------------------------"

# Markdown-formatted block
BLOCK="\`\`\`console
\$ zotexon --help
$HELP_OUTPUT
\`\`\`"

if [[ "${DRY_RUN:-}" == "true" ]]; then
  echo "🚫 DRY_RUN is set — README.md will not be modified."
  echo "🔍 Would replace the following block in README.md:"
  echo "$BLOCK"
else
  echo "📝 Updating README.md..."

  awk -v block="$BLOCK" '
    BEGIN { print_block = 1 }
    /<!-- cli-help-start -->/ { print; print block; print_block = 0; next }
    /<!-- cli-help-end -->/ { print_block = 1 }
    print_block
  ' README.md > README.tmp && mv README.tmp README.md

  echo "✅ README.md successfully updated."
fi