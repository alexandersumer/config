#!/usr/bin/env bash
set -euo pipefail

# Setup canonical agent config globally for Rovo Dev CLI and other coding agents.
# The repo remains the single source of truth:
#   ~/.agents          -> <repo>/.agents
#   ~/.rovodev/skills  -> <repo>/.agents/skills
# plus legacy /prompts compatibility backed by the same skill files.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
AGENTS_DIR="$SCRIPT_DIR/.agents"
SKILLS_DIR="$AGENTS_DIR/skills"
ROVODEV_DIR="$SCRIPT_DIR/rovodev"
GLOBAL_AGENTS_DIR="$HOME/.agents"
GLOBAL_DIR="$HOME/.rovodev"

if [ ! -d "$AGENTS_DIR" ]; then
  echo "Error: repo agents directory is missing: $AGENTS_DIR"
  exit 1
fi

if [ ! -d "$SKILLS_DIR" ]; then
  echo "Error: repo skills directory is missing: $SKILLS_DIR"
  exit 1
fi

if [ ! -d "$GLOBAL_DIR" ]; then
  echo "Error: $GLOBAL_DIR does not exist. Is Rovo Dev CLI installed?"
  exit 1
fi

realpath_of() {
  python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

resolved_symlink_target() {
  python3 -c 'import os, sys; print(os.path.realpath(os.path.join(os.path.dirname(sys.argv[1]), sys.argv[2])))' "$1" "$2"
}

link_file() {
  local source="$1"
  local target="$2"
  local label="$3"
  local existing existing_resolved source_resolved legacy_prompts_resolved

  source_resolved="$(realpath_of "$source")"

  if [ -L "$target" ]; then
    existing="$(readlink "$target")"
    existing_resolved="$(resolved_symlink_target "$target" "$existing")"
    legacy_prompts_resolved="$(realpath_of "$SCRIPT_DIR/.agents/prompts")"

    if [ "$existing_resolved" = "$source_resolved" ]; then
      echo "$label already linked correctly."
    elif [ "$existing_resolved" = "$legacy_prompts_resolved" ] && [ "$source_resolved" = "$(realpath_of "$SKILLS_DIR")" ]; then
      rm "$target"
      ln -s "$source" "$target"
      echo "Migrated $label from legacy prompts -> $source"
    else
      echo "Error: $target is already a symlink to $existing"
      echo "Remove it first if you want to replace it."
      exit 1
    fi
  elif [ -e "$target" ]; then
    if [ -d "$target" ] && [ -z "$(find "$target" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
      rmdir "$target"
      ln -s "$source" "$target"
      echo "Replaced empty $label directory with symlink -> $source"
    else
      echo "Error: $target already exists and is not an empty directory."
      echo "Back it up and remove it first, then re-run this script."
      exit 1
    fi
  else
    ln -s "$source" "$target"
    echo "Linked $label -> $source"
  fi
}

# --- global agent config ---
link_file "$AGENTS_DIR" "$GLOBAL_AGENTS_DIR" "global agents directory"

# --- native Rovo Dev skills ---
link_file "$SKILLS_DIR" "$GLOBAL_DIR/skills" "skills"

# --- legacy prompt compatibility ---
# Rovo Dev may resolve content_file entries relative to either the symlink path
# (~/.rovodev/prompts.yml) or the real prompts.yml path after resolving symlinks
# (<repo>/rovodev/prompts.yml). Keep both bases valid and backed by skills.
link_file "$SKILLS_DIR" "$ROVODEV_DIR/prompts" "repo prompt adapter"
link_file "$ROVODEV_DIR/prompts.yml" "$GLOBAL_DIR/prompts.yml" "prompts.yml"
link_file "$SKILLS_DIR" "$GLOBAL_DIR/prompts" "prompt adapter"

echo ""
echo "Done. Agent config is now managed from $SCRIPT_DIR."
echo "~/.agents points at $AGENTS_DIR, and Rovo Dev skills point at $SKILLS_DIR."
echo "Run /skills to see native skills, or /prompts to use legacy prompt commands."
