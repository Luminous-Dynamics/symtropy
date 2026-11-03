#!/usr/bin/env python3
"""Independent stdlib verifier for DPW-8 OAT15A lane-separated protocol manifests."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from pathlib import Path

SCHEMA = "dpw8-oat15a-protocol-manifest/0.1.0"
EVIDENCE = (
    "canonical_protocol_spec_and_source_admission_state_not_solver_execution_"
    "or_physical_validation"
)
HEADER = (
    "record_type,schema,protocol_revision,geometry_designation,geometry_ip_holder,"
    "geometry_acknowledgement_required_each_share,geometry_share_condition,evidence_level,"
    "lane,flow_mode,angle_schedule,angle_of_attack_degrees,mach,chord_reynolds,"
    "static_temperature_kelvin,grid_policy,sidewall_boundary_condition,common_farfield_chords,"
    "turbulence_model,sutherland_law,wall_thermal_condition,residual_target_log10,gamma,"
    "prandtl,turbulent_prandtl,farfield_kinematic_viscosity_ratio_chi,manifest_sha256,"
    "source_id,source_url,source_kind,source_state,source_byte_length,source_sha256,"
    "source_revision,license_applicability,acknowledgement"
).split(",")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == HEADER, "DPW-8 manifest CSV header mismatch")
        return list(reader)


def add_u64(buffer: bytearray, value: int) -> None:
    require(0 <= value <= (1 << 64) - 1, "u64 value out of range")
    buffer.extend(struct.pack("<Q", value))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    add_u64(buffer, len(raw))
    buffer.extend(raw)


def add_f64(buffer: bytearray, value: float) -> None:
    buffer.extend(struct.pack("<d", value))


def add_f64_option(buffer: bytearray, raw: str) -> None:
    if raw == "":
        buffer.append(0)
    else:
        value = float(raw)
        require(math.isfinite(value), "DPW-8 numeric setting is non-finite")
        buffer.append(1)
        add_f64(buffer, value)


def valid_sha(value: str) -> bool:
    return (
        len(value) == 64
        and value == value.lower()
        and all(character in "0123456789abcdef" for character in value)
    )


def one_record(rows: list[dict[str, str]], kind: str) -> dict[str, str]:
    matches = [row for row in rows if row["record_type"] == kind]
    require(len(matches) == 1, f"expected exactly one {kind} row, got {len(matches)}")
    return matches[0]


def validate_source(row: dict[str, str]) -> None:
    for field in (
        "source_id",
        "source_url",
        "source_kind",
        "source_state",
        "source_revision",
        "license_applicability",
        "acknowledgement",
    ):
        require(bool(row[field].strip()), f"source {field} must not be empty")
    state = row["source_state"]
    length, sha, revision = row["source_byte_length"], row["source_sha256"], row["source_revision"]
    if state == "not_admitted":
        require(not length and not sha, "not-admitted source contains byte metadata")
    elif state == "permission_pending":
        require(
            (not length and not sha)
            or (bool(length) and bool(sha) and int(length) > 0 and valid_sha(sha)
                and revision != "not-yet-byte-pinned"),
            "permission-pending source byte metadata is incomplete",
        )
    elif state in {"byte_pinned_reference_only", "byte_pinned_redistribution_permitted"}:
        require(bool(length) and int(length) > 0 and valid_sha(sha),
                "byte-pinned source lacks nonzero length or SHA-256")
        require(revision != "not-yet-byte-pinned", "byte-pinned source lacks revision")
    else:
        raise ValueError(f"unsupported DPW-8 source state {state!r}")
    if row["source_kind"] == "geometry_ip_and_acknowledgement_policy":
        require(row["acknowledgement"] == "ONERA", "OAT15A geometry acknowledgement must identify ONERA")
        require("ONERA retains" in row["license_applicability"],
                "ONERA geometry IP condition was dropped")


def validate_lane(meta: dict[str, str]) -> None:
    lane = meta["lane"]
    expected = {
        "V1a": {
            "flow_mode": "steady",
            "angle_schedule": "exact_official_list",
            "angles": [1.36, 1.50, 2.50, 3.00, 3.10],
            "grid_policy": "six_levels_four_required_six_desirable",
        },
        "V1b": {
            "flow_mode": "unsteady",
            "angle_schedule": "exact_official_list",
            "angles": [3.25, 3.40, 3.50, 3.60, 3.90],
            "grid_policy": "six_levels_four_required_six_desirable",
        },
        "V1c": {
            "flow_mode": "steady_scatter_reproduction",
            "angle_schedule": "not_specified_by_this_source",
            "angles": [],
            "grid_policy": "updated_family_definition_must_be_admitted",
        },
    }
    require(lane in expected, f"unsupported DPW-8 lane {lane!r}")
    spec = expected[lane]
    require(meta["flow_mode"] == spec["flow_mode"], "DPW-8 lane/flow mode mismatch")
    require(meta["angle_schedule"] == spec["angle_schedule"], "DPW-8 AoA schedule status mismatch")
    actual_angles = [] if not meta["angle_of_attack_degrees"] else [
        float(item) for item in meta["angle_of_attack_degrees"].split(";")
    ]
    require(actual_angles == spec["angles"], "DPW-8 AoA schedule mismatch or cross-lane substitution")
    require(meta["grid_policy"] == spec["grid_policy"], "DPW-8 grid policy mismatch")
    require(meta["geometry_designation"] == "ONERA OAT15A", "ONERA geometry designation mismatch")
    require(meta["geometry_ip_holder"] == "ONERA", "ONERA geometry IP holder mismatch")
    require(meta["geometry_acknowledgement_required_each_share"] == "true",
            "ONERA acknowledgement-per-share condition is missing")
    require("acknowledge ONERA each time" in meta["geometry_share_condition"],
            "ONERA share condition is missing")

    if lane in {"V1a", "V1b"}:
        require(meta["mach"] == "0.73", "V1a/V1b Mach mismatch")
        require(float(meta["chord_reynolds"]) == 3.0e6, "V1a/V1b chord Reynolds mismatch")
        require(float(meta["static_temperature_kelvin"]) == 271.0,
                "V1a/V1b static temperature mismatch")
        require(meta["sidewall_boundary_condition"] == "periodic",
                "V1a/V1b sidewall condition mismatch")
        for field in (
            "common_farfield_chords", "turbulence_model", "sutherland_law",
            "wall_thermal_condition", "residual_target_log10", "gamma", "prandtl",
            "turbulent_prandtl", "farfield_kinematic_viscosity_ratio_chi",
        ):
            require(not meta[field], f"{lane} must not inherit V1c-only setting {field}")
    else:
        for field in ("mach", "chord_reynolds", "static_temperature_kelvin"):
            require(not meta[field], f"V1c base {field} is unspecified in this source and must remain Unknown")
        require(float(meta["common_farfield_chords"]) == 1000.0, "V1c farfield distance mismatch")
        require(meta["turbulence_model"] == "Spalart-Allmaras negative variant, all terms",
                "V1c turbulence specification mismatch")
        require(meta["sutherland_law"] == "true", "V1c must use Sutherland's law")
        require(meta["wall_thermal_condition"] == "adiabatic", "V1c wall condition mismatch")
        require(float(meta["residual_target_log10"]) == -10.0, "V1c residual target mismatch")
        require(float(meta["gamma"]) == 1.4, "V1c gamma mismatch")
        require(float(meta["prandtl"]) == 0.72, "V1c Prandtl mismatch")
        require(float(meta["turbulent_prandtl"]) == 0.9, "V1c turbulent Prandtl mismatch")
        require(float(meta["farfield_kinematic_viscosity_ratio_chi"]) == 3.0,
                "V1c farfield viscosity ratio mismatch")


def canonical_digest(meta: dict[str, str], sources: list[dict[str, str]]) -> str:
    canonical = bytearray()
    for field in (
        "schema",
        "protocol_revision",
        "geometry_designation",
        "geometry_ip_holder",
        "geometry_share_condition",
        "evidence_level",
        "lane",
        "flow_mode",
        "angle_schedule",
        "grid_policy",
        "sidewall_boundary_condition",
    ):
        add_string(canonical, meta[field])
    canonical.append(1 if meta["geometry_acknowledgement_required_each_share"] == "true" else 0)
    angles = [] if not meta["angle_of_attack_degrees"] else [
        float(item) for item in meta["angle_of_attack_degrees"].split(";")
    ]
    add_u64(canonical, len(angles))
    for angle in angles:
        add_f64(canonical, angle)
    for field in (
        "mach",
        "chord_reynolds",
        "static_temperature_kelvin",
        "common_farfield_chords",
        "residual_target_log10",
        "gamma",
        "prandtl",
        "turbulent_prandtl",
        "farfield_kinematic_viscosity_ratio_chi",
    ):
        add_f64_option(canonical, meta[field])
    add_string(canonical, meta["turbulence_model"])
    add_string(canonical, meta["sutherland_law"])
    add_string(canonical, meta["wall_thermal_condition"])

    ordered_sources = sorted(sources, key=lambda row: row["source_id"])
    add_u64(canonical, len(ordered_sources))
    for source in ordered_sources:
        for field in (
            "source_id",
            "source_url",
            "source_kind",
            "source_state",
            "source_revision",
            "license_applicability",
            "acknowledgement",
            "source_sha256",
        ):
            add_string(canonical, source[field])
        if source["source_byte_length"]:
            canonical.append(1)
            add_u64(canonical, int(source["source_byte_length"]))
        else:
            canonical.append(0)
    return hashlib.sha256(canonical).hexdigest()


def main() -> None:
    require(len(sys.argv) == 2, "usage: verify-dpw8-protocol-independent.py MANIFEST.csv")
    rows = read_rows(Path(sys.argv[1]))
    allowed = {"manifest", "source"}
    require(all(row["record_type"] in allowed for row in rows),
            "DPW-8 CSV contains an unknown record type")
    meta = one_record(rows, "manifest")
    require(meta["schema"] == SCHEMA, "DPW-8 schema mismatch")
    require(meta["evidence_level"] == EVIDENCE, "DPW-8 evidence ceiling mismatch")
    require(bool(meta["protocol_revision"]), "DPW-8 protocol revision is empty")
    validate_lane(meta)
    sources = [row for row in rows if row["record_type"] == "source"]
    require(sources, "DPW-8 manifest has no source references")
    source_ids = set()
    for source in sources:
        validate_source(source)
        require(source["source_id"] not in source_ids, "duplicate DPW-8 source ID")
        source_ids.add(source["source_id"])
    required = {
        "dpw8-test-case-definition",
        "onera-oat15a-geometry-policy",
        "dpw8-onera-oat15a-grid-index",
    }
    require(required <= source_ids, "DPW-8 manifest is missing a required source reference")
    digest = canonical_digest(meta, sources)
    require(digest == meta["manifest_sha256"], "DPW-8 canonical manifest digest mismatch")
    print(f"independent_dpw8_protocol_oracle=pass lane={meta['lane']} manifest_sha256={digest}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"independent DPW-8 protocol oracle failed: {error}", file=sys.stderr)
        raise
