#!/usr/bin/env python3
"""Independent standard-library oracle for synthetic cargo/service coupling.

It imports neither Rust crate and recomputes the hourly fuel-to-electricity
conversion, power cap, network output and service-vector verdict.
"""
from __future__ import annotations
import json
import sys
from pathlib import Path

HOURS = 72

def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)

def read_jsonl(path: Path) -> list[dict]:
    values = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise AssertionError(f"{path}:{number}: {exc}") from exc
        require(isinstance(value, dict), f"{path}:{number}: expected object")
        values.append(value)
    return values

def validate(root: Path) -> None:
    planners = read_jsonl(root / "planner-observations.jsonl")
    truths = read_jsonl(root / "oracle-truth.jsonl")
    report = json.loads((root / "coupling-report.json").read_text(encoding="utf-8"))
    require(len(planners) == len(truths) == 12, "expected 12 planner/truth case pairs")
    require(report["caseCount"] == 12 and report["hourlyIntervalCount"] == 864, "report shape")
    require(report["cargoLedgerAndServiceOutputsSeparate"] is True, "planner/oracle separation flag")

    totals = {
        "fuelIssueGrams": 0, "compatibleFuelFeedGrams": 0,
        "grossGenerationWh": 0, "networkDeliverableWh": 0,
        "deliveredLowerWh": 0, "powerCapCurtailedWh": 0,
    }
    hourly = {"guaranteed": 0, "failed": 0, "indeterminate": 0}
    cases = {"guaranteed": 0, "failed": 0, "indeterminate": 0}
    for planner, truth in zip(planners, truths):
        case_id = truth["caseId"]
        require(planner["caseId"] == case_id, f"{case_id}: case pairing")
        require(planner["recordKind"] == "planner_observation", f"{case_id}: planner kind")
        require(truth["recordKind"] == "oracle_truth", f"{case_id}: truth kind")
        require(planner["evidenceMaturity"] == truth["evidenceMaturity"] == "synthetic_fixture", f"{case_id}: maturity")

        def walk(value, where="planner"):
            if isinstance(value, dict):
                for key, child in value.items():
                    require(key not in {
                        "receiptEventId", "receivedAtHour", "custodyRecords", "authorityEventId",
                        "transferEventId", "fuelIssues", "fuelIssueGrams", "grossGenerationWh",
                        "networkDeliverableWh", "serviceDisposition", "serviceVectorVerdict",
                        "caseServiceVerdict", "oracle_truth", "deliveredLowerWh",
                        "generatorStockAvailableGrams", "generatorStockReceivedGrams",
                    }, f"{case_id}: oracle leak at {where}.{key}")
                    walk(child, f"{where}.{key}")
            elif isinstance(value, list):
                for index, child in enumerate(value):
                    walk(child, f"{where}[{index}]")
        walk(planner)

        capability = planner["generatorCapability"]
        require(capability == truth["generatorCapability"], f"{case_id}: capability disagreement")
        cargo_manifest = planner["cargoManifest"]
        cargo = truth["cargoLedger"]
        require(cargo_manifest["recordKind"] == "planner_observation", f"{case_id}: nested cargo planner kind")
        require(cargo["recordKind"] == "oracle_truth", f"{case_id}: nested cargo oracle kind")
        require(cargo_manifest["caseId"] == cargo["caseId"], f"{case_id}: nested cargo case identity")

        fuel_issues = cargo["fuelIssues"]
        issue_by_hour = [0] * HOURS
        for issue in fuel_issues:
            at = issue["issuedAtHour"]
            if at < HOURS:
                issue_by_hour[at] += issue["issuedQuantityGrams"]

        requirements = planner["hourlyRequirements"]
        rows = truth["hourlyServiceTrajectory"]
        cargo_rows = cargo["hourlyTrajectory"]
        require(len(requirements) == len(rows) == len(cargo_rows) == HOURS, f"{case_id}: hourly row count")
        verdicts = []
        for hour, (requirement, row, cargo_row) in enumerate(zip(requirements, rows, cargo_rows)):
            require(requirement["hourIndex"] == row["hourIndex"] == cargo_row["hourIndex"] == hour, f"{case_id}: hour order")
            require(requirement["intervalStartUnixS"] == row["intervalStartUnixS"], f"{case_id}: time boundary")
            require(row["generatorStockReceivedGrams"] == cargo_row["deliveredToGeneratorGrams"], f"{case_id}/{hour}: generator receipt mapping")
            require(row["generatorStockAvailableGrams"] == cargo_row["generatorAvailableGrams"], f"{case_id}/{hour}: generator inventory mapping")

            issue = issue_by_hour[hour]
            compatible = issue if capability["fuelGradeCompatible"] else 0
            energy = (
                compatible
                * capability["syntheticLhvMilliWhPerGram"]
                * capability["efficiencyBasisPoints"]
                // 10_000
                // 1_000
            )
            power_cap = (
                capability["ratedPowerCapacityKw"]
                * 1_000
                * capability["derateBasisPoints"]
                // 10_000
            )
            gross = min(energy, power_cap)
            curtailed = energy - gross
            network = gross * capability["networkDeliveryBasisPoints"] // 10_000
            demand = requirement["demandWh"]
            floor = requirement["criticalFloorWh"]
            uncertainty = capability["uncertaintyBasisPoints"]
            lower = min(demand, network * (10_000 - uncertainty) // 10_000)
            upper = min(demand, network * (10_000 + uncertainty) // 10_000)
            if requirement["telemetryState"] == "stale_or_conflicting":
                disposition = "indeterminate"
            elif lower >= floor:
                disposition = "guaranteed"
            elif upper < floor:
                disposition = "failed"
            else:
                disposition = "indeterminate"

            expected = {
                "fuelIssueGrams": issue,
                "compatibleFuelFeedGrams": compatible,
                "unconstrainedFuelEnergyWh": energy,
                "generatorPowerCapWh": power_cap,
                "powerCapCurtailedWh": curtailed,
                "grossGenerationWh": gross,
                "networkDeliverableWh": network,
                "demandWh": demand,
                "criticalFloorWh": floor,
                "deliveredLowerWh": lower,
                "deliveredUpperWh": upper,
            }
            for key, value in expected.items():
                require(row[key] == value, f"{case_id}/{hour}: {key} expected {value}, got {row[key]}")
            require(row["serviceDisposition"] == row["serviceVectorVerdict"] == disposition, f"{case_id}/{hour}: three-way verdict disagreement")
            require(
                row["requiredGuaranteed"] + row["requiredFailed"] + row["requiredIndeterminate"] == 2,
                f"{case_id}/{hour}: two required predicates",
            )
            require(row["notApplicableCount"] == 5, f"{case_id}/{hour}: scoped N/A count")
            verdicts.append(disposition)
            hourly[disposition] += 1
            for key, value in (
                ("fuelIssueGrams", issue),
                ("compatibleFuelFeedGrams", compatible),
                ("grossGenerationWh", gross),
                ("networkDeliverableWh", network),
                ("deliveredLowerWh", lower),
                ("powerCapCurtailedWh", curtailed),
            ):
                totals[key] += value

        overall = "failed" if "failed" in verdicts else "indeterminate" if "indeterminate" in verdicts else "guaranteed"
        require(truth["caseServiceVerdict"] == overall, f"{case_id}: aggregate service verdict")
        cases[overall] += 1

    for key, value in totals.items():
        require(report[key] == value, f"report {key} mismatch")
    require(report["hourlyGuaranteedCount"] == hourly["guaranteed"], "hourly guarantees")
    require(report["hourlyFailedCount"] == hourly["failed"], "hourly failures")
    require(report["hourlyIndeterminateCount"] == hourly["indeterminate"], "hourly indeterminate")
    require(report["caseGuaranteedCount"] == cases["guaranteed"], "case guarantees")
    require(report["caseFailedCount"] == cases["failed"], "case failures")
    require(report["caseIndeterminateCount"] == cases["indeterminate"], "case indeterminate")
    require(report["requiredPredicateResults"] == 864 * 2, "required predicate total")
    require(report["notApplicablePredicatesPerHour"] == 5, "not-applicable predicate count")

def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: independent_oracle.py OUTPUT_DIRECTORY")
    validate(Path(sys.argv[1]))
    print("independent cargo-to-generator-to-service and vector coupling oracle PASS")

if __name__ == "__main__":
    main()
