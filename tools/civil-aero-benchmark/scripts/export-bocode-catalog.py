#!/usr/bin/env python3
"""Export BOCoDe metadata resources to the normalized Rust catalog interchange.

Run in the separately pinned BOCoDe environment. This exports metadata only: it never
instantiates/evaluates optimization problems and does not download optional task datasets.
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
    "num_constraints", "variable_type_class", "benchmark_set_member", "bounds_state",
    "bounds_sha256", "variable_types_state", "variable_types_sha256",
    "source_reference_state", "source_reference", "metadata_sha256", "required_extra",
)


def append_u64(buffer: bytearray, value: int) -> None:
    if value < 0 or value >= 1 << 64:
        raise ValueError("canonical value length outside u64")
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


def problem_class(objectives: int, constraints: int, variable_class: str) -> str:
    mixed = variable_class == "mixed"
    if mixed and objectives == 1 and constraints == 0:
        return "so_mixed_variable"
    if mixed and objectives == 1 and constraints > 0:
        return "so_mixed_variable_constrained_extension"
    if mixed and objectives > 1 and constraints == 0:
        return "mo_mixed_variable_extension"
    if mixed and objectives > 1 and constraints > 0:
        return "mo_mixed_variable_constrained_extension"
    if not mixed and objectives == 1 and constraints == 0:
        return "so_unconstrained"
    if not mixed and objectives == 1 and constraints > 0:
        return "so_constrained"
    if not mixed and objectives > 1 and constraints == 0:
        return "mo_unconstrained"
    if not mixed and objectives > 1 and constraints > 0:
        return "mo_constrained"
    raise ValueError("BOCoDe metadata contains a zero-objective task")


def read_benchmark_ids(path: Path | None) -> set[str]:
    if path is None:
        return set()
    lines = [
        line.strip() for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    ids = set(lines)
    if len(ids) != len(lines):
        raise ValueError("benchmark-set task ID list contains duplicates")
    return ids


def export(args: argparse.Namespace) -> None:
    try:
        import bocode
    except ImportError as error:
        raise SystemExit(
            "BOCoDe is not importable. Install it in a separate benchmark environment; "
            "do not add it to Symtropy's Rust production dependencies."
        ) from error

    version = importlib.metadata.version("bocode")
    metadata_rows = bocode.list_metadata()
    if not isinstance(metadata_rows, list) or not metadata_rows:
        raise ValueError("bocode.list_metadata() returned no metadata")
    package_resources = importlib.resources.files("bocode.opt_problems_metadata")
    rows_by_name: dict[str, dict[str, Any]] = {}
    raw_by_name: dict[str, bytes] = {}
    for metadata in metadata_rows:
        if not isinstance(metadata, dict):
            raise ValueError("BOCoDe metadata entries must be JSON objects")
        name = metadata.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError("BOCoDe metadata object is missing its name")
        if name in rows_by_name:
            raise ValueError(f"duplicate BOCoDe metadata task: {name}")
        raw = package_resources.joinpath(f"{name}.json").read_bytes()
        if json.loads(raw.decode("utf-8")) != metadata:
            raise ValueError(f"{name}: API metadata differs from the installed raw JSON resource")
        rows_by_name[name] = metadata
        raw_by_name[name] = raw

    benchmark_ids = read_benchmark_ids(args.benchmark_set_ids)
    unknown = benchmark_ids - set(rows_by_name)
    if unknown:
        raise ValueError(
            "benchmark-set IDs not present in this exact package catalog: " + ", ".join(sorted(unknown))
        )

    catalog_bytes = bytearray()
    output_rows: list[list[str]] = []
    missing_sources = unavailable_bounds = unavailable_variable_types = 0
    for name in sorted(rows_by_name):
        metadata = rows_by_name[name]
        raw = raw_by_name[name]
        dimension = metadata.get("dim")
        objectives = metadata.get("num_objectives")
        constraints = metadata.get("num_constraints", 0)
        input_type = metadata.get("input_type")
        for field, value, allow_zero in (
            ("dim", dimension, False),
            ("num_objectives", objectives, False),
            ("num_constraints", constraints, True),
        ):
            if not isinstance(value, int) or isinstance(value, bool) or value < (0 if allow_zero else 1):
                raise ValueError(f"{name}: {field} has an invalid integer value")
        if input_type not in ("continuous", "discrete", "mixed"):
            raise ValueError(f"{name}: unsupported input_type {input_type!r}")
        variable_class = "continuous" if input_type == "continuous" else "mixed"
        variable_types = metadata.get("variable_types")
        if variable_types is None:
            variable_types_state = "unavailable"
            unavailable_variable_types += 1
        elif isinstance(variable_types, list) and len(variable_types) == dimension:
            variable_types_state = "present"
        else:
            raise ValueError(f"{name}: variable_types must be null or have dim entries")
        bounds = metadata.get("bounds")
        if bounds is None:
            bounds_state = "unavailable"
            unavailable_bounds += 1
        elif isinstance(bounds, list) and len(bounds) == dimension:
            bounds_state = "present"
        else:
            raise ValueError(f"{name}: bounds must be null or have dim entries")

        source = metadata.get("source")
        source_ref = source.strip() if isinstance(source, str) else ""
        source_state = "present" if source_ref else "missing"
        missing_sources += source_state == "missing"
        extra = metadata.get("extra")
        required_extra = str(extra).strip() if extra else "none"

        output_rows.append([
            name,
            application_name(metadata.get("application")),
            problem_class(objectives, constraints, variable_class),
            str(dimension),
            str(objectives),
            str(constraints),
            variable_class,
            "true" if name in benchmark_ids else "false",
            bounds_state,
            sha256(canonical_json(bounds)),
            variable_types_state,
            sha256(canonical_json(variable_types)),
            source_state,
            source_ref,
            sha256(raw),
            required_extra,
        ])
        append_bytes(catalog_bytes, name.encode("utf-8"))
        append_bytes(catalog_bytes, raw)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.writer(stream, quoting=csv.QUOTE_ALL, lineterminator="\n")
        writer.writerow(HEADER)
        writer.writerows(output_rows)

    package_manifest = {
        "schema": "bocode-raw-catalog-manifest/0.1.0",
        "source_repository": "https://github.com/rosenyu304/BOCoDe",
        "source_commit": args.source_commit.lower(),
        "package_version": version,
        "source_catalog_sha256": sha256(bytes(catalog_bytes)),
        "raw_metadata_record_count": len(output_rows),
        "benchmark_set_member_count": len(benchmark_ids),
        "benchmark_set_ids_sha256": (
            sha256(("\n".join(sorted(benchmark_ids)) + "\n").encode("utf-8"))
            if benchmark_ids else None
        ),
        "normalized_csv_sha256": sha256(args.output.read_bytes()),
        "missing_source_reference_count": missing_sources,
        "unavailable_bounds_count": unavailable_bounds,
        "unavailable_variable_types_count": unavailable_variable_types,
        "objective_direction": "maximize",
        "constraint_convention": "g(x)<=0_is_feasible",
        "catalog_scope": (
            "bocode.list_metadata() registered task metadata only; synthetic functions from "
            "list_synthetic() are not included unless they gain first-class metadata resources"
        ),
        "benchmark_set_membership": (
            "explicitly supplied task ID list only; names and application categories are not "
            "used to infer membership"
        ),
    }
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(
        json.dumps(package_manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(package_manifest, sort_keys=True, indent=2))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument(
        "--source-commit", required=True,
        help="exact 40-character Git commit for the installed BOCoDe source",
    )
    parser.add_argument(
        "--benchmark-set-ids", type=Path,
        help="reviewed one-ID-per-line membership list; unspecified IDs default false",
    )
    args = parser.parse_args()
    if len(args.source_commit) != 40 or any(
        char not in "0123456789abcdefABCDEF" for char in args.source_commit
    ):
        parser.error("--source-commit must be a full 40-character hexadecimal commit")
    export(args)


if __name__ == "__main__":
    main()
