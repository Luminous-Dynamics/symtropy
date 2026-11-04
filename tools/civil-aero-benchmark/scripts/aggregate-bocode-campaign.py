#!/usr/bin/env python3
"""Aggregate a frozen BOCoDe trial schedule without dropping missing or duplicate attempts."""
from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

VERIFY = Path(__file__).with_name("verify-bocode-run-receipt.py")
EXPECTED_HEADER = ["task_id", "algorithm", "seed"]
OUTPUT_FIELDS = [
    "record_type", "task_id", "algorithm", "seed", "schedule_state",
    "attempt_id", "run_id", "trial_state", "receipt_sha256", "receipt_path",
    "verification_state", "reason",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected", type=Path, required=True,
                        help="frozen CSV with columns task_id,algorithm,seed")
    parser.add_argument("--receipts", type=Path, required=True,
                        help="root directory containing immutable per-attempt receipt folders")
    parser.add_argument("--output", type=Path, required=True, help="campaign ledger CSV")
    parser.add_argument("--summary", type=Path, required=True, help="campaign summary JSON")
    args = parser.parse_args()

    expected: dict[tuple[str, str, int], None] = {}
    with args.expected.open("r", encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == EXPECTED_HEADER, "expected schedule header mismatch")
        for line_number, row in enumerate(reader, start=2):
            key = (row["task_id"].strip(), row["algorithm"].strip(), int(row["seed"]))
            require(all(key[:2]), f"schedule line {line_number}: task and algorithm required")
            require(key not in expected, f"duplicate scheduled trial {key}")
            expected[key] = None
    require(bool(expected), "expected schedule must not be empty")

    attempts: dict[tuple[str, str, int], list[dict[str, object]]] = defaultdict(list)
    invalid_receipts = 0
    receipt_files = sorted(args.receipts.rglob("receipt.json"))
    for receipt_path in receipt_files:
        try:
            data = json.loads(receipt_path.read_text(encoding="utf-8"))
        except Exception as error:
            invalid_receipts += 1
            attempts[("", "", -1)].append({
                "record_type": "receipt",
                "task_id": "",
                "algorithm": "",
                "seed": "",
                "schedule_state": "unmatched_receipt",
                "attempt_id": "",
                "run_id": "",
                "trial_state": "invalid_receipt",
                "receipt_sha256": "",
                "receipt_path": str(receipt_path),
                "verification_state": "invalid",
                "reason": f"receipt JSON cannot be parsed: {type(error).__name__}: {error}",
            })
            continue

        key = (
            str(data.get("task_id", "")),
            str(data.get("algorithm", "")),
            int(data.get("seed", -1)),
        )
        verified = subprocess.run(
            [sys.executable, str(VERIFY), str(receipt_path)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        verification_state = "pass" if verified.returncode == 0 else "invalid"
        if verification_state != "pass":
            invalid_receipts += 1
        attempts[key].append({
            "record_type": "receipt",
            "task_id": key[0],
            "algorithm": key[1],
            "seed": key[2],
            "schedule_state": "",
            "attempt_id": str(data.get("attempt_id", "")),
            "run_id": str(data.get("run_id", "")),
            "trial_state": str(data.get("state", "unknown")),
            "receipt_sha256": str(data.get("receipt_sha256", "")),
            "receipt_path": str(receipt_path),
            "verification_state": verification_state,
            "reason": (
                str(data.get("reason", ""))
                if verification_state == "pass"
                else verified.stderr.strip() or verified.stdout.strip()
            ),
        })

    ledger: list[dict[str, object]] = []
    missing = 0
    duplicates = 0
    scheduled_success = 0
    scheduled_failed = 0
    scheduled_invalid = 0
    for key in sorted(expected):
        rows = sorted(
            attempts.pop(key, []),
            key=lambda row: (str(row["attempt_id"]), str(row["run_id"])),
        )
        if not rows:
            missing += 1
            ledger.append({
                "record_type": "schedule",
                "task_id": key[0],
                "algorithm": key[1],
                "seed": key[2],
                "schedule_state": "missing_receipt",
                "attempt_id": "",
                "run_id": "",
                "trial_state": "missing",
                "receipt_sha256": "",
                "receipt_path": "",
                "verification_state": "missing",
                "reason": "no terminal receipt exists for scheduled trial",
            })
            continue

        valid_rows = [row for row in rows if row["verification_state"] == "pass"]
        success_rows = [row for row in valid_rows if row["trial_state"] == "succeeded"]
        if len(rows) > 1:
            duplicates += 1
            schedule_state = "multiple_attempts_all_retained"
        elif len(valid_rows) == 0:
            schedule_state = "invalid_receipt"
        elif len(success_rows) == 1:
            schedule_state = "one_verified_success"
            scheduled_success += 1
        elif any(row["trial_state"] in ("failed", "invalid_trace", "wrapper_error") for row in valid_rows):
            schedule_state = "verified_failed_attempt"
            scheduled_failed += 1
            scheduled_invalid += sum(row["trial_state"] in ("invalid_trace", "wrapper_error") for row in valid_rows)
        else:
            schedule_state = "verified_terminal_non_success"
            scheduled_failed += 1
        for row in rows:
            ledger.append({**row, "schedule_state": schedule_state})

    unmatched = []
    for key in sorted(attempts):
        if not attempts[key]:
            continue
        unmatched.extend(attempts[key])
    ledger.extend(unmatched)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=OUTPUT_FIELDS, lineterminator="\n")
        writer.writeheader()
        for row in ledger:
            writer.writerow({field: row.get(field, "") for field in OUTPUT_FIELDS})

    campaign_complete = missing == 0 and invalid_receipts == 0 and not unmatched
    campaign_success = (
        campaign_complete
        and duplicates == 0
        and scheduled_success == len(expected)
    )
    summary = {
        "schema": "bocode-campaign-ledger/0.1.0",
        "expected_trial_count": len(expected),
        "receipt_file_count": len(receipt_files),
        "missing_trial_count": missing,
        "duplicate_attempt_trial_count": duplicates,
        "verified_single_success_trial_count": scheduled_success,
        "verified_failed_trial_count": scheduled_failed,
        "verified_invalid_trial_count": scheduled_invalid,
        "invalid_receipt_count": invalid_receipts,
        "unmatched_receipt_count": len(unmatched),
        "campaign_complete": campaign_complete,
        "campaign_success": campaign_success,
        "ledger_csv": str(args.output),
        "note": (
            "Every receipt is retained. Duplicate attempts are not collapsed into a best attempt. "
            "A missing scheduled receipt is an explicit ledger row. campaign_success requires "
            "exactly one verified successful receipt for every scheduled trial."
        ),
    }
    args.summary.parent.mkdir(parents=True, exist_ok=True)
    args.summary.write_text(
        json.dumps(summary, sort_keys=True, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, sort_keys=True))
    return 0 if campaign_success else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"BOCoDe campaign aggregation failed: {type(error).__name__}: {error}", file=sys.stderr)
        raise
