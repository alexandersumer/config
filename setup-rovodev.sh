#!/usr/bin/env bash
set -euo pipefail

# Setup skills as global Rovo Dev CLI prompts.
# Symlinks prompts.yml and prompt content into ~/.rovodev/
# so skills are available in every project.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROVODEV_DIR="$SCRIPT_DIR/rovodev"
GLOBAL_DIR="$HOME/.rovodev"

if [ ! -d "$GLOBAL_DIR" ]; then
  echo "Error: $GLOBAL_DIR does not exist. Is Rovo Dev CLI installed?"
  exit 1
fi

link_file() {
  local source="$1"
  local target="$2"
  local label="$3"

  if [ -L "$target" ]; then
    existing="$(readlink "$target")"
    existing_resolved="$(python3 -c 'import os, sys; print(os.path.realpath(os.path.join(os.path.dirname(sys.argv[1]), sys.argv[2])))' "$target" "$existing")"
    source_resolved="$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$source")"

    if [ "$existing_resolved" = "$source_resolved" ]; then
      echo "$label already linked correctly."
    else
      echo "Error: $target is already a symlink to $existing"
      echo "Remove it first if you want to replace it."
      exit 1
    fi
  elif [ -e "$target" ]; then
    echo "Error: $target already exists."
    echo "Back it up and remove it first, then re-run this script."
    exit 1
  else
    ln -s "$source" "$target"
    echo "Linked $label -> $source"
  fi
}

# --- prompts.yml ---
link_file "$ROVODEV_DIR/prompts.yml" "$GLOBAL_DIR/prompts.yml" "prompts.yml"

# Rovo Dev resolves content_file entries relative to ~/.rovodev/prompts.yml.
# Keep prompt content under ~/.rovodev as well, instead of claiming a generic
# ~/.agents namespace.
link_file "$SCRIPT_DIR/.agents/prompts" "$GLOBAL_DIR/prompts" "prompts"

echo ""
echo "Done. Skills are now available globally in Rovo Dev CLI."
echo "Run /prompts in Rovo Dev to see them."
