import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

root = Path(__file__).resolve().parent
schema_path = root / "CIVBENCH_MEASUREMENT_DISTRIBUTION_V0_1.schema.json"
fixture_path = root / "fixtures" / "HUMSIM_SYNTHETIC_MAD_MANIFEST_V0_1.json"

schema = json.loads(schema_path.read_text(encoding="utf-8"))
manifest = json.loads(fixture_path.read_text(encoding="utf-8"))

Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema, format_checker=FormatChecker())
errors = sorted(validator.iter_errors(manifest), key=lambda error: list(error.absolute_path))
if errors:
    details = "\\n".join(
        f"{list(error.absolute_path)}: {error.message}" for error in errors
    )
    raise SystemExit(f"Manifest does not conform to schema:\\n{details}")


def require_unique(records, key, label):
    values = [record[key] for record in records]
    if len(values) != len(set(values)):
        raise SystemExit(f"{label} values must be unique")


require_unique(manifest["sources"], "source_id", "source_id")
require_unique(manifest["evaluation_distribution"]["factors"], "factor_id", "factor_id")
require_unique(manifest["perturbation_families"], "family_id", "perturbation family ID")
require_unique(manifest["metrics"], "metric_id", "metric ID")
require_unique(manifest["baselines"], "baseline_id", "baseline ID")
require_unique(manifest["negative_controls"], "control_id", "negative-control ID")
require_unique(manifest["holdouts"], "holdout_id", "holdout ID")

for factor in manifest["evaluation_distribution"]["factors"]:
    require_unique(factor["levels"], "level_id", f"{factor['factor_id']} level ID")
    weight_sum = sum(level["target_weight"] for level in factor["levels"])
    if abs(weight_sum - 1.0) > 1e-8:
        raise SystemExit(
            f"{factor['factor_id']} target weights must sum to one; got {weight_sum}"
        )

design = manifest["replication_design"]
if design["inference_claimed"] and design["replication_count"] < 2:
    raise SystemExit("inferential claims require more than one declared replication")
if not design["inference_claimed"] and design["method"] == "deterministic-fixture":
    if design["replication_count"] != 1:
        raise SystemExit("a deterministic fixture must declare exactly one execution")

if manifest["composite_score_policy"] != "no-universal-composite":
    raise SystemExit("CivBench does not permit a universal composite score")

if manifest["construct"]["claim_tier"] == "tier-1-reference":
    if any(source["epistemic_status"] != "synthetic-test-fixture"
           for source in manifest["sources"]):
        raise SystemExit("this reference fixture must not silently mix empirical sources")
    if manifest["replication_design"]["inference_claimed"]:
        raise SystemExit("tier-1 synthetic reference fixture cannot claim population inference")

print("CIVBENCH-MAD-001 schema, synthetic fixture, factor weights, identifier uniqueness and claim ceiling validated.")
