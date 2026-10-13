# HarmonyField runtime adapter: crossing-traffic smoke

## Purpose and scope

This is the first execution adapter for the F/R/E factorial protocol. It runs four bounded-PD-controlled spheres through the actual three-dimensional `symtropy-physics::PhysicsWorld` integration loop with `ConsciousnessField` passed to `step_with_callback`. It sets the real mechanism switches and applies each manifest's source strength and radius to the rebuilt field sources. Every planned run starts with a fresh world.

The task endpoint is the fraction of four agents within two model units of their assigned goal at the terminal tick. The meaningful-effect threshold of 0.25 corresponds to one additional agent completing the task. This is a harness/smoke endpoint chosen to verify the execution/evidence pipeline; it is **not** a validated measure of HarmonyField utility and is not suitable for confirmatory claims.

## Evidence and fail-closed rules

- The adapter accepts one task ID and the frozen solver/time-step/endpoint only. It rejects other dimensions, malformed plans, incomplete factorial grids, noncanonical plan hashes, and configuration-hash mismatches.
- The checkout HEAD must equal the plan's full candidate commit and the source working tree must be clean.
- The artifact directory must be empty and outside the repository before execution. The tracked RNG review note is copied to the artifact root and checked against its expected SHA-256.
- The LCG is consumed only to generate initial coordinate jitter. No RNG is consumed by the integration loop. For each seed, the adapter checks the initial-state digest across all arms and source-strength/radius cells, then executes every configuration twice from a fresh world and compares serialized traces byte-for-byte.
- This only supports **same-seed initial-state blocking** and within-process replay for this deterministic task. It does not claim event-level common random numbers or cross-platform bit identity.
- Every callback-boundary HarmonyField validation failure or non-finite state/metric disqualifies a run. Replay mismatch, missing output, panics, and invalid seed alignment do not produce a completed primary endpoint. Disqualifying a seed alignment retroactively marks every result for that seed unqualified.
- Each result is linked to the exact candidate commit, plan SHA, configuration SHA, raw simulator trace SHA, and replay-trace SHA. The independent manifest validator must still validate the resulting JSONL and artifact root.

The metric named invariant_drift is a **diagnostic**, not a claim of exact energy conservation. It records the maximum positive residual between the measured kinetic-energy change and a discrete controller-work estimate, normalized by the pre-step kinetic energy with a small denominator floor. It excludes no physical loss model and should not be interpreted as a conserved-quantity proof.

## Run procedure

The manifest planner/validator/analyzer comes from the separate HarmonyField manifest-tool change (PR #1597). Run these commands from a clean checkout that contains both this adapter and that tool. Commit SHA binding is intentional: make no source edits after planning.

After the manifest tool is present in the checkout, create a deliberately small pipeline smoke plan with one dimension, one source strength, one radius, one development seed, and two held-out seeds:

\`\`\`bash
python3 scripts/research/harmony_field_ablation.py plan \\
  --candidate-commit "$(git rev-parse HEAD)" \\
  --task-id harmony-field-crossing-traffic-smoke-v1 \\
  --scenario-config scripts/research/scenarios/harmony_field_crossing_traffic_smoke.json \\
  --seeds 101,102,103 --held-out-seeds 102,103 \\
  --dimensions 3 --source-strengths 1.0 --radii 20.0 \\
  --output /tmp/harmony-field-smoke-plan.json
\`\`\`

Use a new, empty artifact directory for each execution. The adapter runs each cell twice; with this example it executes 48 fresh-world runs (24 planned cells, each run twice).

\`\`\`bash
cargo run --locked -p symtropy-consciousness-physics \\
  --example harmony_field_ablation_adapter -- \\
  --plan /tmp/harmony-field-smoke-plan.json \\
  --artifact-root /tmp/harmony-field-smoke-artifacts
\`\`\`

Validate the evidence before any analysis:

\`\`\`bash
python3 scripts/research/harmony_field_ablation.py validate \\
  --plan /tmp/harmony-field-smoke-plan.json \\
  --results /tmp/harmony-field-smoke-artifacts/results.jsonl \\
  --artifact-root /tmp/harmony-field-smoke-artifacts \\
  --report /tmp/harmony-field-smoke-artifacts/validation-report.json
\`\`\`

Only a validated manifest can be analyzed:

\`\`\`bash
python3 scripts/research/harmony_field_ablation.py analyze \\
  --plan /tmp/harmony-field-smoke-plan.json \\
  --results /tmp/harmony-field-smoke-artifacts/results.jsonl \\
  --artifact-root /tmp/harmony-field-smoke-artifacts \\
  --output /tmp/harmony-field-smoke-artifacts/analysis.json
\`\`\`

The two held-out seeds are only enough to exercise the pipeline and catch gross failures; they are not a sample-size rationale, and bootstrap intervals with so few blocks are unstable. Do not publish an effect estimate from this smoke plan as evidence that the model is beneficial. A future confirmatory task needs domain review, pilot-derived power/sample-size planning, prespecified primary utility, expanded held-out blocks, sensitivity coverage, and an independently reviewed task adapter.

## Output files

The artifact root contains `rng-alignment-review.md`, `results.jsonl`, and `runs/<run-id>.trace.json` plus `runs/<run-id>.replay.json` for each successful primary/replay pair. Result records name the byte SHA-256 of both traces. A completed row means the run completed with no observed validation failures/non-finite values and matched its replay; it does not mean the treatment improved performance. The analyzer reports held-out differences and factorial contrasts only after manifest qualification.
