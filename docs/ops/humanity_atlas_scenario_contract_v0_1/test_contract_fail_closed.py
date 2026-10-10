#!/usr/bin/env python3
"""Mutation tests: ensure the local contract gate rejects tampered inputs."""
from __future__ import annotations
import json
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


def main() -> int:
    run_case("baseline byte tampering", mutate_baseline, "baseline byte digest mismatch")
    run_case("model byte tampering", mutate_model, "model byte digest mismatch")
    run_case("intervention outside horizon", mutate_intervention_year, "intervention year outside horizon")
    run_case("unsupported intervention parameter", mutate_unknown_parameter, "unsupported intervention parameter")
    run_case("invalid time step", mutate_zero_step, "invalid horizon")
    print("PASS: 5 fail-closed mutation cases")
    print("BOUNDARY: structural and deterministic contract tests only; no scientific validity claim")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, ValueError, json.JSONDecodeError) as exc:
        raise SystemExit(f"FAIL: {exc}")
