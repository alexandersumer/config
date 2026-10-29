#!/usr/bin/env python3
"""Keep immutable registry dependencies, never restored local package outputs."""
import json
import subprocess
import sys

# A malformed or incomplete inventory must never leave a cached validator usable.
metadata = json.load(sys.stdin)
packages = metadata["packages"]
if not isinstance(packages, list) or not packages:
    raise ValueError("Cargo metadata has no packages")
local = []
for package in packages:
    if "source" not in package or not isinstance(package.get("id"), str) or not package["id"]:
        raise ValueError("incomplete Cargo package metadata")
    source = package["source"]
    if source is not None and not isinstance(source, str):
        raise ValueError("invalid Cargo package source")
    if source is None or not source.startswith("registry+"):
        local.extend(["--package", package["id"]])
if not local:
    raise ValueError("Cargo metadata has no local package")
# Use Cargo's unambiguous package IDs, including path identity. A failed metadata
# or clean command is fatal; callers must not bootstrap a restored root binary.
subprocess.run(["cargo", "clean", "--locked", *local], check=True)
