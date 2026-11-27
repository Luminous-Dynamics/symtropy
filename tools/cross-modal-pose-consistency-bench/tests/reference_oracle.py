#!/usr/bin/env python3
"""Independent oracle for the cross-modal spacecraft pose consistency gate."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from pathlib import Path

SCHEMA = "luminous.spacecraft-cross-modal-pose-consistency.receipt.v0.1"
Q30_ONE = 1 << 30
DEPENDENCIES = (
    "clock", "optics", "extrinsics", "model", "training_data",
    "target_geometry", "renderer", "annotation", "translator",
)


def valid_text(value: str) -> bool:
    return bool(value.strip()) and value != "NONE"


def valid_digest(value: str) -> bool:
    return len(value) == 64 and all(c in "0123456789abcdefABCDEF" for c in value)


def boolean(value: str, label: str) -> bool:
    if value == "true":
        return True
    if value == "false":
        return False
    raise ValueError(f"invalid boolean {label}: {value}")


def ceil_sqrt(value: int) -> int:
    root = math.isqrt(value)
    return root if root * root == value else root + 1


def estimate(row: dict[str, str], modality: str) -> dict[str, object]:
    prefix = modality.lower()
    fields = {
        "sample_id": row[f"{prefix}_sample_id"],
        "frame_id": row[f"{prefix}_frame_id"],
        "target_id": row[f"{prefix}_target_id"],
        "frame_convention": row[f"{prefix}_frame_convention"],
        "transform_direction": row[f"{prefix}_transform_direction"],
        "translation_unit": row[f"{prefix}_translation_unit"],
        "quaternion_order": row[f"{prefix}_quaternion_order"],
    }
    for name, value in fields.items():
        if not valid_text(value):
            raise ValueError(f"{modality} {name} is empty or reserved")

    dependencies = {name: row[f"{prefix}_{name}_sha256"] for name in DEPENDENCIES}
    for name, digest in dependencies.items():
        if not valid_digest(digest):
            raise ValueError(f"{modality} dependency digest {name} is malformed")

    quaternion = [
        int(row[f"{prefix}_q{axis}_q30"])
        for axis in ("x", "y", "z", "w")
    ]
    norm_sq = sum(value * value for value in quaternion)
    target_sq = Q30_ONE * Q30_ONE
    if abs(norm_sq - target_sq) > target_sq // 1000:
        raise ValueError(f"{modality} quaternion is not unit within Q30 tolerance")

    status = row[f"{prefix}_status"]
    if status not in ("CANDIDATE", "ABSTAINED", "INVALID", "OUT_OF_DISTRIBUTION"):
        raise ValueError(f"{modality} unsupported status {status}")

    return {
        **fields,
        "modality": modality,
        "timestamp_ms": int(row[f"{prefix}_timestamp_ms"]),
        "translation_um": [int(row[f"{prefix}_t{axis}_um"]) for axis in ("x", "y", "z")],
        "quaternion_xyzw_q30": quaternion,
        "status": status,
        "ood": boolean(row[f"{prefix}_ood"], f"{modality} OOD"),
        "low_observability": boolean(
            row[f"{prefix}_low_observability"], f"{modality} low observability"
        ),
        "dependencies": {
            **dependencies,
            "declarations_complete": boolean(
                row[f"{prefix}_dependencies_complete"],
                f"{modality} dependency completeness",
            ),
        },
    }


def evaluate_pair(rgb: dict[str, object], event: dict[str, object], row: dict[str, str]) -> dict[str, object]:
    skew = abs(rgb["timestamp_ms"] - event["timestamp_ms"])
    base = {
        "sample_id": rgb["sample_id"],
        "frame_id": rgb["frame_id"],
        "target_id": rgb["target_id"],
        "timestamp_skew_ms": skew,
        "translation_disagreement_um": None,
        "orientation_chord_q30": None,
        "shared_dependency_kinds": [],
        "rgb_estimate_preserved": True,
        "event_estimate_preserved": True,
        "independence_proven": False,
        "source_bytes_authenticated": False,
        "physical_truth_established": False,
        "deterministic_safety_bound": False,
        "maneuver_authority_granted": False,
    }
    compatible = all(
        rgb[key] == event[key]
        for key in (
            "sample_id", "frame_id", "target_id", "frame_convention",
            "transform_direction", "translation_unit", "quaternion_order",
        )
    )
    if not compatible:
        return {
            "disposition": "OUT_OF_SCOPE",
            "reason_codes": ["FRAME_TARGET_OR_COORDINATE_SCOPE_MISMATCH"],
            **base,
        }
    if skew > int(row["max_timestamp_skew_ms"]):
        return {
            "disposition": "OUT_OF_SCOPE",
            "reason_codes": ["TIMESTAMP_SKEW_EXCEEDS_POLICY"],
            **base,
        }

    if rgb["status"] != "CANDIDATE" or event["status"] != "CANDIDATE":
        if (
            rgb["status"] == "OUT_OF_DISTRIBUTION"
            or event["status"] == "OUT_OF_DISTRIBUTION"
            or rgb["ood"]
            or event["ood"]
        ):
            reason = "MODALITY_OUT_OF_DISTRIBUTION"
        elif rgb["status"] == "ABSTAINED" or event["status"] == "ABSTAINED":
            reason = "MODALITY_ABSTAINED"
        else:
            reason = "MODALITY_ESTIMATE_INVALID"
        return {
            "disposition": "ABSTAIN_MODALITY_UNAVAILABLE",
            "reason_codes": [reason],
            **base,
        }
    if rgb["low_observability"] or event["low_observability"]:
        return {
            "disposition": "ABSTAIN_LOW_OBSERVABILITY",
            "reason_codes": ["LOW_OBSERVABILITY_IN_ONE_OR_MORE_MODALITIES"],
            **base,
        }

    translation = ceil_sqrt(sum(
        (left - right) ** 2
        for left, right in zip(rgb["translation_um"], event["translation_um"], strict=True)
    ))
    left_q = rgb["quaternion_xyzw_q30"]
    right_q = event["quaternion_xyzw_q30"]
    direct = [left - right for left, right in zip(left_q, right_q, strict=True)]
    antipodal = [left + right for left, right in zip(left_q, right_q, strict=True)]
    orientation = min(
        ceil_sqrt(sum(value * value for value in direct)),
        ceil_sqrt(sum(value * value for value in antipodal)),
    )

    result = {
        **base,
        "translation_disagreement_um": translation,
        "orientation_chord_q30": orientation,
        "disposition": "CANDIDATE_CONSISTENT",
        "reason_codes": [],
    }
    if (
        translation > int(row["max_translation_disagreement_um"])
        or orientation > int(row["max_orientation_chord_q30"])
    ):
        result["disposition"] = "DISAGREEMENT"
        result["reason_codes"].append("POSE_DISAGREEMENT_EXCEEDS_POLICY")

    shared = sorted(
        name for name in DEPENDENCIES
        if rgb["dependencies"][name] != "NONE"
        and rgb["dependencies"][name] == event["dependencies"][name]
    )
    result["shared_dependency_kinds"] = shared
    if (
        not rgb["dependencies"]["declarations_complete"]
        or not event["dependencies"]["declarations_complete"]
    ):
        result["disposition"] = "DEPENDENCE_UNKNOWN"
        result["reason_codes"].append("DEPENDENCY_DECLARATIONS_INCOMPLETE")
    if shared:
        result["disposition"] = "DEPENDENCE_UNKNOWN"
        result["reason_codes"].append("SHARED_COMMON_CAUSE_DEPENDENCY")
    result["reason_codes"] = sorted(set(result["reason_codes"]))
    return result


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: reference_oracle.py <pairs.tsv> <source-commit> <corpus-sha256>"
        )
    path = Path(sys.argv[1])
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != sys.argv[3]:
        raise SystemExit("manifest SHA-256 does not match raw bytes")
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream, delimiter="\t"))
    if not rows:
        raise ValueError("manifest has no pairs")

    seen = set()
    results = []
    for row in rows:
        rgb = estimate(row, "RGB")
        event = estimate(row, "EVENT")
        if rgb["sample_id"] in seen:
            raise ValueError("duplicate pair sample id")
        seen.add(rgb["sample_id"])
        results.append(evaluate_pair(rgb, event, row))

    receipt = {
        "schema": SCHEMA,
        "source_commit": sys.argv[2],
        "corpus_sha256": digest,
        "pair_count": len(results),
        "results": results,
        "independence_proven": False,
        "physical_truth_established": False,
        "deterministic_safety_bound": False,
        "maneuver_authority_granted": False,
    }
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":"), allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
