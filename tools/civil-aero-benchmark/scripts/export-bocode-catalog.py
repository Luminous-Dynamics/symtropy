#!/usr/bin/env python3
"""Export BOCoDe package metadata to the normalized Rust catalog interchange.

Run only inside the separately pinned BOCoDe environment. This script reads metadata resources;
it does not instantiate or evaluate optimization problems and does not download task datasets.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.metadata
import importlib.resources
import json
from pathlib import Path
from typing import Any

HEADER = (
    "problem_id", "application", "problem_class", "dimension", "num_objectives",
    "num_constraints", "variable_type_class", "benchmark_set_member", "bounds_sha256",
    "variable_types_sha256", "source_reference_state", "source_reference",
    "metadata_sha256", "required_extra",
)


def append_u64(buffer: bytearray, value: int) -> None:
    if value < 0 or value >= 1 << 64:
        raise ValueError("catalog canonical length outside u64")
    buffer.extend(value.to_bytes(8, "little"))


def append_bytes(buffer: bytearray, value: bytes) -> None:
    append_u64(buffer, len(value))
    buffer.extend(value)


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def application_name(value: Any) -> str:
    normalized = str(value or "").strip().lower()
    if normalized == "engineering":
        return "engineering"
    if normalized in ("hpo", "hyperparameter optimization", "hyperparameter-optimization"):
        return "hpo"
    if normalized == "synthetic":
        return "synthetic"
    return "extended"


def class_name(objectives: int, constraints: int, variable_type_class: str) -> str:
    mixed = variable_type_class == "mixed"
    if mixed:
        if objectives == 1 and constraints == 0:
            return "so_mixed_variable"
        if objectives == 1 and constraints > 0:
            return "so_mixed_variable_constrained_extension"
        if objectives > 1 and constraints == 0:
            return "mo_mixed_variable_extension"
        if objectives > 1 and constraints > 0:
            return "mo_mixed_variable_constrained_extension"
    else:
        if objectives == 1 and constraints == 0:
            return "so_unconstrained"
        if objectives == 1 and constraints > 0:
            return "so_constrained"
        if objectives > 1 and constraints == 0:
            return "mo_unconstrained"
        if objectives > 1 and constraints > 0:
            return "mo_constrained"
    raise ValueError("cannot classify zero-objective metadata")


def read_benchmark_ids(path: Path | None) -> set[str]:
    if path is None:
        return set()
    ids = {
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    }
    if len(ids) != sum(
        bool(line.strip()) and not line.lstrip().startswith("#")
        for line in path.read_text(encoding="utf-8").splitlines()
    ):
        raise ValueError("benchmark-set task IDs contain duplicates")
    return ids


def export(args: argparse.Namespace) -> None:
    try:
        import bocode
    except ImportError as error:
        raise SystemExit(
            "BOCoDe is not importable in this environment. Install the separately pinned "
            "BOCoDe package; do not add it to Symtropy's production Rust dependencies."
        ) from error

    try:
        version = importlib.metadata.version("bocode")
    except importlib.metadata.PackageNotFoundError as error:
        raise SystemExit("cannot determine installed BOCoDe package version") from error

    records = bocode.list_metadata()
    if not isinstance(records, list) or not records:
        raise ValueError("bocode.list_metadata() returned no task metadata")
    by_name: dict[str, dict[str, Any]] = {}
    raw_by_name: dict[str, bytes] = {}
    resource_root = importlib.resources.files("bocode.opt_problems_metadata")
    for metadata in records:
        if not isinstance(metadata, dict):
            raise ValueError("BOCoDe metadata entries must be JSON objects")
        name = metadata.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError("BOCoDe metadata entry is missing a non-empty name")
        if name in by_name:
            raise ValueError(f"duplicate BOCoDe metadata name: {name}")
        raw = resource_root.joinpath(f"{name}.json").read_bytes()
        decoded = json.loads(raw.decode("utf-8"))
        if decoded != metadata:
            raise ValueError(
                f"{name}: list_metadata() does not match the installed raw metadata resource"
            )
        by_name[name] = metadata
        raw_by_name[name] = raw

    benchmark_ids = read_benchmark_ids(args.benchmark_set_ids)
    unknown_ids = benchmark_ids - set(by_name)
    if unknown_ids:
        raise ValueError(
            "benchmark-set ID list contains names absent from this exact package catalog: "
            + ", ".join(sorted(unknown_ids))
        )

    rows: list[list[str]] = []
    catalog_canonical = bytearray()
    for name in sorted(by_name):
        metadata = by_name[name]
        raw = raw_by_name[name]
        dimension = metadata.get("dim")
        objectives = metadata.get("num_objectives")
        constraints = metadata.get("num_constraints", 0)
        input_type = metadata.get("input_type")
        if not isinstance(dimension, int) or isinstance(dimension, bool) or dimension <= 0:
            raise ValueError(f"{name}: dim must be a positive integer")
        if not isinstance(objectives, int) or isinstance(objectives, bool) or objectives <= 0:
            raise ValueError(f"{name}: num_objectives must be a positive integer")
        if not isinstance(constraints, int) or isinstance(constraints, bool) or constraints < 0:
            raise ValueError(f"{name}: num_constraints must be a non-negative integer")
        if input_type not in ("continuous", "discrete", "mixed"):
            raise ValueError(f"{name}: unsupported input_type {input_type!r}")
        variable_type_class = "continuous" if input_type == "continuous" else "mixed"

        variable_types = metadata.get("variable_types")
        if not isinstance(variable_types, list) or len(variable_types) != dimension:
            raise ValueError(
                f"{name}: variable_types must have exactly dim={dimension} entries"
            )
        bounds = metadata.get("bounds")
        if bounds is not None:
            if not isinstance(bounds, list) or len(bounds) != dimension:
                raise ValueError(f"{name}: non-null bounds must have exactly dim entries")
        bounds_sha256 = sha256(canonical_json(bounds))
        variable_types_sha256 = sha256(canonical_json(variable_types))
        metadata_sha256 = sha256(raw)

        source = metadata.get("source")
        source_reference = source.strip() if isinstance(source, str) else ""
        source_state = "present" if source_reference else "missing"
        extra = metadata.get("extra")
        required_extra = str(extra).strip() if extra else "none"

        rows.append([
            name,
            application_name(metadata.get("application")),
            class_name(objectives, constraints, variable_type_class),
            str(dimension),
            str(objectives),
            str(constraints),
            variable_type_class,
            "true" if name in benchmark_ids else "false",
            bounds_sha256,
            variable_types_sha256,
            source_state,
            source_reference,
            metadata_sha256,
            required_extra,
        ])
        append_bytes(catalog_canonical, name.encode("utf-8"))
        append_bytes(catalog_canonical, raw)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.writer(stream, quoting=csv.QUOTE_ALL, lineterminator="\n")
        writer.writerow(HEADER)
        writer.writerows(rows)

    package_commit = args.source_commit.lower()
    manifest = {
        "schema": "bocode-raw-catalog-manifest/0.1.0",
        "source_repository": "https://github.com/rosenyu304/BOCoDe",
        "source_commit": package_commit,
        "package_version": version,
        "source_catalog_sha256": sha256(bytes(catalog_canonical)),
        "raw_metadata_record_count": len(rows),
        "benchmark_set_member_count": len(benchmark_ids),
        "benchmark_set_ids_sha256": (
            sha256(("\n".join(sorted(benchmark_ids)) + "\n").encode("utf-8"))
            if benchmark_ids else None
        ),
        "normalized_csv_sha256": sha256(args.output.read_bytes()),
        "objective_direction": "maximize",
        "constraint_convention": "g(x)<=0_is_feasible",
        "note": (
            "list_metadata() registry tasks only; list_synthetic() functions are not included "
            "unless BOCoDe adds per-task metadata resources for them. An empty benchmark-set "
            "ID list marks every task false; it does not infer membership from names."
        ),
    }
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(
        json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(manifest, sort_keys=True, indent=2))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="normalized catalog CSV")
    parser.add_argument("--manifest", type=Path, required=True, help="raw-catalog context JSON")
    parser.add_argument(
        "--source-commit",
        required=True,
        help="exact 40-character BOCoDe Git commit used for this installed package",
    )
    parser.add_argument(
        "--benchmark-set-ids",
        type=Path,
        help="reviewed one-problem-ID-per-line list for the benchmark paper's frozen set",
    )
    args = parser.parse_args()
    if len(args.source_commit) != 40 or any(
        char not in "0123456789abcdefABCDEF" for char in args.source_commit
    ):
        parser.error("--source-commit must be a full 40-character hexadecimal commit")
    export(args)


if __name__ == "__main__":
    main()
