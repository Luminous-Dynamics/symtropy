#!/usr/bin/env python3
"""Adapt a captured MultiWorldSimulator output into Humanity Atlas run v0.1.

This is a transport adapter, not a simulator or a scientific validator. It binds
an envelope to the exact source bytes, carries declared input/model identities,
and always leaves determinism unknown and classification unvalidated unless a
future, separately specified evidence workflow says otherwise.
"""
from __future__ import annotations

import argparse
import os
import hashlib
import json
import math
import re
import sys
import tempfile
from pathlib import Path
from typing import Any

RUN_SCHEMA_VERSION = "humanity-atlas.run.v0.1"
TREE_SCHEMA_VERSION = "mycelix-multiworld-sim.tech-tree.v1"
MANIFEST_SCHEMA_VERSION = "humanity-atlas.run-manifest.v1"
ADAPTER_ID = "mycelix-multiworld-sim-humanity-atlas-adapter"
SHA256_RE = re.compile(r"^[a-f0-9]{64}$")
MAX_SAFE_INTEGER = 9_007_199_254_740_991
MAX_SOURCE_ARTIFACT_BYTES = 64 * 1024 * 1024


def _check_source_artifact_size(size: int) -> None:
    if size < 0 or size > MAX_SOURCE_ARTIFACT_BYTES:
        raise AdapterError(
            f"source simulator artifact exceeds the {MAX_SOURCE_ARTIFACT_BYTES}-byte v0.1 limit"
        )


class JsonNumberToken:
    """Numeric lexeme preserved so hashes match serde_json serialization."""
    def __init__(self, token: str, is_integer: bool) -> None:
        self.token = token
        self.is_integer = is_integer

    def as_int(self) -> int:
        return int(self.token)

    def as_float(self) -> float:
        return float(self.token)


class AdapterError(ValueError):
    """Input does not meet the adapter's fail-closed contract."""


def _object_without_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AdapterError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def _reject_json_constant(token: str) -> None:
    raise AdapterError(f"non-standard JSON numeric token: {token}")


def _check_finite(value: Any, path: str = "$") -> None:
    if isinstance(value, JsonNumberToken) and not math.isfinite(value.as_float()):
        raise AdapterError(f"non-finite number at {path}")
    if isinstance(value, float) and not math.isfinite(value):
        raise AdapterError(f"non-finite number at {path}")
    if isinstance(value, dict):
        for key, child in value.items():
            _check_finite(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _check_finite(child, f"{path}[{index}]")


def parse_json_strict(raw: bytes) -> dict[str, Any]:
    _check_source_artifact_size(len(raw))
    try:
        text = raw.decode("utf-8", errors="strict")
        value = json.loads(
            text,
            object_pairs_hook=_object_without_duplicate_keys,
            parse_constant=_reject_json_constant,
            parse_int=lambda token: JsonNumberToken(token, True),
            parse_float=lambda token: JsonNumberToken(token, False),
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise AdapterError(f"invalid UTF-8 JSON input: {exc}") from exc
    _check_finite(value)
    if not isinstance(value, dict):
        raise AdapterError("simulator output must be a JSON object")
    return value


def canonical_json_bytes(value: Any) -> bytes:
    """Sorted-key compact JSON, preserving source numeric lexemes exactly.

    Simulator config/parameter digests are produced by serde_json after sorting
    object keys. The source artifact itself was serialized by the same numeric
    serializer; preserving those numeric lexemes avoids Python/Rust exponent or
    decimal-format differences while canonicalizing only object-key order.
    """
    def encode(item: Any) -> str:
        if isinstance(item, JsonNumberToken):
            return item.token
        if item is None:
            return "null"
        if item is True:
            return "true"
        if item is False:
            return "false"
        if isinstance(item, str):
            return json.dumps(item, ensure_ascii=False, allow_nan=False, separators=(",", ":"))
        if isinstance(item, int):
            return str(item)
        if isinstance(item, float):
            if not math.isfinite(item):
                raise AdapterError("cannot canonicalize a non-finite number")
            return json.dumps(item, allow_nan=False, separators=(",", ":"))
        if isinstance(item, list):
            return "[" + ",".join(encode(child) for child in item) + "]"
        if isinstance(item, dict):
            return "{" + ",".join(
                encode(key) + ":" + encode(item[key]) for key in sorted(item)
            ) + "}"
        raise AdapterError(f"unsupported JSON value: {type(item).__name__}")

    return encode(value).encode("utf-8")


def sha256_hex(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _required_object(parent: dict[str, Any], key: str, where: str) -> dict[str, Any]:
    value = parent.get(key)
    if not isinstance(value, dict):
        raise AdapterError(f"{where}.{key} must be an object")
    return value


def _required_string(parent: dict[str, Any], key: str, where: str) -> str:
    value = parent.get(key)
    if not isinstance(value, str) or not value.strip():
        raise AdapterError(f"{where}.{key} must be a non-empty string")
    return value


def _required_int(parent: dict[str, Any], key: str, where: str, *, minimum: int = 0) -> int:
    value = parent.get(key)
    if isinstance(value, JsonNumberToken):
        if not value.is_integer:
            raise AdapterError(f"{where}.{key} must be an integer >= {minimum}")
        value = value.as_int()
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise AdapterError(f"{where}.{key} must be an integer >= {minimum}")
    return value


def _required_finite_number(parent: dict[str, Any], key: str, where: str) -> float:
    value = parent.get(key)
    if isinstance(value, JsonNumberToken):
        value = value.as_float()
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(float(value)):
        raise AdapterError(f"{where}.{key} must be a finite number")
    return float(value)


def _required_sha256(parent: dict[str, Any], key: str, where: str) -> str:
    value = _required_string(parent, key, where)
    if not SHA256_RE.fullmatch(value):
        raise AdapterError(f"{where}.{key} must be 64 lowercase hexadecimal characters")
    return value


def read_source_artifact(path: Path) -> bytes:
    """Read at most the v0.1 size limit plus one byte, even if the file grows after stat."""
    input_size = path.stat().st_size
    _check_source_artifact_size(input_size)
    with path.open("rb") as stream:
        raw = stream.read(MAX_SOURCE_ARTIFACT_BYTES + 1)
    _check_source_artifact_size(len(raw))
    return raw


def adapt_source(raw: bytes, *, artifact_ref: str | None = None) -> dict[str, Any]:
    """Validate cross-field bindings and produce a conservative generic envelope."""
    source = parse_json_strict(raw)
    tree = _required_object(source, "technology_tree", "source")
    manifest = _required_object(source, "run_manifest", "source")
    resolved_config = _required_object(source, "resolved_config", "source")
    resolved_params = _required_object(source, "resolved_params", "source")

    if tree.get("schema_version") != TREE_SCHEMA_VERSION:
        raise AdapterError("unsupported technology_tree.schema_version")
    if tree.get("classification") != "simulated_scenario":
        raise AdapterError("technology_tree must remain classified as simulated_scenario")
    if tree.get("year_zero_semantics") != "relative_to_scenario_start":
        raise AdapterError("unsupported technology_tree.year_zero_semantics")
    source_model = _required_string(tree, "source_model", "technology_tree")
    model_version_prefix = "mycelix-multiworld-sim/"
    if not source_model.startswith(model_version_prefix):
        raise AdapterError("source_model must identify the versioned mycelix-multiworld-sim engine")
    engine_version = source_model[len(model_version_prefix):]
    if (
        not engine_version
        or len(engine_version) > 120
        or engine_version.strip() != engine_version
        or "/" in engine_version
        or any(character.isspace() for character in engine_version)
    ):
        raise AdapterError("source_model engine version must be a non-empty version token of at most 120 characters")
    tree_source = _required_string(tree, "definition_source", "technology_tree")
    if len(tree_source) > 200:
        raise AdapterError("technology_tree.definition_source exceeds 200 characters")
    model_sha = _required_sha256(tree, "definition_sha256", "technology_tree")
    ticks_per_year = _required_int(tree, "ticks_per_year", "technology_tree", minimum=1)
    if ticks_per_year > 4_294_967_295:
        raise AdapterError("technology_tree.ticks_per_year exceeds the u32 wire range")
    milestones = tree.get("milestones")
    if not isinstance(milestones, list) or not milestones:
        raise AdapterError("technology_tree.milestones must be a non-empty array")

    if manifest.get("schema_version") != MANIFEST_SCHEMA_VERSION:
        raise AdapterError("unsupported run_manifest.schema_version")
    manifest_source = _required_string(manifest, "selected_definition_source", "run_manifest")
    manifest_model_sha = _required_sha256(manifest, "effective_model_sha256", "run_manifest")
    config_sha = _required_sha256(manifest, "resolved_config_sha256", "run_manifest")
    params_sha = _required_sha256(manifest, "resolved_params_sha256", "run_manifest")
    seed = _required_int(source, "seed", "source")
    years = _required_int(source, "years", "source", minimum=1)
    ticks = _required_int(source, "ticks", "source", minimum=1)
    manifest_seed = _required_int(manifest, "seed", "run_manifest")
    manifest_ticks = _required_int(manifest, "ticks", "run_manifest")
    if ticks > 4_294_967_295 or manifest_ticks > 4_294_967_295:
        raise AdapterError("source or run_manifest ticks exceed the u32 wire range")
    if "source_revision" not in manifest:
        raise AdapterError("run_manifest.source_revision is a required key and may be null")
    source_revision = manifest["source_revision"]
    if source_revision is not None and (
        not isinstance(source_revision, str) or not source_revision.strip() or len(source_revision) > 120
    ):
        raise AdapterError("run_manifest.source_revision must be a non-empty string of at most 120 characters or null")
    if len(manifest_source) > 200:
        raise AdapterError("run_manifest.selected_definition_source exceeds 200 characters")
    if tree_source != manifest_source:
        raise AdapterError("technology-tree definition_source disagrees with run manifest")
    if model_sha != manifest_model_sha:
        raise AdapterError("technology-tree definition digest disagrees with run manifest")
    config_seed = _required_int(resolved_config, "seed", "resolved_config")
    config_ticks = _required_int(resolved_config, "total_ticks", "resolved_config", minimum=1)
    if config_seed != seed:
        raise AdapterError("resolved_config.seed disagrees with top-level seed")
    if config_ticks != ticks:
        raise AdapterError("resolved_config.total_ticks disagrees with top-level ticks")
    if seed != manifest_seed or ticks != manifest_ticks:
        raise AdapterError("top-level seed/ticks disagree with run manifest")
    if ticks != years * ticks_per_year:
        raise AdapterError("top-level ticks do not equal years * ticks_per_year")
    if seed > MAX_SAFE_INTEGER or years > 100_000:
        raise AdapterError("seed or horizon exceeds Humanity Atlas v0.1 numeric range (year <= 100000)")

    # The simulator computes these digests over recursively key-sorted compact
    # JSON. Reject a mismatch; never substitute a freshly invented identity.
    if sha256_hex(canonical_json_bytes(resolved_config)) != config_sha:
        raise AdapterError("resolved_config does not match run_manifest digest")
    if sha256_hex(canonical_json_bytes(resolved_params)) != params_sha:
        raise AdapterError("resolved_params does not match run_manifest digest")

    final_cvs = _required_finite_number(source, "final_cvs", "source")
    final_population = _required_int(source, "final_population", "source")
    tech_milestones = _required_int(source, "tech_milestones", "source")
    if final_population > MAX_SAFE_INTEGER or tech_milestones > MAX_SAFE_INTEGER:
        raise AdapterError("population or milestone count exceeds the exact numeric range of the run envelope")
    survived = source.get("survived")
    if not isinstance(survived, bool):
        raise AdapterError("source.survived must be a boolean")

    scenario_descriptor = {
        "schema_version": "humanity-atlas.effective-scenario.v1",
        "source_model": source_model,
        "selected_definition_source": manifest_source,
        "effective_model_sha256": model_sha,
        "resolved_config_sha256": config_sha,
        "resolved_params_sha256": params_sha,
        "ticks_per_year": ticks_per_year,
        "ticks": ticks,
    }
    scenario_sha = sha256_hex(canonical_json_bytes(scenario_descriptor))
    source_sha = sha256_hex(raw)
    if artifact_ref is not None and (not artifact_ref.strip() or len(artifact_ref) > 2000):
        raise AdapterError("artifact_ref must be non-empty and at most 2000 characters")
    artifact_locator = artifact_ref or f"urn:sha256:{source_sha}"

    warnings = [
        "classification_unvalidated_simulation: no empirical calibration or validation is asserted",
        "determinism_unverified: a seed is recorded, but exact replay has not been independently demonstrated by this adapter",
        "dataset_identity_unestablished: the source manifest does not declare a separately identified dataset",
        "model_sha256_scope: the model digest identifies the technology-tree definition, not the complete executable or source tree",
        "sha256_proves_byte_integrity_only: this envelope does not authenticate a publisher or establish scientific truth",
    ]
    if not survived:
        warnings.append(
            "scenario_survival_failure: the source simulator reports that civilization did not survive the scenario horizon"
        )
    if source_revision is None:
        warnings.append("source_revision_missing: package version is known, but the exact simulator build commit is not identified")

    return {
        "schema_version": RUN_SCHEMA_VERSION,
        "run_id": f"urn:sha256:{source_sha}",
        "scenario_id": f"mycelix-multiworld-sim:scenario:{scenario_sha}",
        "inputs": {
            "scenario_sha256": scenario_sha,
            "scenario_identity_kind": "canonical_effective_scenario",
            "selected_definition_source": manifest_source,
            "dataset_sha256": None,
            "resolved_config_sha256": config_sha,
            "resolved_params_sha256": params_sha,
        },
        "engine": {
            "adapter": ADAPTER_ID,
            "engine_version": engine_version,
            "model_sha256": model_sha,
            "model_identity_kind": "technology_tree_definition",
            "definition_source": tree_source,
            "source_revision": source_revision,
        },
        "run_status": "completed",
        "output_classification": "unvalidated_simulation",
        "deterministic": None,
        "seed": seed,
        "replicates": 1,
        "outputs": [
            {"metric_id": "final_cvs", "year": years, "time_basis": "scenario_relative_year", "unit": "CVS", "statistic": "value", "value": final_cvs, "sample_count": 1},
            {"metric_id": "final_population", "year": years, "time_basis": "scenario_relative_year", "unit": "persons", "statistic": "value", "value": float(final_population), "sample_count": 1},
            {"metric_id": "tech_milestones_achieved", "year": years, "time_basis": "scenario_relative_year", "unit": "milestones", "statistic": "value", "value": float(tech_milestones), "sample_count": 1},
            {"metric_id": "civilization_survived", "year": years, "time_basis": "scenario_relative_year", "unit": "boolean indicator (0=false, 1=true)", "statistic": "value", "value": 1.0 if survived else 0.0, "sample_count": 1},
        ],
        "validation": {
            "status": "not_run",
            "notes": ["Structural adaptation only; no calibration, empirical validation, or prospective forecast receipt was inferred."],
            "evidence": [],
        },
        "warnings": warnings,
        "artifacts": [
            {
                "artifact_id": "source-simulator-output",
                "role": "source_simulation_output",
                "media_type": "application/json",
                "artifact_ref": artifact_locator,
                "artifact_sha256": source_sha,
                "json_pointer": "/technology_tree",
            }
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="captured humanity-atlas-json source artifact")
    parser.add_argument("--artifact-ref", help="retrievable artifact URI; defaults to urn:sha256:<exact-input-digest>")
    parser.add_argument("--output", type=Path, help="write envelope to this path; default is stdout")
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="atomically replace an existing output path (never the captured source artifact)",
    )
    args = parser.parse_args(argv)
    try:
        if args.output:
            # Protect captured source bytes through literal, resolved-path, symlink,
            # and existing hard-link aliases. This protection cannot be overridden.
            try:
                input_resolved = args.input.resolve()
                output_resolved = args.output.resolve()
            except (OSError, RuntimeError) as exc:
                raise AdapterError(f"cannot safely resolve input/output paths: {exc}") from exc
            aliases_input = input_resolved == output_resolved
            if not aliases_input and args.output.exists():
                try:
                    aliases_input = args.input.samefile(args.output)
                except OSError as exc:
                    raise AdapterError(f"cannot safely compare input/output file identity: {exc}") from exc
            if aliases_input:
                raise AdapterError(
                    "output path aliases the input source artifact; refusing to overwrite captured source bytes"
                )
            if (args.output.exists() or args.output.is_symlink()) and not args.overwrite:
                raise AdapterError(
                    "output path already exists; refusing to replace it without --overwrite"
                )
        raw = read_source_artifact(args.input)
        envelope = adapt_source(raw, artifact_ref=args.artifact_ref)
        rendered = json.dumps(envelope, ensure_ascii=False, allow_nan=False, indent=2) + "\n"
        if args.output:
            # Stage in the destination directory. Without --overwrite, a hard link
            # installs the completed temporary file atomically only if the path is
            # still absent; with --overwrite, os.replace swaps the directory entry
            # rather than following a destination symlink or mutating other hard links.
            temporary_path: Path | None = None
            try:
                with tempfile.NamedTemporaryFile(
                    mode="w",
                    encoding="utf-8",
                    dir=args.output.parent,
                    prefix=f".{args.output.name}.",
                    suffix=".tmp",
                    delete=False,
                ) as stream:
                    temporary_path = Path(stream.name)
                    stream.write(rendered)
                    stream.flush()
                    os.fsync(stream.fileno())
                if args.overwrite:
                    os.replace(temporary_path, args.output)
                    temporary_path = None
                else:
                    os.link(temporary_path, args.output)
                    temporary_path.unlink()
                    temporary_path = None
            finally:
                if temporary_path is not None:
                    try:
                        temporary_path.unlink()
                    except FileNotFoundError:
                        pass
        else:
            sys.stdout.write(rendered)
    except (OSError, AdapterError, RecursionError) as exc:
        print(f"adapter rejected input: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
