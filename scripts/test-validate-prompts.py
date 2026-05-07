#!/usr/bin/env python3
"""Regression tests for scripts/validate-prompts.py.

These tests copy the skill registry into temporary repositories, introduce
realistic skill/prompt compatibility regressions, and assert that the validator
catches each one.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Callable

import yaml

REPO_ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = REPO_ROOT / "scripts" / "validate-prompts.py"
GENERATOR = REPO_ROOT / "scripts" / "generate-prompts.py"


def copy_fixture() -> Path:
    temp_dir = Path(tempfile.mkdtemp(prefix="tmp_rovodev_skill_validation_"))
    shutil.copytree(REPO_ROOT / ".agents", temp_dir / ".agents", symlinks=True)
    shutil.copytree(REPO_ROOT / "rovodev", temp_dir / "rovodev", symlinks=True)
    return temp_dir


def run_validator(repo_root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(VALIDATOR), "--repo-root", str(repo_root)],
        text=True,
        capture_output=True,
        check=False,
    )


def run_generator(repo_root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(GENERATOR), "--repo-root", str(repo_root)],
        text=True,
        capture_output=True,
        check=False,
    )


def load_config(repo_root: Path) -> dict:
    config_path = repo_root / "rovodev" / "prompts.yml"
    with config_path.open("r", encoding="utf-8") as handle:
        return yaml.safe_load(handle)


def write_config(repo_root: Path, config: dict) -> None:
    config_path = repo_root / "rovodev" / "prompts.yml"
    with config_path.open("w", encoding="utf-8") as handle:
        yaml.safe_dump(config, handle, sort_keys=False)


def mutate_missing_skill_file(repo_root: Path) -> str:
    config = load_config(repo_root)
    name = config["prompts"][0]["name"]
    (repo_root / ".agents" / "skills" / name / "SKILL.md").unlink()
    return "content_file does not exist"


def mutate_front_matter_name_mismatch(repo_root: Path) -> str:
    config = load_config(repo_root)
    name = config["prompts"][0]["name"]
    skill_path = repo_root / ".agents" / "skills" / name / "SKILL.md"
    text = skill_path.read_text(encoding="utf-8")
    skill_path.write_text(text.replace(f"name: {name}", "name: broken-name", 1), encoding="utf-8")
    return "does not match directory"


def mutate_generated_registry_drift(repo_root: Path) -> str:
    config = load_config(repo_root)
    prompt = next(item for item in config["prompts"] if item["name"] == "apply-changes")
    prompt["inputs"][0]["required"] = True
    write_config(repo_root, config)
    return "generated content is not up to date"


def mutate_unregistered_skill_file(repo_root: Path) -> str:
    extra_dir = repo_root / ".agents" / "skills" / "unregistered-skill"
    extra_dir.mkdir(parents=True)
    (extra_dir / "SKILL.md").write_text(
        "---\n"
        "name: unregistered-skill\n"
        "description: Unregistered skill\n"
        "---\n\n"
        "This skill is intentionally not listed in the generated registry.\n",
        encoding="utf-8",
    )
    return "generated content is not up to date"


def mutate_prompt_adapter_broken(repo_root: Path) -> str:
    (repo_root / "rovodev" / "prompts").unlink()
    (repo_root / "rovodev" / "prompts").symlink_to("../.agents/missing")
    return "must resolve to"


def assert_clean_fixture_passes() -> None:
    fixture = copy_fixture()
    try:
        generated = run_generator(fixture)
        assert generated.returncode == 0, generated.stderr or generated.stdout
        result = run_validator(fixture)
        assert result.returncode == 0, result.stderr or result.stdout
    finally:
        shutil.rmtree(fixture)


def assert_mutation_fails(name: str, mutate: Callable[[Path], str]) -> None:
    fixture = copy_fixture()
    try:
        expected = mutate(fixture)
        result = run_validator(fixture)
        output = result.stdout + result.stderr
        assert result.returncode != 0, f"{name}: validator unexpectedly passed\n{output}"
        assert expected in output, f"{name}: expected {expected!r} in output\n{output}"
    finally:
        shutil.rmtree(fixture)


def main() -> int:
    assert_clean_fixture_passes()
    mutations = {
        "missing skill file": mutate_missing_skill_file,
        "front matter name mismatch": mutate_front_matter_name_mismatch,
        "generated registry drift": mutate_generated_registry_drift,
        "unregistered skill file": mutate_unregistered_skill_file,
        "broken prompt adapter": mutate_prompt_adapter_broken,
    }
    for name, mutate in mutations.items():
        assert_mutation_fails(name, mutate)
    print("Skill validator regression tests passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
