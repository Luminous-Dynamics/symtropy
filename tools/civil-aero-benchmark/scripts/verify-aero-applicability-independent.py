#!/usr/bin/env python3
"""Independent stdlib oracle for applicability manifest, query and assessment receipts."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from pathlib import Path

MANIFEST_SCHEMA = "aero-applicability-domain/0.1.0"
QUERY_SCHEMA = "aero-applicability-query/0.1.0"
EVIDENCE_LEVEL = (
    "deterministic_exact_identity_and_explicit_axis_aligned_support_region_membership_"
    "not_physical_validation_or_generalization"
)

MANIFEST_HEADER = (
    "record_type,schema,model_id,quantity,quantity_unit,solver_identity_sha256,"
    "output_definition_sha256,feature_name,feature_unit,evidence_id,evidence_kind,"
    "source_ref,license_applicability,artifact_sha256,scope_sha256,region_id,"
    "geometry_family,flow_regime,support,rationale,supporting_evidence_id,"
    "bound_name,bound_unit,lower_inclusive,upper_inclusive,manifest_sha256"
).split(",")
QUERY_HEADER = (
    "record_type,model_id,quantity,geometry_family,flow_regime,solver_identity_sha256,"
    "output_definition_sha256,feature_name,feature_unit,feature_value,feature_value_bits"
).split(",")
ASSESSMENT_HEADER = (
    "record_type,schema,model_id,quantity,manifest_sha256,query_sha256,status,"
    "region_id,reason,evidence_level,assessment_sha256"
).split(",")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_table(path: Path, expected_header: list[str]) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == expected_header, f"{path.name}: CSV header mismatch")
        return list(reader)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def add_u64(buffer: bytearray, value: int) -> None:
    require(0 <= value <= (1 << 64) - 1, "u64 value out of range")
    buffer.extend(struct.pack("<Q", value))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    add_u64(buffer, len(raw))
    buffer.extend(raw)


def add_f64(buffer: bytearray, value: float) -> None:
    buffer.extend(struct.pack("<d", value))


def valid_digest(value: str) -> bool:
    return (
        len(value) == 64
        and value == value.lower()
        and all(character in "0123456789abcdef" for character in value)
    )


def one_record(rows: list[dict[str, str]], kind: str) -> dict[str, str]:
    matches = [row for row in rows if row["record_type"] == kind]
    require(len(matches) == 1, f"expected exactly one {kind} record, got {len(matches)}")
    return matches[0]


def nonempty(value: str, label: str) -> None:
    require(bool(value.strip()), f"{label} must not be empty")


def region_scope_digest(
    meta: dict[str, str],
    region: dict[str, str],
    bounds: list[dict[str, str]],
) -> str:
    frame = bytearray()
    add_string(frame, "aero-applicability-region-scope/0.1.0")
    for name in (
        "model_id",
        "quantity",
        "quantity_unit",
        "solver_identity_sha256",
        "output_definition_sha256",
    ):
        add_string(frame, meta[name])
    add_string(frame, region["geometry_family"])
    add_string(frame, region["flow_regime"])
    ordered = sorted(bounds, key=lambda item: item["bound_name"])
    add_u64(frame, len(ordered))
    for bound in ordered:
        add_string(frame, bound["bound_name"])
        add_string(frame, bound["bound_unit"])
        add_f64(frame, float(bound["lower_inclusive"]))
        add_f64(frame, float(bound["upper_inclusive"]))
    return digest(bytes(frame))


def verify_manifest(rows: list[dict[str, str]]) -> dict:
    meta = one_record(rows, "manifest")
    require(meta["schema"] == MANIFEST_SCHEMA, "unsupported manifest schema")
    for name in ("model_id", "quantity", "quantity_unit"):
        nonempty(meta[name], name)
    for name in ("solver_identity_sha256", "output_definition_sha256"):
        require(valid_digest(meta[name]), f"{name} digest malformed")

    features: dict[str, str] = {}
    for row in (r for r in rows if r["record_type"] == "feature"):
        nonempty(row["feature_name"], "feature name")
        nonempty(row["feature_unit"], "feature unit")
        require(row["feature_name"] not in features, "duplicate feature name")
        features[row["feature_name"]] = row["feature_unit"]
    require(bool(features), "empty feature schema")

    evidence: dict[str, dict[str, str]] = {}
    for row in (r for r in rows if r["record_type"] == "evidence"):
        evidence_id = row["evidence_id"]
        nonempty(evidence_id, "evidence ID")
        for field in ("evidence_kind", "source_ref", "license_applicability"):
            nonempty(row[field], field)
        for field in ("artifact_sha256", "scope_sha256"):
            require(valid_digest(row[field]), f"{field} malformed")
        require(evidence_id not in evidence, "duplicate evidence ID")
        evidence[evidence_id] = row
    require(bool(evidence), "no evidence references")

    region_rows = [r for r in rows if r["record_type"] == "region"]
    regions: dict[str, dict] = {}
    for row in region_rows:
        region_id = row["region_id"]
        nonempty(region_id, "region ID")
        for field in ("geometry_family", "flow_regime", "support", "rationale"):
            nonempty(row[field], field)
        require(row["support"] in {"dense", "sparse", "boundary_or_regime_transition"},
                "unsupported region support class")
        require(region_id not in regions, "duplicate region ID")
        regions[region_id] = {"row": row, "bounds": [], "evidence_ids": []}
    require(bool(regions), "no support regions")

    for row in (r for r in rows if r["record_type"] == "bound"):
        region_id = row["region_id"]
        require(region_id in regions, f"bound references missing region {region_id}")
        name = row["bound_name"]
        require(name in features, f"bound uses undeclared feature {name}")
        require(row["bound_unit"] == features[name], f"bound unit mismatch for {name}")
        lower, upper = float(row["lower_inclusive"]), float(row["upper_inclusive"])
        require(math.isfinite(lower) and math.isfinite(upper) and lower <= upper,
                f"invalid bounds for {name}")
        require(all(b["bound_name"] != name for b in regions[region_id]["bounds"]),
                f"duplicate bound {name} in region {region_id}")
        regions[region_id]["bounds"].append(row)
    for region_id, record in regions.items():
        names = {b["bound_name"] for b in record["bounds"]}
        require(names == set(features), f"region {region_id} feature coverage mismatch")

    for row in (r for r in rows if r["record_type"] == "region_evidence"):
        region_id, evidence_id = row["region_id"], row["supporting_evidence_id"]
        require(region_id in regions, f"evidence link references missing region {region_id}")
        require(evidence_id in evidence, f"region {region_id} references missing evidence {evidence_id}")
        require(evidence_id not in regions[region_id]["evidence_ids"],
                f"duplicate evidence link {evidence_id}")
        regions[region_id]["evidence_ids"].append(evidence_id)
    used_evidence: set[str] = set()
    for region_id, record in regions.items():
        require(bool(record["evidence_ids"]), f"region {region_id} has no evidence reference")
        region_scope = region_scope_digest(meta, record["row"], record["bounds"])
        for evidence_id in record["evidence_ids"]:
            require(evidence[evidence_id]["scope_sha256"] == region_scope,
                    f"region {region_id} evidence {evidence_id} scope mismatch")
            used_evidence.add(evidence_id)
    require(used_evidence == set(evidence), "orphan evidence references")

    canonical = bytearray()
    add_string(canonical, MANIFEST_SCHEMA)
    for field in (
        "model_id",
        "quantity",
        "quantity_unit",
        "solver_identity_sha256",
        "output_definition_sha256",
    ):
        add_string(canonical, meta[field])
    ordered_features = sorted(features.items())
    add_u64(canonical, len(ordered_features))
    for name, unit in ordered_features:
        add_string(canonical, name)
        add_string(canonical, unit)

    add_u64(canonical, len(evidence))
    for evidence_id in sorted(evidence):
        row = evidence[evidence_id]
        for field in (
            "evidence_id",
            "evidence_kind",
            "source_ref",
            "license_applicability",
            "artifact_sha256",
            "scope_sha256",
        ):
            add_string(canonical, row[field])

    add_u64(canonical, len(regions))
    for region_id in sorted(regions):
        record = regions[region_id]
        row = record["row"]
        for field in ("region_id", "geometry_family", "flow_regime", "support", "rationale"):
            add_string(canonical, row[field])
        ids = sorted(record["evidence_ids"])
        add_u64(canonical, len(ids))
        for evidence_id in ids:
            add_string(canonical, evidence_id)
        bounds = sorted(record["bounds"], key=lambda item: item["bound_name"])
        add_u64(canonical, len(bounds))
        for bound in bounds:
            add_string(canonical, bound["bound_name"])
            add_string(canonical, bound["bound_unit"])
            add_f64(canonical, float(bound["lower_inclusive"]))
            add_f64(canonical, float(bound["upper_inclusive"]))

    actual = digest(bytes(canonical))
    require(actual == meta["manifest_sha256"], "manifest canonical digest mismatch")
    return {"meta": meta, "features": features, "regions": regions, "digest": actual}


def total_cmp_key(value: float) -> int:
    bits = struct.unpack("<Q", struct.pack("<d", value))[0]
    sign = 1 << 63
    return (~bits & ((1 << 64) - 1)) if bits & sign else (bits ^ sign)


def verify_query(rows: list[dict[str, str]]) -> dict:
    meta = one_record(rows, "query")
    features = []
    for row in (r for r in rows if r["record_type"] == "feature"):
        name, unit = row["feature_name"], row["feature_unit"]
        value = float(row["feature_value"])
        bits_text = row["feature_value_bits"]
        require(
            len(bits_text) == 16
            and all(character in "0123456789abcdef" for character in bits_text),
            f"query feature {name} has malformed exact float bits",
        )
        bits = int(bits_text, 16)
        value = struct.unpack("<d", bits.to_bytes(8, "little"))[0]
        features.append({"name": name, "unit": unit, "value": value})
    canonical = bytearray()
    add_string(canonical, QUERY_SCHEMA)
    for field in (
        "model_id",
        "quantity",
        "geometry_family",
        "flow_regime",
        "solver_identity_sha256",
        "output_definition_sha256",
    ):
        add_string(canonical, meta[field])
    ordered = sorted(
        features,
        key=lambda item: (item["name"], item["unit"], total_cmp_key(item["value"])),
    )
    add_u64(canonical, len(ordered))
    for feature in ordered:
        add_string(canonical, feature["name"])
        add_string(canonical, feature["unit"])
        canonical.extend(
            struct.pack("<Q", struct.unpack("<Q", struct.pack("<d", feature["value"]))[0])
        )
    return {"meta": meta, "features": features, "digest": digest(bytes(canonical))}


def classify(manifest: dict, query: dict) -> tuple[str, list[str], str]:
    m = manifest["meta"]
    q = query["meta"]

    for field in ("model_id", "quantity", "geometry_family", "flow_regime"):
        if not q[field].strip():
            return (
                "unknown_due_to_missing_or_unqualified_inputs",
                [],
                f"query {field} is missing",
            )
    if q["model_id"] != m["model_id"] or q["quantity"] != m["quantity"]:
        return (
            "unknown_due_to_missing_or_unqualified_inputs",
            [],
            "query model or quantity differs from the applicability manifest",
        )
    if (
        q["solver_identity_sha256"] != m["solver_identity_sha256"]
        or q["output_definition_sha256"] != m["output_definition_sha256"]
    ):
        return (
            "unknown_due_to_missing_or_unqualified_inputs",
            [],
            "solver or output-definition identity differs from the applicability manifest",
        )

    values = {}
    ordered_features = sorted(
        query["features"],
        key=lambda item: (item["name"], item["unit"], total_cmp_key(item["value"])),
    )
    for feature in ordered_features:
        if not feature["name"].strip() or not feature["unit"].strip():
            return (
                "unknown_due_to_missing_or_unqualified_inputs",
                [],
                "query contains a missing feature name or unit",
            )
        if not math.isfinite(feature["value"]):
            return (
                "unknown_due_to_missing_or_unqualified_inputs",
                [],
                f"query feature {feature['name']} is non-finite",
            )
        if feature["name"] in values:
            return (
                "unknown_due_to_missing_or_unqualified_inputs",
                [],
                f"query duplicates feature {feature['name']}",
            )
        values[feature["name"]] = feature["value"]
    expected = manifest["features"]
    if set(values) != set(expected):
        return (
            "unknown_due_to_missing_or_unqualified_inputs",
            [],
            "query feature set is incomplete or contains undeclared features",
        )
    for feature in ordered_features:
        if expected.get(feature["name"]) != feature["unit"]:
            return (
                "unknown_due_to_missing_or_unqualified_inputs",
                [],
                f"query feature {feature['name']} has unit {feature['unit']}, expected {expected.get(feature['name'], 'undeclared')}",
            )

    matching = []
    for region_id, record in manifest["regions"].items():
        region = record["row"]
        if region["geometry_family"] != q["geometry_family"] or region["flow_regime"] != q["flow_regime"]:
            continue
        contains = True
        for bound in record["bounds"]:
            value = values[bound["bound_name"]]
            if value < float(bound["lower_inclusive"]) or value > float(bound["upper_inclusive"]):
                contains = False
                break
        if contains:
            matching.append(region_id)

    if len(matching) > 1:
        return (
            "unknown_due_to_missing_or_unqualified_inputs",
            sorted(matching),
            "query matches overlapping support regions; policy is ambiguous",
        )
    if not matching:
        return (
            "out_of_domain",
            [],
            "query is outside every explicitly declared support region for this geometry family and flow regime",
        )
    region = manifest["regions"][matching[0]]["row"]
    status = {
        "dense": "supported_within_declared_domain",
        "sparse": "supported_but_sparse",
        "boundary_or_regime_transition": "boundary_or_regime_transition",
    }[region["support"]]
    return (
        status,
        matching,
        f"query lies within declared region {region['region_id']}; support class is {region['support']}",
    )


def verify_assessment(
    rows: list[dict[str, str]],
    manifest: dict,
    query: dict,
) -> None:
    meta = one_record(rows, "assessment")
    matches = sorted(
        row["region_id"] for row in rows if row["record_type"] == "matched_region"
    )
    status, expected_matches, reason = classify(manifest, query)
    require(meta["schema"] == MANIFEST_SCHEMA, "assessment schema mismatch")
    require(meta["evidence_level"] == EVIDENCE_LEVEL, "assessment evidence level mismatch")
    require(meta["model_id"] == manifest["meta"]["model_id"], "assessment model mismatch")
    require(meta["quantity"] == manifest["meta"]["quantity"], "assessment quantity mismatch")
    require(meta["manifest_sha256"] == manifest["digest"], "assessment manifest digest mismatch")
    require(meta["query_sha256"] == query["digest"], "assessment query digest mismatch")
    require(meta["status"] == status, "assessment status mismatch")
    require(matches == expected_matches, "assessment matched-region IDs mismatch")
    require(meta["reason"] == reason, "assessment reason mismatch")

    canonical = bytearray()
    for value in (
        meta["schema"],
        meta["model_id"],
        meta["quantity"],
        meta["manifest_sha256"],
        meta["query_sha256"],
        meta["status"],
    ):
        add_string(canonical, value)
    add_u64(canonical, len(matches))
    for region_id in matches:
        add_string(canonical, region_id)
    add_string(canonical, meta["reason"])
    add_string(canonical, meta["evidence_level"])
    require(digest(bytes(canonical)) == meta["assessment_sha256"],
            "assessment canonical digest mismatch")


def main() -> None:
    require(len(sys.argv) == 4, "usage: oracle MANIFEST.csv QUERY.csv ASSESSMENT.csv")
    manifest = verify_manifest(read_table(Path(sys.argv[1]), MANIFEST_HEADER))
    query = verify_query(read_table(Path(sys.argv[2]), QUERY_HEADER))
    verify_assessment(
        read_table(Path(sys.argv[3]), ASSESSMENT_HEADER),
        manifest,
        query,
    )
    print("independent applicability manifest/query/assessment oracle: PASS")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"independent applicability oracle: FAIL: {error}", file=sys.stderr)
        raise
