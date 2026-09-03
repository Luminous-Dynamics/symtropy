#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Deterministically summarize semantic differences between two Cargo.lock files.

This tool is intentionally descriptive, not policy-enforcing. It reports package
record additions/removals, name-level version/source set changes, checksum
changes, and dependency-list changes. Stable expectations may be promoted into
policy only after real Stage A/Stage B resolver output has been reviewed.
"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path
from typing import Any


Identity = tuple[str, str, str]


def load_lock(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: Cargo.lock root is not a TOML table")
    packages = data.get("package", [])
    if not isinstance(packages, list):
        raise ValueError(f"{path}: package is not an array of tables")
    return data


def normalized_source(package: dict[str, Any]) -> str:
    source = package.get("source")
    if source is None:
        return "<workspace>"
    if not isinstance(source, str):
        raise ValueError("package source is not a string")
    return source


def identity(package: dict[str, Any]) -> Identity:
    name = package.get("name")
    version = package.get("version")
    if not isinstance(name, str) or not isinstance(version, str):
        raise ValueError("package name/version must be strings")
    return (name, version, normalized_source(package))


def normalized_dependencies(package: dict[str, Any]) -> list[str]:
    dependencies = package.get("dependencies", [])
    if not isinstance(dependencies, list) or not all(
        isinstance(value, str) for value in dependencies
    ):
        raise ValueError("package dependencies must be an array of strings")
    return sorted(dependencies)


def normalized_checksum(package: dict[str, Any]) -> str | None:
    checksum = package.get("checksum")
    if checksum is not None and not isinstance(checksum, str):
        raise ValueError("package checksum must be a string when present")
    return checksum


def package_map(data: dict[str, Any]) -> dict[Identity, dict[str, Any]]:
    result: dict[Identity, dict[str, Any]] = {}
    for package in data.get("package", []):
        if not isinstance(package, dict):
            raise ValueError("package entry is not a TOML table")
        key = identity(package)
        if key in result:
            raise ValueError(f"duplicate package identity: {key!r}")
        result[key] = package
    return result


def identity_record(key: Identity, package: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": key[0],
        "version": key[1],
        "source": key[2],
        "checksum": normalized_checksum(package),
        "dependencies": normalized_dependencies(package),
    }


def name_identity_sets(packages: dict[Identity, dict[str, Any]]) -> dict[str, list[list[str]]]:
    by_name: dict[str, set[tuple[str, str]]] = {}
    for name, version, source in packages:
        by_name.setdefault(name, set()).add((version, source))
    return {
        name: [[version, source] for version, source in sorted(values)]
        for name, values in sorted(by_name.items())
    }


def analyze(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    before_packages = package_map(before)
    after_packages = package_map(after)
    before_keys = set(before_packages)
    after_keys = set(after_packages)

    added_keys = sorted(after_keys - before_keys)
    removed_keys = sorted(before_keys - after_keys)
    common_keys = sorted(before_keys & after_keys)

    checksum_changes: list[dict[str, Any]] = []
    dependency_changes: list[dict[str, Any]] = []
    for key in common_keys:
        before_package = before_packages[key]
        after_package = after_packages[key]
        before_checksum = normalized_checksum(before_package)
        after_checksum = normalized_checksum(after_package)
        if before_checksum != after_checksum:
            checksum_changes.append(
                {
                    "name": key[0],
                    "version": key[1],
                    "source": key[2],
                    "before": before_checksum,
                    "after": after_checksum,
                }
            )

        before_dependencies = normalized_dependencies(before_package)
        after_dependencies = normalized_dependencies(after_package)
        if before_dependencies != after_dependencies:
            dependency_changes.append(
                {
                    "name": key[0],
                    "version": key[1],
                    "source": key[2],
                    "before": before_dependencies,
                    "after": after_dependencies,
                    "added": sorted(set(after_dependencies) - set(before_dependencies)),
                    "removed": sorted(set(before_dependencies) - set(after_dependencies)),
                }
            )

    before_by_name = name_identity_sets(before_packages)
    after_by_name = name_identity_sets(after_packages)
    name_identity_changes: list[dict[str, Any]] = []
    for name in sorted(set(before_by_name) | set(after_by_name)):
        before_set = before_by_name.get(name, [])
        after_set = after_by_name.get(name, [])
        if before_set != after_set:
            name_identity_changes.append(
                {"name": name, "before": before_set, "after": after_set}
            )

    result = {
        "schema": "symtropy.cuf.cargo-lock-semantic-delta.v1",
        "lockfile_version_before": before.get("version"),
        "lockfile_version_after": after.get("version"),
        "package_count_before": len(before_packages),
        "package_count_after": len(after_packages),
        "added_packages": [identity_record(key, after_packages[key]) for key in added_keys],
        "removed_packages": [identity_record(key, before_packages[key]) for key in removed_keys],
        "name_identity_sets_changed": name_identity_changes,
        "checksum_changes": checksum_changes,
        "dependency_changes": dependency_changes,
    }
    result["summary"] = {
        "added_packages": len(result["added_packages"]),
        "removed_packages": len(result["removed_packages"]),
        "name_identity_sets_changed": len(name_identity_changes),
        "checksum_changes": len(checksum_changes),
        "dependency_changes": len(dependency_changes),
    }
    return result


def emit_text(result: dict[str, Any]) -> str:
    summary = result["summary"]
    lines = [
        f"schema={result['schema']}",
        f"lockfile_version={result['lockfile_version_before']}->{result['lockfile_version_after']}",
        f"package_count={result['package_count_before']}->{result['package_count_after']}",
        f"added_packages={summary['added_packages']}",
        f"removed_packages={summary['removed_packages']}",
        f"name_identity_sets_changed={summary['name_identity_sets_changed']}",
        f"checksum_changes={summary['checksum_changes']}",
        f"dependency_changes={summary['dependency_changes']}",
    ]

    for package in result["added_packages"]:
        lines.append(
            f"ADD\t{package['name']}\t{package['version']}\t{package['source']}"
        )
    for package in result["removed_packages"]:
        lines.append(
            f"REMOVE\t{package['name']}\t{package['version']}\t{package['source']}"
        )
    for change in result["name_identity_sets_changed"]:
        lines.append(
            "IDENTITY_SET\t{}\t{}\t{}".format(
                change["name"],
                json.dumps(change["before"], separators=(",", ":"), sort_keys=True),
                json.dumps(change["after"], separators=(",", ":"), sort_keys=True),
            )
        )
    for change in result["checksum_changes"]:
        lines.append(
            "CHECKSUM\t{}\t{}\t{}\t{}\t{}".format(
                change["name"],
                change["version"],
                change["source"],
                change["before"],
                change["after"],
            )
        )
    for change in result["dependency_changes"]:
        lines.append(
            "DEPENDENCIES\t{}\t{}\t{}\tadded={}\tremoved={}".format(
                change["name"],
                change["version"],
                change["source"],
                json.dumps(change["added"], separators=(",", ":")),
                json.dumps(change["removed"], separators=(",", ":")),
            )
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("--format", choices=("json", "text"), default="json")
    args = parser.parse_args()

    try:
        result = analyze(load_lock(args.before), load_lock(args.after))
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1

    if args.format == "json":
        json.dump(result, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    else:
        sys.stdout.write(emit_text(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
