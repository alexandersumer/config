# gap-coding-skills

Shared coding skills for the Gen AI Platform team. Each skill is a `.md` file at the repo root with YAML front-matter (`name`, `description`). Browse the files to see what's available.

## Setup

**Claude Code** — import individual skills in your project's `CLAUDE.md`:

```
/import-skill path/to/gap-coding-skills/<skill>.md
```

**Cursor** — symlink each skill:

```sh
mkdir -p ~/.cursor/skills-cursor/git-commit
ln -s /path/to/gap-coding-skills/git-commit.md ~/.cursor/skills-cursor/git-commit/SKILL.md
```

**Rovo Dev CLI** — register all skills globally:

```sh
./setup-rovodev.sh
```

## Adding a skill

First-time setup — enable the pre-commit hook so `rovodev/prompts.yml` stays in sync automatically:

```sh
git config core.hooksPath hooks
```

Then:

1. Create the `.md` file with front-matter:

```markdown
---
name: my-skill
description: What it does
---
Instructions for the agent.
```

2. Symlink it for Rovo Dev:

```sh
ln -s ../../my-skill.md rovodev/templates/my-skill.md
```

The pre-commit hook regenerates `rovodev/prompts.yml` on commit. To regenerate manually: `./generate-rovodev-prompts-yml`.
