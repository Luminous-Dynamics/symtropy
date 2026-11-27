#!/usr/bin/env python3
"""Independent split-integrity and translation-lineage challenger."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

SCHEMA = "luminous.spacecraft-domain-split-audit.receipt.v0.1"
PARTITIONS = ("TRAIN", "CALIBRATION", "TEST")
GROUP_FIELDS = {
    "TARGET": "target_id",
    "SESSION": "session_id",
    "SEQUENCE": "sequence_id",
}
HASH_FIELDS = (
    "frame_sha256",
    "image_sha256",
)


def valid_text(value: str) -> bool:
    return bool(value.strip()) and value != "NONE"


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(c in "0123456789abcdefABCDEF" for c in value)


def cross_partition(values: dict[str, set[str]]) -> list[str]:
    return [
        f"{identity}:{'|'.join(sorted(partitions))}"
        for identity, partitions in sorted(values.items())
        if len(partitions) > 1
    ]


def audit(rows: list[dict[str, str]]) -> dict[str, object]:
    if not rows:
        raise ValueError("manifest has no samples")
    group_kind = rows[0]["split_group_kind"]
    if group_kind not in GROUP_FIELDS:
        raise ValueError(f"unsupported split_group_kind: {group_kind}")

    counts = {partition: 0 for partition in PARTITIONS}
    sample_counts: dict[str, int] = defaultdict(int)
    group_partitions: dict[str, set[str]] = defaultdict(set)
    frame_partitions: dict[str, set[str]] = defaultdict(set)
    image_partitions: dict[str, set[str]] = defaultdict(set)
    translated_partitions: dict[str, set[str]] = defaultdict(set)
    translation_issues: set[str] = set()

    for line_number, row in enumerate(rows, start=2):
        for name in (
            "sample_id", "target_id", "session_id", "sequence_id",
            "domain_id", "split_group_id",
        ):
            if not valid_text(row[name]):
                raise ValueError(f"row {line_number} has invalid {name}")
        if row["split_group_kind"] != group_kind:
            raise ValueError(f"row {line_number} changes split_group_kind")
        if row["split_group_id"] != row[GROUP_FIELDS[group_kind]]:
            raise ValueError(f"row {line_number} split_group_id does not match declared identity")
        if row["partition"] not in counts:
            raise ValueError(f"row {line_number} has unsupported partition")
        counts[row["partition"]] += 1
        sample_counts[row["sample_id"]] += 1

        for name in HASH_FIELDS:
            if not valid_sha(row[name]):
                raise ValueError(f"row {line_number} has invalid {name}")
        group_partitions[row["split_group_id"]].add(row["partition"])
        frame_partitions[row["frame_sha256"]].add(row["partition"])
        image_partitions[row["image_sha256"]].add(row["partition"])

        if row["translation_kind"] == "NONE":
            if any(row[name] != "NONE" for name in (
                "translator_sha256", "translated_sha256",
                "label_audit_sha256", "label_audit_state",
            )):
                raise ValueError(f"row {line_number} has translation metadata without IMAGE_TRANSLATED")
        elif row["translation_kind"] == "IMAGE_TRANSLATED":
            for name in ("translator_sha256", "translated_sha256", "label_audit_sha256"):
                if not valid_sha(row[name]):
                    raise ValueError(f"row {line_number} has incomplete translation lineage")
            translated_partitions[row["translated_sha256"]].add(row["partition"])
            if row["label_audit_state"] != "PASS":
                translation_issues.add(row["sample_id"])
        else:
            raise ValueError(f"row {line_number} has unsupported translation_kind")

    missing = [partition for partition in PARTITIONS if counts[partition] == 0]
    duplicates = sorted(sample for sample, count in sample_counts.items() if count > 1)
    group_leaks = cross_partition(group_partitions)
    frame_leaks = cross_partition(frame_partitions)
    image_leaks = cross_partition(image_partitions)
    translated_leaks = cross_partition(translated_partitions)
    translation_issues = sorted(translation_issues)
    split_leak = bool(duplicates or group_leaks or frame_leaks or image_leaks or translated_leaks)

    if missing:
        disposition = "INCOMPLETE_PARTITION_SET"
    elif split_leak:
        disposition = "SPLIT_LEAKAGE"
    elif translation_issues:
        disposition = "BLOCKED_TRANSLATION_LINEAGE"
    else:
        disposition = "PASS_SPLIT_INTEGRITY"

    return {
        "schema": SCHEMA,
        "row_count": len(rows),
        "split_group_kind": group_kind,
        "partition_counts": counts,
        "missing_partitions": missing,
        "duplicate_sample_ids": duplicates,
        "cross_partition_group_leaks": group_leaks,
        "cross_partition_frame_digest_leaks": frame_leaks,
        "cross_partition_image_digest_leaks": image_leaks,
        "cross_partition_translated_digest_leaks": translated_leaks,
        "translation_lineage_unqualified": translation_issues,
        "disposition": disposition,
        "dataset_source_bytes_authenticated": False,
        "translation_geometry_verified_here": False,
        "operational_exchangeability_proven": False,
        "flight_qualification": False,
    }


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <manifest.tsv> <source-commit> <corpus-sha256>")
    path = Path(sys.argv[1])
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != sys.argv[3]:
        raise SystemExit("manifest SHA-256 does not match input bytes")

    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream, delimiter="\t"))
    result = audit(rows)
    result["source_commit"] = sys.argv[2]
    result["corpus_sha256"] = digest
    receipt = {
        "schema": result.pop("schema"),
        "source_commit": result.pop("source_commit"),
        "corpus_sha256": result.pop("corpus_sha256"),
        **result,
    }
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":"), allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
