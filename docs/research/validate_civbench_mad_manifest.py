import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

root = Path(__file__).resolve().parent
schema_path = root / "CIVBENCH_MEASUREMENT_DISTRIBUTION_V0_1.schema.json"
fixture_dir = root / "fixtures"

schema = json.loads(schema_path.read_text(encoding="utf-8"))
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema, format_checker=FormatChecker())

fixture_paths = sorted(fixture_dir.glob("*_MAD_MANIFEST_V0_1.json"))
if not fixture_paths:
    raise SystemExit("No CIVBENCH MAD fixture manifests were found")

evaluation_ids = set()
validated = []

def require_unique(records, key, label):
    values = [record[key] for record in records]
    if len(values) != len(set(values)):
        raise SystemExit(f"{label} values must be unique")

for fixture_path in fixture_paths:
    manifest = json.loads(fixture_path.read_text(encoding="utf-8"))
    errors = sorted(
        validator.iter_errors(manifest),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        details = "\n".join(
            f"{fixture_path.name} {list(error.absolute_path)}: {error.message}"
            for error in errors
        )
        raise SystemExit(f"Manifest does not conform to schema:\n{details}")

    evaluation_id = manifest["evaluation_id"]
    if evaluation_id in evaluation_ids:
        raise SystemExit(f"evaluation_id must be globally unique: {evaluation_id}")
    evaluation_ids.add(evaluation_id)

    require_unique(manifest["sources"], "source_id", "source_id")
    require_unique(
        manifest["evaluation_distribution"]["factors"],
        "factor_id",
        "factor_id",
    )
    require_unique(
        manifest["perturbation_families"],
        "family_id",
        "perturbation family ID",
    )
    require_unique(manifest["metrics"], "metric_id", "metric ID")
    require_unique(manifest["baselines"], "baseline_id", "baseline ID")
    require_unique(
        manifest["negative_controls"],
        "control_id",
        "negative-control ID",
    )
    require_unique(manifest["holdouts"], "holdout_id", "holdout ID")

    for factor in manifest["evaluation_distribution"]["factors"]:
        require_unique(
            factor["levels"],
            "level_id",
            f"{factor['factor_id']} level ID",
        )
        weight_sum = sum(level["target_weight"] for level in factor["levels"])
        if abs(weight_sum - 1.0) > 1e-8:
            raise SystemExit(
                f"{fixture_path.name}: {factor['factor_id']} target weights "
                f"must sum to one; got {weight_sum}"
            )

    design = manifest["replication_design"]
    if design["inference_claimed"] and design["replication_count"] < 2:
        raise SystemExit(
            f"{fixture_path.name}: inferential claims require more than one declared replication"
        )
    if design["method"] == "deterministic-fixture":
        if design["inference_claimed"]:
            raise SystemExit(
                f"{fixture_path.name}: deterministic fixture cannot claim inference"
            )
        if design["replication_count"] != 1:
            raise SystemExit(
                f"{fixture_path.name}: deterministic fixture must declare exactly one execution"
            )

    if manifest["composite_score_policy"] != "no-universal-composite":
        raise SystemExit(
            f"{fixture_path.name}: CivBench does not permit a universal composite score"
        )

    if manifest["construct"]["claim_tier"] == "tier-1-reference":
        if any(
            source["epistemic_status"] != "synthetic-test-fixture"
            for source in manifest["sources"]
        ):
            raise SystemExit(
                f"{fixture_path.name}: tier-1 reference fixture must not silently mix empirical sources"
            )
        if design["inference_claimed"]:
            raise SystemExit(
                f"{fixture_path.name}: tier-1 synthetic reference fixture cannot claim population inference"
            )

    validated.append(manifest["benchmark_id"])

print(
    "CIVBENCH-MAD-001 validated "
    f"{len(fixture_paths)} manifests across {len(set(validated))} benchmark IDs; "
    "schema, weights, identifiers, replication design and claim ceilings passed."
)
