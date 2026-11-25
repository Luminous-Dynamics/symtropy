#!/usr/bin/env python3
"""Independent standard-library checker for SAFRB cargo receipt ledger outputs.

Test-only. It does not import the Rust crate or share its projection functions.
"""
from __future__ import annotations
import json
import sys
from pathlib import Path

HOURS = 72
ORACLE_ONLY = {
    "receiptEventId", "receivedAtHour", "receivedLowerGrams", "actualReceivedGrams",
    "receivedUpperGrams", "evidence", "evidenceRef", "evidenceDigest", "assayEventId",
    "assessedAtHour", "disposition", "releasedLowerGrams", "sampleChainRef",
    "authorityEventId", "validFromHour", "validUntilHour", "encumberedGrams",
    "maxTransferGrams", "authorityRef", "transfers", "fuelIssues", "hourlyTrajectory",
    "batchBalances", "inTransitGrams", "deliveredToGeneratorGrams", "fuelIssuedGrams",
}

def require(test: bool, message: str) -> None:
    if not test:
        raise AssertionError(message)

def read_jsonl(path: Path) -> list[dict]:
    rows = []
    for index, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise AssertionError(f"{path}:{index}: {exc}") from exc
        require(isinstance(value, dict), f"{path}:{index}: row must be an object")
        rows.append(value)
    return rows

def walk_planner(value: object, where: str = "planner") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            require(key not in ORACLE_ONLY, f"oracle-only key leaked: {where}.{key}")
            walk_planner(child, f"{where}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            walk_planner(child, f"{where}[{index}]")

def validate(root: Path) -> None:
    planners = read_jsonl(root / "planner-manifests.jsonl")
    truths = read_jsonl(root / "oracle-ledger-truth.jsonl")
    report = json.loads((root / "cargo-ledger-report.json").read_text(encoding="utf-8"))
    require(len(planners) == len(truths) == 9, "expected nine planner/oracle cases")
    require(report["caseCount"] == 9 and report["plannerOracleFilesSeparate"] is True, "report shape/boundary")
    require([p["caseId"] for p in planners] == [t["caseId"] for t in truths], "case pairing")
    for planner, truth in zip(planners, truths):
        require(planner["recordKind"] == "planner_observation", "planner record kind")
        require(truth["recordKind"] == "oracle_truth", "oracle record kind")
        require(planner["schemaVersion"] == truth["schemaVersion"] == "0.1.0", "schema version")
        require(planner["evidenceMaturity"] == truth["evidenceMaturity"] == "synthetic_fixture", "maturity")
        walk_planner(planner)
        rows = truth["hourlyTrajectory"]
        require(len(rows) == HOURS, f'{truth["caseId"]}: horizon length')
        require([row["hourIndex"] for row in rows] == list(range(HOURS)), "hour order")
        for row in rows:
            require(
                row["admittedLowerGrams"] == row["terminalInventoryGrams"]
                + row["inTransitGrams"] + row["deliveredToGeneratorGrams"]
                + row["routeLossGrams"],
                f'{truth["caseId"]} hour {row["hourIndex"]}: mass balance',
            )
            require(
                row["deliveredToGeneratorGrams"] == row["generatorAvailableGrams"] + row["fuelIssuedGrams"],
                f'{truth["caseId"]} hour {row["hourIndex"]}: generator stock balance',
            )
        for batch in truth["batchBalances"]:
            require(
                batch["admittedGrams"] == batch["terminalClosingGrams"] + batch["transferredSourceGrams"],
                f'{truth["caseId"]}: terminal closing balance',
            )
            require(
                batch["transferredSourceGrams"] == batch["deliveredGrams"] + batch["routeLossGrams"],
                f'{truth["caseId"]}: route balance',
            )
            require(batch["deliveredGrams"] == batch["generatorClosingGrams"] + batch["fuelIssuedGrams"], f'{truth["caseId"]}: generator closing balance')
        if truth["caseId"] in {"unverified-receipt", "conflicting-receipt", "after-horizon"}:
            require(all(row["deliveredToGeneratorGrams"] == 0 for row in rows), "non-usable case promoted to service")
        if truth["caseId"] == "partial-late-receipt":
            require(all(row["admittedLowerGrams"] == 0 for row in rows[:12]), "late receipt backdated")
            require(rows[22]["deliveredToGeneratorGrams"] == 195000, "receipt-to-service timing")
    require(report["admittedLowerGrams"] >= report["deliveredToGeneratorGrams"] >= report["fuelIssuedGrams"], "report totals")

def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: independent_oracle.py OUTPUT_DIRECTORY")
    validate(Path(sys.argv[1]))
    print("independent cargo receipt, blind split, event chronology and mass-balance oracle PASS")

if __name__ == "__main__":
    main()
