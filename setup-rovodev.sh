#!/usr/bin/env bash
set -euo pipefail

# Setup canonical agent skills globally for Rovo Dev CLI.
# Symlinks .agents/skills into ~/.rovodev/skills for native skill discovery,
# and keeps legacy /prompts compatibility backed by the same skill files.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKILLS_DIR="$SCRIPT_DIR/.agents/skills"
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
    legacy_prompts_resolved="$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$SCRIPT_DIR/.agents/prompts")"

    if [ "$existing_resolved" = "$source_resolved" ]; then
      echo "$label already linked correctly."
    elif [ "$existing_resolved" = "$legacy_prompts_resolved" ] && [ "$source_resolved" = "$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$SKILLS_DIR")" ]; then
      rm "$target"
      ln -s "$source" "$target"
      echo "Migrated $label from legacy prompts -> $source"
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

# --- native skills ---
link_file "$SKILLS_DIR" "$GLOBAL_DIR/skills" "skills"

# --- legacy prompt compatibility ---
# Rovo Dev may resolve content_file entries relative to either the symlink path
# (~/.rovodev/prompts.yml) or the real prompts.yml path after resolving symlinks
# (<repo>/rovodev/prompts.yml). Keep both bases valid and backed by skills.
link_file "$SKILLS_DIR" "$ROVODEV_DIR/prompts" "repo prompt adapter"
link_file "$ROVODEV_DIR/prompts.yml" "$GLOBAL_DIR/prompts.yml" "prompts.yml"
link_file "$SKILLS_DIR" "$GLOBAL_DIR/prompts" "prompt adapter"

echo ""
echo "Done. Skills are now available globally in Rovo Dev CLI."
echo "Run /skills to see native skills, or /prompts to use legacy prompt commands."
