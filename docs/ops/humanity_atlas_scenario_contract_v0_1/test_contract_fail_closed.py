#!/usr/bin/env python3
"""Mutation tests: ensure the local contract gate rejects tampered inputs."""
from __future__ import annotations
import copy
import json
import runpy
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def need(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run_case(name: str, mutate, expected: str) -> None:
    with tempfile.TemporaryDirectory(prefix="humanity-atlas-contract-") as temp:
        root = Path(temp) / "contract"
        shutil.copytree(HERE / "fixtures", root / "fixtures")
        shutil.copytree(HERE / "schemas", root / "schemas")
        shutil.copy2(HERE / "validate_contract.py", root / "validate_contract.py")
        shutil.copy2(HERE / "test_contract_fail_closed.py", root / "test_contract_fail_closed.py")
        mutate(root)
        proc = subprocess.run(
            [sys.executable, str(root / "validate_contract.py")],
            text=True, capture_output=True, check=False,
        )
        combined = proc.stdout + proc.stderr
        need(proc.returncode != 0, f"{name}: mutated fixture unexpectedly passed")
        need(expected in combined, f"{name}: expected {expected!r}, got {combined!r}")
        print(f"PASS: rejected {name} ({expected})")


def mutate_baseline(root: Path) -> None:
    p = root / "fixtures/synthetic_grid_baseline.json"
    p.write_text(p.read_text() + " ", encoding="utf-8")


def mutate_model(root: Path) -> None:
    p = root / "fixtures/synthetic_grid_model.json"
    original = p.read_text()
    changed = original.replace('"annual_storage_growth_rate":0.08', '"annual_storage_growth_rate":0.09')
    need(changed != original, "model mutation target not found")
    p.write_text(changed, encoding="utf-8")


def mutate_intervention_year(root: Path) -> None:
    p = root / "fixtures/earlier_storage_rollout.scenario.json"
    doc = json.loads(p.read_text())
    doc["interventions"][0]["effective_year"] = 2040
    p.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")


def mutate_unknown_parameter(root: Path) -> None:
    p = root / "fixtures/earlier_storage_rollout.scenario.json"
    doc = json.loads(p.read_text())
    doc["interventions"][0]["parameter"] = "magic_forecast_probability"
    p.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")


def mutate_zero_step(root: Path) -> None:
    p = root / "fixtures/earlier_storage_rollout.scenario.json"
    doc = json.loads(p.read_text())
    doc["horizon"]["step_years"] = 0
    p.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")


def test_claim_promotion_gate() -> None:
    namespace = runpy.run_path(str(HERE / "validate_contract.py"))
    gate = namespace["validate_run_claim_gate"]
    fixture = json.loads((HERE / "fixtures/synthetic_smoke_run.v0.1.json").read_text())

    calibrated = copy.deepcopy(fixture)
    calibrated["output_classification"] = "calibrated_simulation"
    calibrated["validation"]["status"] = "retrospective"
    try:
        gate(calibrated)
    except ValueError as exc:
        need("requires retrospective evaluation evidence" in str(exc), f"unexpected calibration rejection: {exc}")
    else:
        raise AssertionError("calibrated_simulation without evidence unexpectedly passed")
    print("PASS: rejected calibration promotion without an evaluation receipt")

    reviewed_but_not_calibrated = copy.deepcopy(fixture)
    reviewed_but_not_calibrated["output_classification"] = "calibrated_simulation"
    reviewed_but_not_calibrated["validation"]["status"] = "external_review"
    reviewed_but_not_calibrated["validation"]["evidence"] = [{
        "evidence_id": "test.review.v1", "kind": "independent_review",
        "artifact_ref": "test-fixtures/review.json", "artifact_sha256": "c" * 64,
        "description": "A review receipt does not replace a retrospective evaluation.",
    }]
    try:
        gate(reviewed_but_not_calibrated)
    except ValueError as exc:
        need("requires retrospective evaluation evidence" in str(exc), f"unexpected review-only rejection: {exc}")
    else:
        raise AssertionError("independent review alone unexpectedly qualified calibration")
    print("PASS: rejected calibration promotion with review but no retrospective evaluation")

    forecast = copy.deepcopy(fixture)
    forecast["output_classification"] = "prospective_forecast"
    forecast["validation"]["status"] = "prospective"
    try:
        gate(forecast)
    except ValueError as exc:
        need("requires forecast receipt evidence" in str(exc), f"unexpected forecast rejection: {exc}")
    else:
        raise AssertionError("prospective_forecast without a receipt unexpectedly passed")
    print("PASS: rejected forecast promotion without a forecast receipt")

    calibrated["validation"]["evidence"] = [{
        "evidence_id": "test.hindcast.report.v1",
        "kind": "retrospective_evaluation",
        "artifact_ref": "test-fixtures/hindcast-report.json",
        "artifact_sha256": "a" * 64,
        "description": "Synthetic evidence shape used only to exercise the gate.",
    }]
    gate(calibrated)
    forecast["validation"]["evidence"] = [{
        "evidence_id": "test.forecast.receipt.v1",
        "kind": "prospective_forecast",
        "artifact_ref": "test-fixtures/forecast-receipt.json",
        "artifact_sha256": "b" * 64,
        "description": "Synthetic evidence shape used only to exercise the gate.",
    }]
    gate(forecast)
    failed_calibrated = copy.deepcopy(calibrated)
    failed_calibrated["run_status"] = "failed"
    try:
        gate(failed_calibrated)
    except ValueError as exc:
        need("requires a completed run" in str(exc), f"unexpected failed-run rejection: {exc}")
    else:
        raise AssertionError("failed calibrated run unexpectedly passed")
    failed_forecast = copy.deepcopy(forecast)
    failed_forecast["run_status"] = "failed"
    try:
        gate(failed_forecast)
    except ValueError as exc:
        need("requires a completed run" in str(exc), f"unexpected failed-forecast rejection: {exc}")
    else:
        raise AssertionError("failed forecast run unexpectedly passed")
    print("PASS: rejected calibrated/forecast labels on failed runs")
    print("PASS: accepted well-formed evidence-receipt shapes (not authenticated evidence)")


def main() -> int:
    run_case("baseline byte tampering", mutate_baseline, "baseline byte digest mismatch")
    run_case("model byte tampering", mutate_model, "model byte digest mismatch")
    run_case("intervention outside horizon", mutate_intervention_year, "intervention year outside horizon")
    run_case("unsupported intervention parameter", mutate_unknown_parameter, "unsupported intervention parameter")
    run_case("invalid time step", mutate_zero_step, "invalid horizon")
    test_claim_promotion_gate()
    print("PASS: 5 fail-closed mutation cases plus evidence, class and run-status gates")
    print("BOUNDARY: structural and deterministic contract tests only; no scientific validity claim")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, ValueError, json.JSONDecodeError) as exc:
        raise SystemExit(f"FAIL: {exc}")
