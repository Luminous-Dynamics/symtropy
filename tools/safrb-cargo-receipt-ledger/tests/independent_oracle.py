#!/usr/bin/env python3
"""Independent standard-library reference oracle for SAFRB cargo ledger outputs.

Test-only: this program does not import the Rust crate or call its projection
functions. It reconstructs state from planner manifests and oracle events.
"""
from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

HOURS = 72
ORACLE_ONLY = {
    "receiptEventId", "receivedAtHour", "receivedLowerGrams", "actualReceivedGrams",
    "receivedUpperGrams", "evidence", "evidenceRef", "evidenceDigest", "assayEventId",
    "assessedAtHour", "disposition", "releasedLowerGrams", "sampleChainRef",
    "authorityEventId", "validFromHour", "validUntilHour", "encumberedGrams",
    "maxTransferGrams", "authorityRef", "transfers", "fuelIssues", "hourlyTrajectory",
    "batchBalances", "inTransitGrams", "deliveredToGeneratorGrams", "fuelIssuedGrams",
    "custodyEventId", "custodyRecords", "effectiveFromHour", "effectiveUntilHour",
    "custodiedLowerGrams", "title", "contract",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise AssertionError(f"{path}:{line_number}: {exc}") from exc
        require(isinstance(value, dict), f"{path}:{line_number}: row must be an object")
        rows.append(value)
    return rows


def walk_planner(value: Any, where: str = "planner") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            require(key not in ORACLE_ONLY, f"oracle-only key leaked: {where}.{key}")
            walk_planner(child, f"{where}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            walk_planner(child, f"{where}[{index}]")


def validate_case(planner: dict[str, Any], truth: dict[str, Any]) -> tuple[int, int, int]:
    case_id = truth["caseId"]
    require(planner["recordKind"] == "planner_observation", f"{case_id}: planner record kind")
    require(truth["recordKind"] == "oracle_truth", f"{case_id}: oracle record kind")
    require(planner["schemaVersion"] == truth["schemaVersion"] == "0.1.0", f"{case_id}: schema version")
    require(planner["evidenceMaturity"] == truth["evidenceMaturity"] == "synthetic_fixture", f"{case_id}: evidence maturity")
    require(planner["caseId"] == case_id, f"{case_id}: planner/oracle case mismatch")
    require(planner["horizonHours"] == truth["horizonHours"] == HOURS, f"{case_id}: horizon mismatch")
    walk_planner(planner)

    manifests = {item["cargoId"]: item for item in planner["manifests"]}
    receipts = {item["batchId"]: item for item in truth["receipts"]}
    assays = {item["batchId"]: item for item in truth["qualityAssessments"]}
    authorities = {item["batchId"]: item for item in truth["authorizations"]}
    custody_records = {item["batchId"]: item for item in truth["custodyRecords"]}
    routes = {item["routeId"]: item for item in truth["routes"]}
    transfers = {item["transferEventId"]: item for item in truth["transfers"]}
    issues = truth["fuelIssues"]

    # Identity and anti-replay checks are recomputed from the raw event stream.
    event_ids: list[str] = []
    for collection, key in (
        (truth["receipts"], "receiptEventId"),
        (truth["qualityAssessments"], "assayEventId"),
        (truth["authorizations"], "authorityEventId"),
        (truth["custodyRecords"], "custodyEventId"),
        (truth["transfers"], "transferEventId"),
        (issues, "issueEventId"),
    ):
        event_ids.extend(item[key] for item in collection)
    require(len(event_ids) == len(set(event_ids)), f"{case_id}: duplicate/replayed event identity")
    require(len(receipts) == len(truth["receipts"]), f"{case_id}: duplicate batch receipt")
    require(len(assays) == len(truth["qualityAssessments"]), f"{case_id}: duplicate batch assay")
    require(len(authorities) == len(truth["authorizations"]), f"{case_id}: duplicate batch authority")
    require(len(custody_records) == len(truth["custodyRecords"]), f"{case_id}: duplicate batch custody record")

    for receipt in truth["receipts"]:
        manifest = manifests.get(receipt["cargoId"])
        require(manifest is not None, f"{case_id}: receipt without planner manifest")
        for field in ("terminalId", "product", "grade"):
            require(receipt[field] == manifest[field], f"{case_id}: receipt {field} mismatch")
        require(
            receipt["receivedLowerGrams"] <= receipt["actualReceivedGrams"] <= receipt["receivedUpperGrams"],
            f"{case_id}: receipt interval excludes actual quantity",
        )
        require(
            receipt["receivedUpperGrams"] <= manifest["plannedQuantityGrams"],
            f"{case_id}: unexplained receipt overdelivery without manifest amendment",
        )
        if receipt["evidence"] == "verified":
            require(bool(receipt["evidenceRef"]) and bool(receipt["evidenceDigest"]), f"{case_id}: verified receipt lacks evidence")
        else:
            require(receipt["evidenceRef"] == "" and receipt["evidenceDigest"] == "", f"{case_id}: unverified receipt claims evidence")

    admitted_by_batch: dict[str, int] = {}
    for batch_id, receipt in receipts.items():
        assay = assays.get(batch_id)
        if assay is not None:
            require(assay["assessedAtHour"] >= receipt["receivedAtHour"], f"{case_id}: assay predates receipt")
            require(assay["releasedLowerGrams"] <= receipt["receivedLowerGrams"], f"{case_id}: assay release exceeds receipt lower bound")
            if assay["disposition"] == "passed":
                require(bool(assay["sampleChainRef"]) and bool(assay["evidenceRef"]), f"{case_id}: passing assay lacks sample/evidence chain")
            else:
                require(assay["releasedLowerGrams"] == 0, f"{case_id}: non-passing assay released inventory")
        admitted = 0
        if receipt["evidence"] == "verified" and assay is not None and assay["disposition"] == "passed":
            admitted = assay["releasedLowerGrams"]
        admitted_by_batch[batch_id] = admitted

    for batch_id, custody in custody_records.items():
        receipt = receipts.get(batch_id)
        assay = assays.get(batch_id)
        require(receipt is not None and assay is not None, f"{case_id}: custody without receipt/assay")
        require(custody["effectiveFromHour"] >= receipt["receivedAtHour"], f"{case_id}: custody predates receipt")
        require(custody["effectiveFromHour"] >= assay["assessedAtHour"], f"{case_id}: custody predates assay")
        require(custody["effectiveUntilHour"] > custody["effectiveFromHour"], f"{case_id}: invalid custody time window")
        if custody["status"] == "verified":
            require(receipt["evidence"] == "verified" and assay["disposition"] == "passed", f"{case_id}: custody promotes inadmissible stock")
            require(custody["custodiedLowerGrams"] <= admitted_by_batch[batch_id], f"{case_id}: custody exceeds admitted stock")
            require(bool(custody["evidenceRef"]) and bool(custody["evidenceDigest"]), f"{case_id}: verified custody lacks evidence")
        else:
            require(custody["custodiedLowerGrams"] == 0, f"{case_id}: conflicting custody claims usable stock")

    for batch_id, authority in authorities.items():
        receipt = receipts.get(batch_id)
        assay = assays.get(batch_id)
        require(receipt is not None and assay is not None, f"{case_id}: authority without receipt/assay")
        require(authority["validFromHour"] >= receipt["receivedAtHour"], f"{case_id}: authority predates receipt")
        require(authority["validFromHour"] >= assay["assessedAtHour"], f"{case_id}: authority predates assay")
        require(authority["validUntilHour"] > authority["validFromHour"], f"{case_id}: invalid authority interval")
        admitted = admitted_by_batch[batch_id]
        if authority["disposition"] == "authorized":
            require(receipt["evidence"] == "verified" and assay["disposition"] == "passed", f"{case_id}: authority promotes unverified/non-passing stock")
            require(bool(authority["authorityRef"]), f"{case_id}: authority reference absent")
            require(authority["encumberedGrams"] <= admitted, f"{case_id}: encumbrance exceeds admitted quantity")
            require(
                authority["maxTransferGrams"] <= admitted - authority["encumberedGrams"],
                f"{case_id}: authority exceeds unencumbered admitted quantity",
            )
        else:
            require(authority["maxTransferGrams"] == 0, f"{case_id}: withheld authority grants transfer quantity")

    # Recompute route loading utilization over every hour, including overlaps.
    route_usage: dict[tuple[str, int], int] = defaultdict(int)
    transferred: dict[str, int] = defaultdict(int)
    delivered_by_batch: dict[str, int] = defaultdict(int)
    route_loss_by_batch: dict[str, int] = defaultdict(int)
    transfer_ids = set(transfers)
    for transfer in truth["transfers"]:
        batch_id = transfer["batchId"]
        receipt = receipts.get(batch_id)
        assay = assays.get(batch_id)
        authority = authorities.get(batch_id)
        custody = custody_records.get(batch_id)
        route = routes.get(transfer["routeId"])
        require(receipt is not None and assay is not None and authority is not None and custody is not None and route is not None, f"{case_id}: transfer missing parent")
        require(receipt["evidence"] == "verified" and assay["disposition"] == "passed" and authority["disposition"] == "authorized", f"{case_id}: transfer promotes inadmissible stock")
        require(custody["status"] == "verified" and custody["title"] == "established" and custody["contract"] == "active", f"{case_id}: transfer lacks custody/title/contract eligibility")
        require(transfer["loadStartHour"] >= receipt["receivedAtHour"], f"{case_id}: transfer predates receipt")
        require(transfer["loadStartHour"] >= assay["assessedAtHour"], f"{case_id}: transfer predates assay")
        require(transfer["loadStartHour"] >= authority["validFromHour"], f"{case_id}: transfer predates authority")
        require(transfer["loadStartHour"] >= custody["effectiveFromHour"], f"{case_id}: transfer predates custody/title/contract")
        require(transfer["dispatchHour"] > transfer["loadStartHour"], f"{case_id}: empty/negative loading window")
        require(transfer["dispatchHour"] < authority["validUntilHour"], f"{case_id}: dispatch outside authority window")
        require(transfer["dispatchHour"] < custody["effectiveUntilHour"], f"{case_id}: dispatch outside custody/title/contract window")
        require(
            transfer["arrivalHour"] >= transfer["dispatchHour"] + route["transitDelayHours"],
            f"{case_id}: transit time understated",
        )
        source = transfer["sourceQuantityGrams"]
        delivered = transfer["deliveredQuantityGrams"]
        loss = transfer["declaredRouteLossGrams"]
        require(source == delivered + loss, f"{case_id}: transfer mass balance")
        duration = transfer["dispatchHour"] - transfer["loadStartHour"]
        quotient, remainder = divmod(source, duration)
        for offset in range(duration):
            route_usage[(transfer["routeId"], transfer["loadStartHour"] + offset)] += quotient + (1 if offset < remainder else 0)
        transferred[batch_id] += source
        delivered_by_batch[batch_id] += delivered
        route_loss_by_batch[batch_id] += loss
    for (route_id, hour), quantity in route_usage.items():
        require(quantity <= routes[route_id]["capacityGramsPerHour"], f"{case_id}: route capacity exceeded at {route_id}/{hour}")

    for batch_id, source in transferred.items():
        authority = authorities[batch_id]
        custody = custody_records[batch_id]
        admitted = admitted_by_batch[batch_id]
        accessible = max(0, admitted - authority["encumberedGrams"])
        require(source <= custody["custodiedLowerGrams"], f"{case_id}: transfer exceeds custodied quantity")
        require(source <= accessible, f"{case_id}: transfer exceeds accessible stock")
        require(source <= authority["maxTransferGrams"], f"{case_id}: transfer exceeds authority maximum")

    issued_by_transfer: dict[str, int] = defaultdict(int)
    issued_by_batch: dict[str, int] = defaultdict(int)
    for issue in issues:
        transfer = transfers.get(issue["transferEventId"])
        require(transfer is not None, f"{case_id}: issue without transfer")
        require(issue["issuedAtHour"] >= transfer["arrivalHour"], f"{case_id}: fuel issued before generator receipt")
        issued_by_transfer[issue["transferEventId"]] += issue["issuedQuantityGrams"]
        issued_by_batch[transfer["batchId"]] += issue["issuedQuantityGrams"]
    for transfer_id, issued in issued_by_transfer.items():
        require(issued <= transfers[transfer_id]["deliveredQuantityGrams"], f"{case_id}: issue exceeds transfer delivery")

    # Independently reconstruct the 72 hourly states from raw events.
    rows = truth["hourlyTrajectory"]
    require(len(rows) == HOURS, f"{case_id}: hourly trajectory length")
    require([row["hourIndex"] for row in rows] == list(range(HOURS)), f"{case_id}: hour order")
    for hour, row in enumerate(rows):
        admitted_total = terminal_total = accessible_total = in_transit_total = 0
        generator_delivery_total = loss_total = issue_total = 0
        for batch_id, receipt in receipts.items():
            if receipt["receivedAtHour"] > hour:
                continue
            assay = assays.get(batch_id)
            admitted = 0
            if (
                assay is not None
                and assay["assessedAtHour"] <= hour
                and receipt["evidence"] == "verified"
                and assay["disposition"] == "passed"
            ):
                admitted = assay["releasedLowerGrams"]
            dispatched = 0
            for transfer in truth["transfers"]:
                if transfer["batchId"] != batch_id:
                    continue
                if transfer["dispatchHour"] <= hour:
                    dispatched += transfer["sourceQuantityGrams"]
                if transfer["dispatchHour"] <= hour < transfer["arrivalHour"]:
                    in_transit_total += transfer["sourceQuantityGrams"]
                if transfer["arrivalHour"] <= hour:
                    generator_delivery_total += transfer["deliveredQuantityGrams"]
                    loss_total += transfer["declaredRouteLossGrams"]
            terminal = admitted - dispatched
            require(terminal >= 0, f"{case_id}: negative terminal inventory at hour {hour}")
            admitted_total += admitted
            terminal_total += terminal
            authority = authorities.get(batch_id)
            custody = custody_records.get(batch_id)
            if (
                authority is not None
                and authority["disposition"] == "authorized"
                and custody is not None
                and custody["status"] == "verified"
                and custody["title"] == "established"
                and custody["contract"] == "active"
                and authority["validFromHour"] <= hour < authority["validUntilHour"]
                and custody["effectiveFromHour"] <= hour < custody["effectiveUntilHour"]
            ):
                available = max(0, admitted - authority["encumberedGrams"] - dispatched)
                transfer_remaining = max(0, authority["maxTransferGrams"] - dispatched)
                custody_remaining = max(0, custody["custodiedLowerGrams"] - dispatched)
                accessible_total += min(available, transfer_remaining, custody_remaining)
        for issue in issues:
            if issue["issuedAtHour"] <= hour:
                issue_total += issue["issuedQuantityGrams"]
        generator_available = generator_delivery_total - issue_total
        require(generator_available >= 0, f"{case_id}: negative generator inventory at hour {hour}")
        expected = {
            "admittedLowerGrams": admitted_total,
            "terminalInventoryGrams": terminal_total,
            "terminalAccessibleGrams": accessible_total,
            "inTransitGrams": in_transit_total,
            "deliveredToGeneratorGrams": generator_delivery_total,
            "generatorAvailableGrams": generator_available,
            "fuelIssuedGrams": issue_total,
            "routeLossGrams": loss_total,
        }
        for key, value in expected.items():
            require(row[key] == value, f"{case_id}: hour {hour} {key} expected {value}, got {row[key]}")
        require(
            admitted_total == terminal_total + in_transit_total + generator_delivery_total + loss_total,
            f"{case_id}: hour {hour} inventory conservation",
        )
        require(
            generator_delivery_total == generator_available + issue_total,
            f"{case_id}: hour {hour} generator conservation",
        )

    # Reconcile every batch subledger, including post-horizon events.
    balances = {item["batchId"]: item for item in truth["batchBalances"]}
    require(set(balances) == set(receipts), f"{case_id}: batch balance set mismatch")
    for batch_id, receipt in receipts.items():
        admitted = admitted_by_batch[batch_id]
        transferred_total = transferred[batch_id]
        delivered_total = delivered_by_batch[batch_id]
        loss_total = route_loss_by_batch[batch_id]
        issue_total = issued_by_batch[batch_id]
        unverified = receipt["receivedLowerGrams"] if receipt["evidence"] != "verified" else 0
        unreleased = receipt["receivedLowerGrams"] - admitted if receipt["evidence"] == "verified" else 0
        expected = {
            "receivedLowerGrams": receipt["receivedLowerGrams"],
            "admittedGrams": admitted,
            "quarantinedOrUnreleasedGrams": unreleased,
            "unverifiedGrams": unverified,
            "terminalClosingGrams": admitted - transferred_total,
            "transferredSourceGrams": transferred_total,
            "deliveredGrams": delivered_total,
            "routeLossGrams": loss_total,
            "generatorClosingGrams": delivered_total - issue_total,
            "fuelIssuedGrams": issue_total,
        }
        require(admitted >= transferred_total, f"{case_id}: batch overdraw")
        for key, value in expected.items():
            require(balances[batch_id][key] == value, f"{case_id}: batch {batch_id} {key} mismatch")

    # Cases that have no receipt/admission before the horizon cannot show service-ready delivery.
    if case_id in {"unverified-receipt", "conflicting-receipt", "after-horizon"}:
        require(all(row["deliveredToGeneratorGrams"] == 0 for row in rows), f"{case_id}: inadmissible stock promoted")
    if case_id == "partial-late-receipt":
        require(all(row["admittedLowerGrams"] == 0 for row in rows[:12]), f"{case_id}: receipt backdated")
        require(rows[22]["deliveredToGeneratorGrams"] == 195_000, f"{case_id}: partial receipt delivery timing")

    return (
        sum(item["plannedQuantityGrams"] for item in planner["manifests"]),
        sum(item["admittedGrams"] for item in balances.values()),
        sum(item["deliveredGrams"] for item in balances.values()),
    )


def validate(root: Path) -> None:
    planners = read_jsonl(root / "planner-manifests.jsonl")
    truths = read_jsonl(root / "oracle-ledger-truth.jsonl")
    report = json.loads((root / "cargo-ledger-report.json").read_text(encoding="utf-8"))
    require(len(planners) == len(truths) == 12, "expected twelve planner/oracle cases")
    require(report["caseCount"] == 12 and report["plannerOracleFilesSeparate"] is True, "report shape/boundary")
    require([row["caseId"] for row in planners] == [row["caseId"] for row in truths], "planner/oracle pairing")
    totals = [validate_case(planner, truth) for planner, truth in zip(planners, truths)]
    require(report["manifestCount"] == sum(len(row["manifests"]) for row in planners), "manifest total mismatch")
    require(report["receiptCount"] == sum(len(row["receipts"]) for row in truths), "receipt total mismatch")
    require(report["admittedLowerGrams"] == sum(item["admittedGrams"] for truth in truths for item in truth["batchBalances"]), "admitted aggregate mismatch")
    require(report["deliveredToGeneratorGrams"] == sum(item["deliveredGrams"] for truth in truths for item in truth["batchBalances"]), "delivery aggregate mismatch")
    require(report["fuelIssuedGrams"] == sum(item["fuelIssuedGrams"] for truth in truths for item in truth["batchBalances"]), "fuel issue aggregate mismatch")
    require(len(totals) == 9, "case validation count")

def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: independent_oracle.py OUTPUT_DIRECTORY")
    validate(Path(sys.argv[1]))
    print("independent cargo receipt, admission, authority, route, generator and blind-split oracle PASS")

if __name__ == "__main__":
    main()
