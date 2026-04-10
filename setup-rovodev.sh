#!/usr/bin/env bash
set -euo pipefail

# Setup skills as global Rovo Dev CLI prompts.
# Symlinks prompts.yml into ~/.rovodev/ so skills
# are available in every project.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROVODEV_DIR="$SCRIPT_DIR/rovodev"
GLOBAL_DIR="$HOME/.rovodev"

if [ ! -d "$GLOBAL_DIR" ]; then
  echo "Error: $GLOBAL_DIR does not exist. Is Rovo Dev CLI installed?"
  exit 1
fi

# --- prompts.yml ---
TARGET="$GLOBAL_DIR/prompts.yml"
if [ -L "$TARGET" ]; then
  existing="$(readlink "$TARGET")"
  if [ "$existing" = "$ROVODEV_DIR/prompts.yml" ]; then
    echo "prompts.yml already linked correctly."
  else
    echo "Error: $TARGET is already a symlink to $existing"
    echo "Remove it first if you want to replace it."
    exit 1
  fi
elif [ -f "$TARGET" ]; then
  echo "Error: $TARGET already exists as a regular file."
  echo "Back it up and remove it first, then re-run this script."
  exit 1
else
  ln -s "$ROVODEV_DIR/prompts.yml" "$TARGET"
  echo "Linked prompts.yml -> $ROVODEV_DIR/prompts.yml"
fi

echo ""
echo "Done. Skills are now available globally in Rovo Dev CLI."
echo "Run /prompts in Rovo Dev to see them."
