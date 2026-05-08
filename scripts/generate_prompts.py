#!/usr/bin/env python3
"""Generate the Rovo Dev prompt registry from canonical agent skills."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

import yaml

SKILL_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
INPUT_NAME_RE = re.compile(r"^[a-z][a-z0-9_]*$")
INPUT_KEYS = {"name", "label", "description", "type", "required"}
SKILL_FRONT_MATTER_KEYS = {"name", "description", "license", "compatibility", "metadata", "allowed-tools"}
PROMPT_METADATA_KEYS = {"inputs"}


class SkillError(Exception):
    """Raised when skill metadata is invalid."""


def load_yaml_file(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return yaml.safe_load(handle)
    except yaml.YAMLError as exc:
        raise SkillError(f"{path}: invalid YAML: {exc}") from exc
    except OSError as exc:
        raise SkillError(f"{path}: cannot read file: {exc}") from exc


def parse_front_matter(path: Path) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise SkillError(f"{path}: cannot read skill file: {exc}") from exc

    lines = text.splitlines()
    if not lines or lines[0] != "---":
        raise SkillError(f"{path}: missing opening front matter delimiter")

    try:
        closing_index = lines[1:].index("---") + 1
    except ValueError as exc:
        raise SkillError(f"{path}: missing closing front matter delimiter") from exc

    body = "\n".join(lines[closing_index + 1 :]).strip()
    if not body:
        raise SkillError(f"{path}: skill body must not be empty")

    try:
        metadata = yaml.safe_load("\n".join(lines[1:closing_index]))
    except yaml.YAMLError as exc:
        raise SkillError(f"{path}: invalid front matter YAML: {exc}") from exc

    if not isinstance(metadata, dict):
        raise SkillError(f"{path}: front matter must be a mapping")

    return metadata


def validate_skill_name(name: Any, context: str) -> str:
    if not isinstance(name, str) or not name:
        raise SkillError(f"{context}: name must be a non-empty string")
    if not SKILL_NAME_RE.fullmatch(name):
        raise SkillError(f"{context}: skill name must be kebab-case: {name!r}")
    return name


def validate_input_name(name: Any, context: str) -> str:
    if not isinstance(name, str) or not name:
        raise SkillError(f"{context}: input name must be a non-empty string")
    if not INPUT_NAME_RE.fullmatch(name):
        raise SkillError(f"{context}: input name must be lower_snake_case: {name!r}")
    return name


def validate_description(description: Any, context: str) -> str:
    if not isinstance(description, str) or not description.strip():
        raise SkillError(f"{context}: description must be a non-empty string")
    if "\n" in description:
        raise SkillError(f"{context}: description must be a single line")
    return description


def normalize_inputs(inputs: Any, context: str) -> list[dict[str, Any]]:
    if inputs is None:
        return []
    if not isinstance(inputs, list):
        raise SkillError(f"{context}: inputs must be a list")

    normalized: list[dict[str, Any]] = []
    seen_names: set[str] = set()
    for index, item in enumerate(inputs):
        item_context = f"{context}: inputs[{index}]"
        if not isinstance(item, dict):
            raise SkillError(f"{item_context}: input must be a mapping")
        missing = INPUT_KEYS - item.keys()
        if missing:
            raise SkillError(f"{item_context}: missing keys: {sorted(missing)}")
        extra = item.keys() - INPUT_KEYS
        if extra:
            raise SkillError(f"{item_context}: unsupported keys: {sorted(extra)}")

        name = validate_input_name(item.get("name"), item_context)
        if name in seen_names:
            raise SkillError(f"{item_context}: duplicate input name: {name}")
        seen_names.add(name)

        label = item.get("label")
        description = item.get("description")
        input_type = item.get("type")
        required = item.get("required")
        if not isinstance(label, str) or not label.strip():
            raise SkillError(f"{item_context}: label must be a non-empty string")
        validate_description(description, item_context)
        if input_type != "string":
            raise SkillError(f"{item_context}: only string inputs are currently supported")
        if not isinstance(required, bool):
            raise SkillError(f"{item_context}: required must be a boolean")

        normalized.append(
            {
                "name": name,
                "label": label,
                "description": description,
                "type": input_type,
                "required": required,
            }
        )
    return normalized


def existing_prompt_order(repo_root: Path) -> dict[str, int]:
    config_path = repo_root / "rovodev" / "prompts.yml"
    if not config_path.exists():
        return {}
    try:
        config = load_yaml_file(config_path)
    except SkillError:
        return {}
    prompts = config.get("prompts") if isinstance(config, dict) else None
    if not isinstance(prompts, list):
        return {}
    order: dict[str, int] = {}
    for index, prompt in enumerate(prompts):
        if isinstance(prompt, dict) and isinstance(prompt.get("name"), str):
            order.setdefault(prompt["name"], index)
    return order


def load_prompt_metadata(repo_root: Path) -> dict[str, dict[str, Any]]:
    metadata_path = repo_root / "rovodev" / "prompt-metadata.yml"
    if not metadata_path.exists():
        return {}
    metadata = load_yaml_file(metadata_path)
    if metadata is None:
        return {}
    if not isinstance(metadata, dict):
        raise SkillError(f"{metadata_path}: top-level document must be a mapping")
    prompts = metadata.get("prompts", {})
    if not isinstance(prompts, dict):
        raise SkillError(f"{metadata_path}: prompts must be a mapping")

    normalized: dict[str, dict[str, Any]] = {}
    for name, prompt_metadata in prompts.items():
        context = f"{metadata_path}: prompts[{name!r}]"
        skill_name = validate_skill_name(name, context)
        if not isinstance(prompt_metadata, dict):
            raise SkillError(f"{context}: metadata must be a mapping")
        extra = prompt_metadata.keys() - PROMPT_METADATA_KEYS
        if extra:
            raise SkillError(f"{context}: unsupported keys: {sorted(extra)}")
        normalized[skill_name] = {
            "inputs": normalize_inputs(prompt_metadata.get("inputs"), context),
        }
    return normalized


def collect_skills(repo_root: Path) -> list[dict[str, Any]]:
    skills_root = repo_root / ".agents" / "skills"
    if not skills_root.is_dir():
        raise SkillError(f"{skills_root}: canonical skills directory does not exist")

    prompt_metadata = load_prompt_metadata(repo_root)
    skills: list[dict[str, Any]] = []
    seen_names: set[str] = set()
    for skill_file in sorted(skills_root.glob("*/SKILL.md")):
        skill_dir = skill_file.parent.name
        context = str(skill_file)
        validate_skill_name(skill_dir, str(skill_file.parent))
        metadata = parse_front_matter(skill_file)
        extra_keys = metadata.keys() - SKILL_FRONT_MATTER_KEYS
        if extra_keys:
            raise SkillError(f"{skill_file}: unsupported front matter keys: {sorted(extra_keys)}")
        name = validate_skill_name(metadata.get("name"), context)
        if name != skill_dir:
            raise SkillError(f"{skill_file}: front matter name {name!r} does not match directory {skill_dir!r}")
        if name in seen_names:
            raise SkillError(f"{skill_file}: duplicate skill name: {name}")
        seen_names.add(name)

        prompt: dict[str, Any] = {
            "name": name,
            "description": validate_description(metadata.get("description"), context),
            "content_file": f"prompts/{name}/SKILL.md",
        }
        inputs = prompt_metadata.get(name, {}).get("inputs", [])
        if inputs:
            prompt["inputs"] = inputs
        skills.append(prompt)

    unknown_metadata = set(prompt_metadata) - seen_names
    if unknown_metadata:
        raise SkillError(f"{repo_root / 'rovodev' / 'prompt-metadata.yml'}: metadata for unknown skills: {sorted(unknown_metadata)}")

    if not skills:
        raise SkillError(f"{skills_root}: no skills found")
    return skills


def generate_registry(repo_root: Path) -> dict[str, list[dict[str, Any]]]:
    order = existing_prompt_order(repo_root)
    prompts = collect_skills(repo_root)
    prompts.sort(key=lambda prompt: (order.get(prompt["name"], len(order)), prompt["name"]))
    return {"prompts": prompts}


def dump_registry(registry: dict[str, list[dict[str, Any]]]) -> str:
    return yaml.safe_dump(registry, sort_keys=False, allow_unicode=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        default=Path(__file__).resolve().parents[1],
        type=Path,
        help="Repository root. Defaults to this script's repository.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check that rovodev/prompts.yml is up to date without writing it.",
    )
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    output_path = repo_root / "rovodev" / "prompts.yml"
    try:
        rendered = dump_registry(generate_registry(repo_root))
    except SkillError as exc:
        print(f"Prompt generation failed:\n- {exc}", file=sys.stderr)
        return 1

    if args.check:
        try:
            current = output_path.read_text(encoding="utf-8")
        except OSError as exc:
            print(f"Prompt generation check failed:\n- {output_path}: cannot read file: {exc}", file=sys.stderr)
            return 1
        if current != rendered:
            print(f"Prompt generation check failed:\n- {output_path}: generated content is not up to date", file=sys.stderr)
            return 1
        print("Prompt registry is up to date.")
        return 0

    output_path.write_text(rendered, encoding="utf-8")
    print(f"Generated {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
