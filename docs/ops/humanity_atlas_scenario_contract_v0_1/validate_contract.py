#!/usr/bin/env python3
"""Dependency-free structural gate and synthetic replay smoke test."""
from __future__ import annotations
import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
FIXTURES = ROOT / "fixtures"
SCHEMAS = ROOT / "schemas"


def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def need(ok: bool, message: str) -> None:
    if not ok:
        raise ValueError(message)


def replay(scenario: dict[str, Any], baseline: dict[str, Any], model: dict[str, Any]) -> dict[str, Any]:
    start, end, step = (scenario["horizon"][k] for k in ("start_year", "end_year", "step_years"))
    ref = baseline["reference_year"]
    initial_storage = float(baseline["installed_storage_gwh"])
    storage = initial_storage
    base_unserved = float(baseline["unserved_energy_gwh_per_year"])
    outputs: list[dict[str, Any]] = []

    def record(year: int) -> None:
        unserved = base_unserved * (initial_storage / storage) * (1 + float(model["annual_demand_growth_rate"])) ** (year - ref)
        for metric, unit, value in (
            ("installed_storage_gwh", "GWh", storage),
            ("unserved_energy_gwh_per_year", "GWh/year", unserved),
        ):
            outputs.append({"metric_id": metric, "year": year, "unit": unit,
                            "statistic": "value", "value": round(value, 6), "sample_count": 1})

    need(start == ref, "reference runner requires horizon start == baseline reference year")
    record(start)
    for year in range(start + 1, end + 1):
        growth = float(model["annual_storage_growth_rate"])
        for intervention in scenario["interventions"]:
            if intervention["parameter"] != "annual_storage_growth_rate" or year < intervention["effective_year"]:
                continue
            if "expires_year" in intervention and year > intervention["expires_year"]:
                continue
            amount = float(intervention["value"])
            if intervention["operation"] == "set":
                growth = amount
            elif intervention["operation"] == "add":
                growth += amount
            elif intervention["operation"] == "multiply":
                growth *= amount
            else:
                raise ValueError(f"unsupported operation: {intervention['operation']}")
        need(growth > -1, "intervention creates a nonphysical negative storage factor")
        storage *= 1 + growth
        if (year - start) % step == 0 or year == end:
            record(year)

    scenario_path = FIXTURES / "earlier_storage_rollout.scenario.json"
    scenario_sha = digest(scenario_path)
    return {
        "schema_version": "humanity-atlas.run.v0.1",
        "run_id": f"ha.run.synthetic.{scenario_sha[:16]}",
        "scenario_id": scenario["scenario_id"],
        "inputs": {"scenario_sha256": scenario_sha, "dataset_sha256": scenario["baseline"]["dataset_sha256"]},
        "engine": {"adapter": scenario["engine"]["adapter"],
                   "engine_version": scenario["engine"]["engine_version"],
                   "model_sha256": scenario["engine"]["model_sha256"]},
        "run_status": "completed",
        "output_classification": "synthetic_fixture",
        "deterministic": True,
        "seed": scenario["engine"]["seed"],
        "replicates": 1,
        "outputs": outputs,
        "validation": {"status": "contract_smoke", "notes": [
            "Synthetic reference calculation only; not a Symtropy engine run.",
            "No empirical calibration, historical validation, or forecasting claim is made.",
            "Single deterministic replicate does not quantify uncertainty."], "evidence": []},
        "warnings": list(scenario["limitations"]),
    }



def validate_run_claim_gate(run: dict[str, Any]) -> None:
    """Reject stronger run labels without matching evidence receipts."""
    classification = run["output_classification"]
    validation = run["validation"]
    status = validation["status"]
    evidence = validation.get("evidence", [])
    kinds = {item.get("kind") for item in evidence}
    if classification == "calibrated_simulation":
        need(status in {"retrospective", "external_review"},
             "calibrated_simulation requires retrospective or external_review status")
        need(bool(kinds & {"retrospective_evaluation", "independent_review"}),
             "calibrated_simulation requires evaluation evidence")
    elif classification == "prospective_forecast":
        need(status in {"prospective", "external_review"},
             "prospective_forecast requires prospective or external_review status")
        need("prospective_forecast" in kinds,
             "prospective_forecast requires forecast receipt evidence")
    for item in evidence:
        need(bool(item.get("evidence_id", "").strip()), "validation evidence ID is required")
        need(bool(item.get("artifact_ref", "").strip()), "validation evidence artifact reference is required")
        digest_value = item.get("artifact_sha256", "")
        need(len(digest_value) == 64 and all(ch in "0123456789abcdef" for ch in digest_value),
             "validation evidence artifact_sha256 must be lowercase SHA-256")
        need(bool(item.get("description", "").strip()), "validation evidence description is required")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--emit-run", action="store_true", help="print the generated synthetic run envelope")
    args = parser.parse_args()

    scenario_path = FIXTURES / "earlier_storage_rollout.scenario.json"
    baseline_path = FIXTURES / "synthetic_grid_baseline.json"
    model_path = FIXTURES / "synthetic_grid_model.json"
    scenario, baseline, model = load(scenario_path), load(baseline_path), load(model_path)
    schemas = {"scenario": load(SCHEMAS / "scenario-v0.1.schema.json"),
               "run": load(SCHEMAS / "run-v0.1.schema.json")}
    for name, schema in schemas.items():
        need(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema", f"{name} schema draft missing")
        need(schema.get("type") == "object" and schema.get("additionalProperties") is False, f"{name} schema envelope invalid")
        need(bool(schema.get("required")), f"{name} schema must define required keys")

    need(scenario.get("schema_version") == "humanity-atlas.scenario.v0.1", "scenario version mismatch")
    need(scenario["intent"] in {"baseline", "forecast_candidate", "historical_counterfactual", "design_study"}, "unknown scenario intent")
    need(baseline["classification"] == model["classification"] == "synthetic_fixture", "fixture data and model must be marked synthetic")
    need(scenario["baseline"]["dataset_id"] == baseline["dataset_id"], "dataset ID mismatch")
    need(scenario["baseline"]["dataset_sha256"] == digest(baseline_path), "baseline byte digest mismatch")
    need(scenario["engine"]["model_sha256"] == digest(model_path), "model byte digest mismatch")
    need(scenario["baseline"]["reference_year"] == baseline["reference_year"] == scenario["horizon"]["start_year"], "reference year mismatch")
    h = scenario["horizon"]
    need(h["start_year"] <= h["end_year"] and h["step_years"] >= 1, "invalid horizon")
    need(scenario["engine"]["deterministic"] is True and scenario["engine"]["replicates"] == 1, "smoke run must be deterministic single-replicate")
    need(scenario["engine"]["adapter"] == "synthetic_reference_only", "fixture must not claim to be a Symtropy engine run")
    need(len({x["id"] for x in scenario["assumptions"]}) == len(scenario["assumptions"]), "duplicate assumption IDs")
    need(len({x["metric_id"] for x in scenario["requested_metrics"]}) == len(scenario["requested_metrics"]), "duplicate metric IDs")
    for item in scenario["interventions"]:
        need(item["target_id"] == "world.energy-system", "unsupported intervention target")
        need(item["parameter"] == "annual_storage_growth_rate", "unsupported intervention parameter")
        need(item["operation"] in {"set", "add", "multiply"}, "unsupported intervention operation")
        need(math.isfinite(float(item["value"])), "intervention value must be finite")
        need(h["start_year"] <= item["effective_year"] <= h["end_year"], "intervention year outside horizon")
        need("expires_year" not in item or item["expires_year"] >= item["effective_year"], "intervention expiry precedes start")
    for assumption in scenario["assumptions"]:
        need("range" not in assumption or assumption["range"][0] <= assumption["range"][1], "assumption range reversed")

    run_a, run_b = replay(scenario, baseline, model), replay(scenario, baseline, model)
    need(run_a == run_b, "deterministic replay diverged")
    expected = load(FIXTURES / "synthetic_smoke_run.v0.1.json")
    need(run_a == expected, "checked-in output fixture differs from replay output")
    need(run_a["inputs"]["scenario_sha256"] == digest(scenario_path), "run scenario byte digest mismatch")
    need(run_a["output_classification"] == "synthetic_fixture" and run_a["validation"]["status"] == "contract_smoke", "run epistemic classification invalid")
    validate_run_claim_gate(run_a)
    need(all(math.isfinite(x["value"]) for x in run_a["outputs"]), "non-finite output")
    need({x["metric_id"] for x in run_a["outputs"]} == {x["metric_id"] for x in scenario["requested_metrics"]}, "output metric set mismatch")
    duration = h["end_year"] - h["start_year"]
    expected_years = duration // h["step_years"] + 1 + (1 if duration % h["step_years"] else 0)
    need(len({x["year"] for x in run_a["outputs"]}) == expected_years, "output timeline does not match horizon")

    schema_status = "not installed; structural checks only"
    try:
        import jsonschema  # type: ignore[import-not-found]
        from importlib.metadata import version
    except ImportError:
        pass
    else:
        jsonschema.Draft202012Validator.check_schema(schemas["scenario"])
        jsonschema.Draft202012Validator.check_schema(schemas["run"])
        jsonschema.Draft202012Validator(schemas["scenario"]).validate(scenario)
        jsonschema.Draft202012Validator(schemas["run"]).validate(run_a)
        schema_status = f"jsonschema {version('jsonschema')}: both schemas and fixtures valid"

    if args.emit_run:
        print(json.dumps(run_a, indent=2, sort_keys=True))
    else:
        print("PASS: structural scenario/run contract invariants")
        print("PASS: exact-byte SHA-256 bindings for baseline, model and scenario")
        print("PASS: deterministic replay and checked-in run fixture")
        print(f"PASS: {len(run_a['outputs'])} output rows across {len({r['year'] for r in run_a['outputs']})} years")
        print(f"INFO: {schema_status}")
        print("BOUNDARY: synthetic contract smoke only; not an empirical model, Symtropy run, or forecast")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        raise SystemExit(f"FAIL: {exc}")
