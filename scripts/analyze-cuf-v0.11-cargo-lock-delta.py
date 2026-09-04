#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Deterministically summarize semantic differences between two Cargo.lock files.

The v2 schema is intentionally structurally complete: known Cargo package fields
receive focused semantic reporting, while every other top-level/package field is
reported explicitly rather than silently ignored. Policy remains descriptive
until real Stage A/Stage B resolver output has been reviewed.
"""

from __future__ import annotations

import argparse
import datetime as datetime_module
import json
import math
import sys
import tomllib
from pathlib import Path
from typing import Any


Identity = tuple[str, str, str]
SOURCE_OMITTED = "<source-omitted>"
KNOWN_PACKAGE_FIELDS = frozenset({"name", "version", "source", "checksum", "dependencies"})
MODELED_TOP_LEVEL_FIELDS = frozenset({"version", "package"})


def load_lock(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: Cargo.lock root is not a TOML table")
    if "version" in data:
        version = data["version"]
        if type(version) is not int or version <= 0:
            raise ValueError(f"{path}: Cargo.lock version must be a positive integer")
    packages = data.get("package", [])
    if not isinstance(packages, list):
        raise ValueError(f"{path}: package is not an array of tables")
    return data


def canonical_value(value: Any) -> Any:
    """Convert arbitrary TOML values into deterministic JSON-safe values."""
    if isinstance(value, dict):
        return {key: canonical_value(value[key]) for key in sorted(value)}
    if isinstance(value, list):
        return [canonical_value(item) for item in value]
    if isinstance(value, (datetime_module.datetime, datetime_module.date, datetime_module.time)):
        return {"__toml_type__": type(value).__name__, "value": value.isoformat()}
    if isinstance(value, float):
        if math.isnan(value):
            return {"__toml_type__": "float", "value": "nan"}
        if math.isinf(value):
            return {"__toml_type__": "float", "value": "inf" if value > 0 else "-inf"}
        return value
    if isinstance(value, (str, int, bool)) or value is None:
        return value
    raise ValueError(f"unsupported TOML value type: {type(value).__name__}")


def normalized_source(package: dict[str, Any]) -> str:
    source = package.get("source")
    if source is None:
        # Cargo omits source for path/local packages. Source omission alone does
        # not prove workspace membership, so keep the evidence label neutral.
        return SOURCE_OMITTED
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


def extra_package_fields(package: dict[str, Any]) -> dict[str, Any]:
    return {
        key: canonical_value(package[key])
        for key in sorted(package)
        if key not in KNOWN_PACKAGE_FIELDS
    }


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
        "extra_fields": extra_package_fields(package),
    }


def name_identity_sets(packages: dict[Identity, dict[str, Any]]) -> dict[str, list[list[str]]]:
    by_name: dict[str, set[tuple[str, str]]] = {}
    for name, version, source in packages:
        by_name.setdefault(name, set()).add((version, source))
    return {
        name: [[version, source] for version, source in sorted(values)]
        for name, values in sorted(by_name.items())
    }


def unmodeled_top_level(data: dict[str, Any]) -> dict[str, Any]:
    return {
        key: canonical_value(data[key])
        for key in sorted(data)
        if key not in MODELED_TOP_LEVEL_FIELDS
    }


def presence_record(values: dict[str, Any], key: str) -> dict[str, Any]:
    if key not in values:
        return {"present": False}
    return {"present": True, "value": values[key]}


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
    unmodeled_package_field_changes: list[dict[str, Any]] = []
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

        before_extra = extra_package_fields(before_package)
        after_extra = extra_package_fields(after_package)
        if before_extra != after_extra:
            unmodeled_package_field_changes.append(
                {
                    "name": key[0],
                    "version": key[1],
                    "source": key[2],
                    "before": before_extra,
                    "after": after_extra,
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

    before_top = unmodeled_top_level(before)
    after_top = unmodeled_top_level(after)
    unmodeled_top_level_changes: list[dict[str, Any]] = []
    for field in sorted(set(before_top) | set(after_top)):
        before_value = presence_record(before_top, field)
        after_value = presence_record(after_top, field)
        if before_value != after_value:
            unmodeled_top_level_changes.append(
                {"field": field, "before": before_value, "after": after_value}
            )

    result = {
        "schema": "symtropy.cuf.cargo-lock-semantic-delta.v2",
        "lockfile_version_before": before.get("version"),
        "lockfile_version_after": after.get("version"),
        "package_count_before": len(before_packages),
        "package_count_after": len(after_packages),
        "added_packages": [identity_record(key, after_packages[key]) for key in added_keys],
        "removed_packages": [identity_record(key, before_packages[key]) for key in removed_keys],
        "name_identity_sets_changed": name_identity_changes,
        "checksum_changes": checksum_changes,
        "dependency_changes": dependency_changes,
        "unmodeled_top_level_changes": unmodeled_top_level_changes,
        "unmodeled_package_field_changes": unmodeled_package_field_changes,
    }
    result["summary"] = {
        "lockfile_version_changed": int(
            result["lockfile_version_before"] != result["lockfile_version_after"]
        ),
        "added_packages": len(result["added_packages"]),
        "removed_packages": len(result["removed_packages"]),
        "name_identity_sets_changed": len(name_identity_changes),
        "checksum_changes": len(checksum_changes),
        "dependency_changes": len(dependency_changes),
        "unmodeled_top_level_changes": len(unmodeled_top_level_changes),
        "unmodeled_package_field_changes": len(unmodeled_package_field_changes),
    }
    return result


def compact_json(value: Any) -> str:
    return json.dumps(value, separators=(",", ":"), sort_keys=True, allow_nan=False)


def emit_text(result: dict[str, Any]) -> str:
    summary = result["summary"]
    lines = [
        f"schema={result['schema']}",
        f"lockfile_version={result['lockfile_version_before']}->{result['lockfile_version_after']}",
        f"package_count={result['package_count_before']}->{result['package_count_after']}",
        f"lockfile_version_changed={summary['lockfile_version_changed']}",
        f"added_packages={summary['added_packages']}",
        f"removed_packages={summary['removed_packages']}",
        f"name_identity_sets_changed={summary['name_identity_sets_changed']}",
        f"checksum_changes={summary['checksum_changes']}",
        f"dependency_changes={summary['dependency_changes']}",
        f"unmodeled_top_level_changes={summary['unmodeled_top_level_changes']}",
        f"unmodeled_package_field_changes={summary['unmodeled_package_field_changes']}",
    ]

    for package in result["added_packages"]:
        lines.append(
            "ADD\t{}\t{}\t{}\tchecksum={}\tdependencies={}\textra_fields={}".format(
                package["name"],
                package["version"],
                package["source"],
                compact_json(package["checksum"]),
                compact_json(package["dependencies"]),
                compact_json(package["extra_fields"]),
            )
        )
    for package in result["removed_packages"]:
        lines.append(
            "REMOVE\t{}\t{}\t{}\tchecksum={}\tdependencies={}\textra_fields={}".format(
                package["name"],
                package["version"],
                package["source"],
                compact_json(package["checksum"]),
                compact_json(package["dependencies"]),
                compact_json(package["extra_fields"]),
            )
        )
    for change in result["name_identity_sets_changed"]:
        lines.append(
            "IDENTITY_SET\t{}\t{}\t{}".format(
                change["name"], compact_json(change["before"]), compact_json(change["after"])
            )
        )
    for change in result["checksum_changes"]:
        lines.append(
            "CHECKSUM\t{}\t{}\t{}\tbefore={}\tafter={}".format(
                change["name"],
                change["version"],
                change["source"],
                compact_json(change["before"]),
                compact_json(change["after"]),
            )
        )
    for change in result["dependency_changes"]:
        lines.append(
            "DEPENDENCIES\t{}\t{}\t{}\tadded={}\tremoved={}".format(
                change["name"], change["version"], change["source"],
                compact_json(change["added"]), compact_json(change["removed"]),
            )
        )
    for change in result["unmodeled_top_level_changes"]:
        lines.append(
            "TOP_LEVEL\t{}\t{}\t{}".format(
                change["field"], compact_json(change["before"]), compact_json(change["after"])
            )
        )
    for change in result["unmodeled_package_field_changes"]:
        lines.append(
            "PACKAGE_FIELDS\t{}\t{}\t{}\t{}\t{}".format(
                change["name"], change["version"], change["source"],
                compact_json(change["before"]), compact_json(change["after"]),
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
        if args.format == "json":
            output = json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + "\n"
        else:
            output = emit_text(result)
    except (OSError, tomllib.TOMLDecodeError, ValueError, TypeError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1

    sys.stdout.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
