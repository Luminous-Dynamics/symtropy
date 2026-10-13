# HarmonyField ablation manifest tool

This standard-library Python tool creates a deterministic run plan and validates a supplied JSONL result manifest for the HarmonyField F/R/E factorial experiment.

**It does not execute simulations, calculate treatment effects, or prove that HarmonyField is useful.** A plan is not a result. Only records produced by an actual run harness should be put into the result file.

## 1. Prepare a scenario contract

Create a JSON file whose primary endpoint, units, direction, decision-relevant effect threshold, task description, solver, and timestep are chosen before confirmatory runs.

Example structure (illustrative only; replace these values with a real, reviewed scenario):

    {
      "task_description": "REPLACE_WITH_A_PREDECLARED_SIMULATION_TASK",
      "primary_endpoint": {
        "name": "REPLACE_WITH_PRIMARY_METRIC",
        "units": "REPLACE_WITH_UNITS",
        "direction": "higher_is_better",
        "meaningful_effect_threshold": 0.05
      },
      "solver_and_timestep": {
        "solver": "REPLACE_WITH_EXACT_SOLVER_AND_VERSION",
        "dt": 0.016,
        "max_steps": 1000
      }
    }

Do not use this illustrative threshold or timestep merely because they appear here. They must suit the task and selected solver. Include all other scenario inputs in the same file so the file hash identifies the declared scenario configuration.

## 2. Generate a plan

    python3 scripts/research/harmony_field_ablation.py plan \
      --candidate-commit FULL_40_CHARACTER_COMMIT_SHA \
      --task-id YOUR_PREDECLARED_TASK_ID \
      --scenario-config scenario.json \
      --seeds 11,12,13,14,15 \
      --held-out-seeds 15 \
      --dimensions 2,3 \
      --source-strengths 0.5,1.0 \
      --radii 5.0,10.0 \
      --output plan.json

The seed values and grid above are syntax examples, not recommended experimental values. Choose a pilot-supported seed count and defensible parameter ranges before evaluation. The tool rejects a plan without at least one development seed and one held-out seed.

The run count is:

    number_of_seeds × dimensions × strengths × radii × 8 factorial_arms

Each planned run includes a canonical configuration SHA-256 and a run ID derived from it. The plan has a separate hash. Canonical JSON here means sorted object keys, compact separators, UTF-8 encoding, and rejection of NaN/Infinity as JSON numbers.

The current kernel exponent rule is recorded as max(dimension - 1, 1). The tool does not pretend that exponent is a tunable parameter.

## 3. Run the simulations

A simulation adapter is still required. It must consume each planned configuration, instantiate a fresh identical scenario for each run, apply the arm's three independent toggles, and write one result record per planned run. The adapter must capture actual output files/traces and their SHA-256 hashes. Do not manually label jobs as completed from a plan alone.

Each result line must include the following minimum fields:

    {
      "schema_version": "luminous.harmony-field-ablation-result.v1",
      "run_id": "COPY_FROM_PLAN",
      "candidate_commit": "FULL_40_CHARACTER_COMMIT_SHA",
      "configuration_sha256": "COPY_FROM_PLAN",
      "run_status": "completed",
      "termination_reason": "RECORD_ACTUAL_REASON",
      "primary_endpoint": 0.75,
      "secondary_metrics": {"collision_count": 2},
      "validation_failure_count": 0,
      "nonfinite_count": 0,
      "invariant_drift": 0.001,
      "artifact_sha256": "ACTUAL_64_CHARACTER_ARTIFACT_SHA256",
      "replay_status": "exact_match",
      "replay_evidence_sha256": "ACTUAL_64_CHARACTER_REPLAY_EVIDENCE_SHA256"
    }

The values above are placeholders for the schema only, not measured Symtropy results. Failed/skipped/cancelled/unqualified runs must also be recorded, with null primary_endpoint, and must never be silently removed. If replay has not been checked, use not_tested; such a manifest cannot qualify.

## 4. Validate results

    python3 scripts/research/harmony_field_ablation.py validate \
      --plan plan.json \
      --results results.jsonl \
      --report validation-report.json

A qualified manifest requires every planned record, complete eight-arm/grid coverage, matching configuration identities, completed runs, zero validation/non-finite counters, valid artifact hashes, and recorded replay evidence. Missing records, unexpected records, duplicate records, malformed data, or unaccepted replay statuses fail closed.

Exit code 0 means the manifest meets these mechanical checks only. Exit code 1 means results are not qualified; exit code 2 means the plan/input is invalid. Even a mechanically qualified manifest is **not** a utility analysis. A separate, predeclared paired/factorial effect analysis with uncertainty intervals is still required.

## 5. Built-in checks

    python3 scripts/research/harmony_field_ablation.py self-test

These tests cover arm enumeration, deterministic hashing, plan repeatability, detection of omitted factorial configurations, and failure on missing or malformed run records.
