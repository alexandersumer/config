#!/usr/bin/env python3
"""Validate local Rovo Dev prompt registry and prompt files."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

import yaml

PROMPT_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
INPUT_NAME_RE = re.compile(r"^[a-z][a-z0-9_]*$")
REQUIRED_PROMPT_KEYS = {"name", "description", "content_file"}
INPUT_KEYS = {"name", "label", "description", "type", "required"}


class ValidationError(Exception):
    """Raised when prompt validation fails."""


def load_yaml_file(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return yaml.safe_load(handle)
    except yaml.YAMLError as exc:
        raise ValidationError(f"{path}: invalid YAML: {exc}") from exc
    except OSError as exc:
        raise ValidationError(f"{path}: cannot read file: {exc}") from exc


def parse_front_matter(path: Path) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ValidationError(f"{path}: cannot read prompt file: {exc}") from exc

    lines = text.splitlines()
    if not lines or lines[0] != "---":
        raise ValidationError(f"{path}: missing opening front matter delimiter")

    try:
        closing_index = lines[1:].index("---") + 1
    except ValueError as exc:
        raise ValidationError(f"{path}: missing closing front matter delimiter") from exc

    body = "\n".join(lines[closing_index + 1 :]).strip()
    if not body:
        raise ValidationError(f"{path}: prompt body must not be empty")

    try:
        metadata = yaml.safe_load("\n".join(lines[1:closing_index]))
    except yaml.YAMLError as exc:
        raise ValidationError(f"{path}: invalid front matter YAML: {exc}") from exc

    if not isinstance(metadata, dict):
        raise ValidationError(f"{path}: front matter must be a mapping")

    return metadata


def validate_prompt_name(name: Any, context: str) -> str:
    if not isinstance(name, str) or not name:
        raise ValidationError(f"{context}: name must be a non-empty string")
    if not PROMPT_NAME_RE.fullmatch(name):
        raise ValidationError(f"{context}: prompt name must be kebab-case: {name!r}")
    return name


def validate_input_name(name: Any, context: str) -> str:
    if not isinstance(name, str) or not name:
        raise ValidationError(f"{context}: input name must be a non-empty string")
    if not INPUT_NAME_RE.fullmatch(name):
        raise ValidationError(f"{context}: input name must be lower_snake_case: {name!r}")
    return name


def validate_description(description: Any, context: str) -> str:
    if not isinstance(description, str) or not description.strip():
        raise ValidationError(f"{context}: description must be a non-empty string")
    if "\n" in description:
        raise ValidationError(f"{context}: description must be a single line")
    return description


def normalize_inputs(inputs: Any, context: str) -> list[dict[str, Any]]:
    if inputs is None:
        return []
    if not isinstance(inputs, list):
        raise ValidationError(f"{context}: inputs must be a list")

    normalized: list[dict[str, Any]] = []
    seen_names: set[str] = set()
    for index, item in enumerate(inputs):
        item_context = f"{context}: inputs[{index}]"
        if not isinstance(item, dict):
            raise ValidationError(f"{item_context}: input must be a mapping")
        missing = INPUT_KEYS - item.keys()
        if missing:
            raise ValidationError(f"{item_context}: missing keys: {sorted(missing)}")
        extra = item.keys() - INPUT_KEYS
        if extra:
            raise ValidationError(f"{item_context}: unsupported keys: {sorted(extra)}")

        name = validate_input_name(item.get("name"), item_context)
        if name in seen_names:
            raise ValidationError(f"{item_context}: duplicate input name: {name}")
        seen_names.add(name)

        label = item.get("label")
        description = item.get("description")
        input_type = item.get("type")
        required = item.get("required")
        if not isinstance(label, str) or not label.strip():
            raise ValidationError(f"{item_context}: label must be a non-empty string")
        validate_description(description, item_context)
        if input_type != "string":
            raise ValidationError(f"{item_context}: only string inputs are currently supported")
        if not isinstance(required, bool):
            raise ValidationError(f"{item_context}: required must be a boolean")

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


def resolve_content_path(repo_root: Path, content_file: Any, context: str) -> Path:
    if not isinstance(content_file, str) or not content_file:
        raise ValidationError(f"{context}: content_file must be a non-empty string")
    if Path(content_file).is_absolute() or ".." in Path(content_file).parts:
        raise ValidationError(f"{context}: content_file must be a relative path inside prompts/")
    if not content_file.startswith("prompts/"):
        raise ValidationError(f"{context}: content_file must start with prompts/")
    if not content_file.endswith("/SKILL.md"):
        raise ValidationError(f"{context}: content_file must point to a SKILL.md file")
    return repo_root / "rovodev" / content_file


def validate_registry(repo_root: Path) -> list[str]:
    config_path = repo_root / "rovodev" / "prompts.yml"
    config = load_yaml_file(config_path)
    if not isinstance(config, dict):
        raise ValidationError(f"{config_path}: top-level document must be a mapping")
    prompts = config.get("prompts")
    if not isinstance(prompts, list) or not prompts:
        raise ValidationError(f"{config_path}: prompts must be a non-empty list")

    errors: list[str] = []
    seen_names: set[str] = set()
    registered_skill_files: set[Path] = set()

    for index, prompt in enumerate(prompts):
        context = f"{config_path}: prompts[{index}]"
        if not isinstance(prompt, dict):
            errors.append(f"{context}: prompt must be a mapping")
            continue

        missing = REQUIRED_PROMPT_KEYS - prompt.keys()
        if missing:
            errors.append(f"{context}: missing keys: {sorted(missing)}")
            continue

        try:
            name = validate_prompt_name(prompt.get("name"), context)
            description = validate_description(prompt.get("description"), context)
            registry_inputs = normalize_inputs(prompt.get("inputs"), context)
            content_path = resolve_content_path(repo_root, prompt.get("content_file"), context)
        except ValidationError as exc:
            errors.append(str(exc))
            continue

        if name in seen_names:
            errors.append(f"{context}: duplicate prompt name: {name}")
            continue
        seen_names.add(name)

        expected_content_file = f"prompts/{name}/SKILL.md"
        if prompt.get("content_file") != expected_content_file:
            errors.append(
                f"{context}: content_file must be {expected_content_file!r} for prompt {name!r}"
            )

        if not content_path.is_file():
            errors.append(f"{context}: content_file does not exist: {content_path}")
            continue
        registered_skill_files.add(content_path.resolve())

        try:
            metadata = parse_front_matter(content_path)
            front_matter_name = validate_prompt_name(metadata.get("name"), str(content_path))
            front_matter_description = validate_description(metadata.get("description"), str(content_path))
            front_matter_inputs = normalize_inputs(metadata.get("inputs"), str(content_path))
        except ValidationError as exc:
            errors.append(str(exc))
            continue

        if front_matter_name != name:
            errors.append(
                f"{content_path}: front matter name {front_matter_name!r} does not match registry name {name!r}"
            )
        if front_matter_description != description:
            errors.append(
                f"{content_path}: front matter description does not match registry description"
            )
        if front_matter_inputs != registry_inputs:
            errors.append(f"{content_path}: front matter inputs do not match registry inputs")

    prompt_root = repo_root / ".agents" / "prompts"
    for skill_file in sorted(prompt_root.glob("*/SKILL.md")):
        if skill_file.resolve() not in registered_skill_files:
            errors.append(f"{skill_file}: prompt file is not registered in {config_path}")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        default=Path(__file__).resolve().parents[1],
        type=Path,
        help="Repository root to validate. Defaults to this script's repository.",
    )
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    errors = validate_registry(repo_root)
    if errors:
        print("Prompt validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("Prompt validation passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
