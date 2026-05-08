#!/usr/bin/env python3
"""Validate canonical agent skills and generated Rovo Dev prompt compatibility."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

import yaml

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

from generate_prompts import (  # type: ignore[import-not-found]
    SkillError,
    collect_skills,
    dump_registry,
    generate_registry,
    load_yaml_file,
    load_prompt_metadata,
    normalize_inputs,
    parse_front_matter,
    validate_description,
    validate_skill_name,
)

REQUIRED_PROMPT_KEYS = {"name", "description", "content_file"}
ALLOWED_PROMPT_KEYS = REQUIRED_PROMPT_KEYS | {"inputs"}


class ValidationError(Exception):
    """Raised when validation fails."""


def resolve_content_path(repo_root: Path, content_file: Any, context: str) -> Path:
    if not isinstance(content_file, str) or not content_file:
        raise ValidationError(f"{context}: content_file must be a non-empty string")
    path = Path(content_file)
    if path.is_absolute() or ".." in path.parts:
        raise ValidationError(f"{context}: content_file must be a relative path inside prompts/")
    if not content_file.startswith("prompts/"):
        raise ValidationError(f"{context}: content_file must start with prompts/")
    if not content_file.endswith("/SKILL.md"):
        raise ValidationError(f"{context}: content_file must point to a SKILL.md file")
    return repo_root / "rovodev" / content_file


def validate_prompts_adapter(repo_root: Path) -> list[str]:
    errors: list[str] = []
    prompts_link = repo_root / "rovodev" / "prompts"
    skills_root = repo_root / ".agents" / "skills"

    if not prompts_link.is_symlink():
        errors.append(f"{prompts_link}: must be a symlink to {skills_root}")
    elif prompts_link.resolve() != skills_root.resolve():
        errors.append(f"{prompts_link}: must resolve to {skills_root}; got {prompts_link.resolve()}")

    config_path = repo_root / "rovodev" / "prompts.yml"
    try:
        config = load_yaml_file(config_path)
    except SkillError as exc:
        return errors + [str(exc)]

    if not isinstance(config, dict):
        return errors + [f"{config_path}: top-level document must be a mapping"]
    prompts = config.get("prompts")
    if not isinstance(prompts, list) or not prompts:
        return errors + [f"{config_path}: prompts must be a non-empty list"]

    try:
        prompt_metadata = load_prompt_metadata(repo_root)
    except SkillError as exc:
        return errors + [str(exc)]

    seen_names: set[str] = set()
    for index, prompt in enumerate(prompts):
        context = f"{config_path}: prompts[{index}]"
        if not isinstance(prompt, dict):
            errors.append(f"{context}: prompt must be a mapping")
            continue

        missing = REQUIRED_PROMPT_KEYS - prompt.keys()
        if missing:
            errors.append(f"{context}: missing keys: {sorted(missing)}")
            continue
        extra = prompt.keys() - ALLOWED_PROMPT_KEYS
        if extra:
            errors.append(f"{context}: unsupported keys: {sorted(extra)}")

        try:
            name = validate_skill_name(prompt.get("name"), context)
            description = validate_description(prompt.get("description"), context)
            registry_inputs = normalize_inputs(prompt.get("inputs"), context)
            content_path = resolve_content_path(repo_root, prompt.get("content_file"), context)
        except SkillError as exc:
            errors.append(str(exc))
            continue
        except ValidationError as exc:
            errors.append(str(exc))
            continue

        if name in seen_names:
            errors.append(f"{context}: duplicate prompt name: {name}")
            continue
        seen_names.add(name)

        expected_content_file = f"prompts/{name}/SKILL.md"
        if prompt.get("content_file") != expected_content_file:
            errors.append(f"{context}: content_file must be {expected_content_file!r} for prompt {name!r}")

        if not content_path.is_file():
            errors.append(f"{context}: content_file does not exist: {content_path}")
            continue

        try:
            metadata = parse_front_matter(content_path)
            front_matter_name = validate_skill_name(metadata.get("name"), str(content_path))
            front_matter_description = validate_description(metadata.get("description"), str(content_path))
            extra_keys = metadata.keys() - {"name", "description", "license", "compatibility", "metadata", "allowed-tools"}
            if extra_keys:
                raise SkillError(f"{content_path}: unsupported front matter keys: {sorted(extra_keys)}")
            metadata_inputs = prompt_metadata.get(name, {}).get("inputs", [])
        except SkillError as exc:
            errors.append(str(exc))
            continue

        if front_matter_name != name:
            errors.append(f"{content_path}: front matter name {front_matter_name!r} does not match registry name {name!r}")
        if front_matter_description != description:
            errors.append(f"{content_path}: front matter description does not match registry description")
        if metadata_inputs != registry_inputs:
            errors.append(f"{repo_root / 'rovodev' / 'prompt-metadata.yml'}: inputs for {name!r} do not match registry inputs")

    unknown_metadata = set(prompt_metadata) - seen_names
    if unknown_metadata:
        errors.append(f"{repo_root / 'rovodev' / 'prompt-metadata.yml'}: metadata for unknown skills: {sorted(unknown_metadata)}")

    return errors


def validate_generated_registry(repo_root: Path) -> list[str]:
    config_path = repo_root / "rovodev" / "prompts.yml"
    try:
        expected = dump_registry(generate_registry(repo_root))
        actual = config_path.read_text(encoding="utf-8")
    except (SkillError, OSError) as exc:
        return [str(exc)]
    if actual != expected:
        return [f"{config_path}: generated content is not up to date; run scripts/generate-prompts.py"]
    return []


def validate_registry(repo_root: Path) -> list[str]:
    errors: list[str] = []
    try:
        collect_skills(repo_root)
    except SkillError as exc:
        errors.append(str(exc))
    errors.extend(validate_prompts_adapter(repo_root))
    errors.extend(validate_generated_registry(repo_root))
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
        print("Skill validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("Skill validation passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
