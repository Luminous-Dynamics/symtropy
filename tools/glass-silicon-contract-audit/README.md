# Glass / Silicon Contract Audit

A small Rust reference verifier (dependency versions pinned and lockfile included) for the GLASS-SILICON-001 benchmark.

It audits the **evidence contract** for the checked-in synthetic benchmark corpus and checks elemental mass-balance arithmetic on an explicitly declared boundary. It currently defines 63 Rust unit tests (31 original contract/material/hash tests, 6 one-step decision-value tests, 8 adaptive-campaign tests, 8 staged-feasibility-gate tests, 1 integrated feasibility-gated campaign test, and 9 grouped-split leakage tests); their count is not a statement that the current exact-head suite has executed. It is not a glass-property predictor, thermochemical solver, furnace digital twin, lifecycle assessor, product certification tool, or hardware controller.

## Commands

From the repository root:

```sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-corpus \
  docs/tech/glass-silicon-process-benchmark-v0.1.schema.json \
  docs/tech/glass-silicon-process-benchmark-v0.1.json

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-feasibility \
  tools/glass-silicon-contract-audit/fixtures/glass-feasibility-gate.synthetic.json

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-grouped-split \
  tools/glass-silicon-contract-audit/fixtures/grouped-split-manifest.synthetic.json
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-decision-value \
  tools/glass-silicon-contract-audit/fixtures/decision-value-pool.synthetic.json
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-adaptive-campaign \
  tools/glass-silicon-contract-audit/fixtures/adaptive-decision-pool.synthetic.json

# Same gate-aware fixture: compare myopic and adaptive decision policies
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-decision-value \
  tools/glass-silicon-contract-audit/fixtures/adaptive-decision-pool-feasibility-gated.synthetic.json
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-adaptive-campaign \
  tools/glass-silicon-contract-audit/fixtures/adaptive-decision-pool-feasibility-gated.synthetic.json
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- check-mass \
  tools/glass-silicon-contract-audit/fixtures/mass-balanced.synthetic.json

# Run only after permitted local acquisition of the source files; this command does not download or admit them.
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- hash-artifacts \
  docs/tech/glass-silicon-process-benchmark-v0.1.json \
  scale-zenodo-18430657-v1 /path/to/lawfully-acquired-files
```

Exit codes: 0 = checks pass at the declared contract/arithmetic level; 2 = failed contract/arithmetic; 3 = unresolved or insufficient evidence; 64 = command/usage error. Reports are deterministic JSON with sorted element maps and stable finding codes. Run the CLI with `--locked`; Cargo.lock pins the standalone verifier's resolved dependency graph.

## Local artifact hash receipt

`hash-artifacts` takes a corpus manifest, artifact ID and caller-supplied local directory. It rejects path traversal and symlinks escaping the supplied directory, computes SHA-256 for each listed file, and compares the locally computed MD5 with the legacy checksum published in the source manifest. If a SHA-256 has already been pinned, it compares that too.

The command never downloads data, changes the corpus manifest, resolves a license, or marks an artifact admitted. A successful report uses `HASHED_PENDING_ADMISSION` and lists the computed SHA-256 values for review. Update the manifest only after exact artifact identity and permitted use have been independently reviewed. MD5 is only legacy checksum corroboration; it is not secure authentication. Never claim reproducibility from a public record listing alone.

## Mass ledger contract

Inputs and outputs are elemental masses in the *same declared unit*. Inventory change is final minus initial boundary inventory. For each element:

`residual = input - output - inventory_change`

The record must declare a complete stream census and element-specific tolerances accompanied by per-element justification metadata; justification text is not independently verified by this tool. Missing elements, incomplete census, unknown evidence status, or missing tolerance yield `UNRESOLVED`; a negative input/output quantity or residual outside tolerance yields `FAIL`. Negative inventory change is permitted because it can represent inventory drawdown.

The result proves only that the supplied arithmetic fits the supplied contract. It cannot prove that the stream census is physically complete, that an instrument is calibrated, or that a material was produced. Those require independent evidence.

## Claim ceiling

The checked-in fixtures are synthetic software tests. They are not plant data, recipes, measured energy consumption, qualified glass, or silicon production results. Full JSON Schema validation, source-data reproduction, and physical-process validation remain separate gates.


## Synthetic decision-value pool

The `audit-decision-value` command evaluates a fully enumerated **synthetic** scenario/action pool. It computes the best robust action before measurement, the post-measurement worst-case regret for each observation partition, gross regret reduction, eligibility under separate cost and duration caps, and a Pareto frontier. Its exact one-step oracle is evaluator-side and chooses only among the enumerated budget-feasible measurements.

The fixture includes an informative measurement, an uninformative measurement and an informative-but-over-budget measurement. Unit mutation tests reject a missing cost, producer-access leakage declarations, feasibility-control bypasses, and incomplete observation maps. The producer-access values are a declared contract, **not proof that a real producer process is isolated**. All utility values, scenarios, costs and feasibility controls in this fixture are synthetic; this command is not a materials model, physical value-of-information estimate, experiment authorization, or evidence of industrial gains.

Exit codes retain the tool convention: 0 = PASS at the declared contract/arithmetic level; 2 = FAIL; 3 = UNRESOLVED; 64 = usage error.


## Adaptive multi-step synthetic campaign

The `audit-adaptive-campaign` command runs an exhaustive small-instance oracle over an explicitly synthetic finite pool. Unlike the one-step evaluator, the policy may select a second unused measurement separately within each first-observation branch. The exact planner minimizes worst-case regret, then maximum cumulative measurement cost, duration and measurement count. Each path must obey the same independent cost and duration caps; measurements cannot be reused within a path.

The oracle is deliberately bounded to at most 32 scenarios, 12 available measurements and 8 measurement steps. Exhaustive search is for a tiny correctness reference only, not a production optimizer. The committed fixture has two crossed, ambiguous measurements: neither alone reduces worst-case regret, but both together reveal enough to choose a feasible scenario-specific route. Its known answer is pre-measurement regret 6, terminal regret 0, gross regret reduction 6, maximum path cost 2 units, maximum duration 20 seconds, and two measurements along each worst path.

The planner is deterministic under fixed input and tie-break rules. A separate frozen negative fixture proves the solver returns UNRESOLVED when no robust baseline action exists and the budget cannot fund the complementary pair. A drift/failure stress fixture expands hidden worlds across clean, misclassified and invalid primary-instrument states; it tests a lower-cost primary measurement followed by a secondary measurement on every ambiguous/failure branch. This is set-based robustness, not a probability model. The suite does not represent real material properties or measured physical experiment cost.


### Sensor-drift / invalid-measurement stress fixture

Run:

```sh
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-adaptive-campaign \
  tools/glass-silicon-contract-audit/fixtures/adaptive-decision-pool-drift-failure.synthetic.json
```

The synthetic scenario pool includes valid, drifted/misclassified and invalid primary-measurement states. The one-step policy cannot reduce worst-case regret below 6 synthetic units. The adaptive plan uses the primary test first, stops after an unambiguous signal, and requests the secondary test only on the mixed or invalid branches; its terminal worst-case regret is 0 at maximum path cost 4 synthetic units / 20 synthetic seconds. A perfect-information option is excluded because it exceeds both caps. No scenario probabilities are assigned; the expanded rows are possible worlds, not a statistical distribution. A Rust unit test and workflow known-answer assertions compare the one-step and adaptive results.

### Lockfile maintenance and reproducible CI

The standalone verifier is tested with the repository-pinned Rust toolchain (1.96.0) and `--locked`. If `Cargo.toml` changes, regenerate `Cargo.lock` locally or in a dedicated diagnostic job using that exact toolchain, review the dependency diff, commit the generated lockfile, and then verify `cargo test --locked`. Do not regenerate dependencies inside the primary contract-audit workflow: a stale lockfile should fail closed so the dependency graph is reviewable and reproducible. The lockfile's presence does not establish that the physical source data or industrial process has been validated.



The 54-test implementation and feasibility-gated campaign passed the complete focused workflow at `784d02cc0be13519e467da455b3236cad779643a` ([run](https://github.com/Luminous-Dynamics/symtropy/actions/runs/40041754485)). The branch now adds an executable grouped-split manifest auditor and nine tests; the full 63-test suite remains unqualified until the latest exact-head workflow passes. Earlier passing runs do not qualify later code. This tooling checks declared split assignments, not the full training pipeline or physical glass performance.

## Staged feasibility gate

`audit-feasibility` runs the frozen synthetic staged-gate corpus. Every candidate must declare the same five ordered gates: composition validity, glass formation, process window, joint product qualification, and manufacturing feasibility. A candidate is `FEASIBLE` only when every gate is `PASS`; any evidenced hard-gate `FAIL` makes it `INFEASIBLE`; with no hard failure but any `UNKNOWN`, `NOT_ATTEMPTED`, `CENSORED`, `STALE`, or `OUT_OF_DOMAIN` gate it returns `UNRESOLVED`.

The contract rejects missing/extra gates, unknown statuses, untraceable `PASS`/`FAIL` declarations, and arbitrary fields (including trying to attach a numeric zero to an unknown gate). The six checked-in controls cover a feasible candidate, a confirmed non-forming candidate, a mandatory-property failure, unknown measurement evidence, out-of-domain input and stale process-window evidence. Eight mutation tests check that unsupported passes, untraceable hard failures and incomplete contracts fail closed.

These controls test contract semantics only. They do not predict glass-forming ability, establish that a recipe can be manufactured, or validate real source data. For the scientific protocol and current GFR baselines, see [GLASS-FEASIBILITY-GATE-001](../../docs/tech/GLASS_FEASIBILITY_GATE_BENCHMARK_V0_1.md). The protocol separates the March 2026 cross-family XGBoost/SHAP GFR study (DOI 10.1111/jace.70616) from July 2026 GFRI-ML with explicit non-glass-forming oxide negatives (DOI 10.1111/jace.70980), and from PNNL's waste-glass multi-property validation. Reproduce each task/split independently; data/code use still requires rights and digest review.

## Gate-conditioned adaptive campaign

The fixture `adaptive-decision-pool-feasibility-gated.synthetic.json` tests a two-stage decision policy. A cheap formation screen costs 1 synthetic unit / 2 seconds; a downstream route characterization costs 3 units / 10 seconds. The one-step oracle has worst-case regret 10 before and after its best single measurement (gross regret reduction 0). The adaptive oracle screens first, stops with the safe `reject` action for the non-forming case, and pays for route characterization only on the forming branch. Its terminal worst-case regret is 0, with maximum path cost 4 units, duration 12 seconds and two measurements.

There are no scenario probabilities. This is not an expected-cost estimate or a plant-savings claim; it demonstrates that the policy tree can gate expensive follow-up actions on earlier evidence. CI asserts both reports and the exact branch structure. The integrated feasibility-gated campaign is test 54; grouped-split auditing adds nine tests, including evidence-class preservation, for a current total of 63. See the exact-head run before describing the current 63-test suite as qualified.

## Grouped split leakage audit

`audit-grouped-split` checks an assignment-only manifest whose records contain only a stable record ID, split name and declared grouping keys. The fixture uses composition-family and synthesis-campaign IDs and must keep every group inside exactly one of train/validation/test. It fails on group overlap across splits, duplicated IDs, missing group keys, unknown splits, missing partitions, non-string grouping-field declarations, or a label/feature field added to the assignment manifest. It preserves the declared evidence class but does not independently validate that class, data rights, or data admission. Sharing a group inside one split is allowed.

The command reports the SHA-256 fingerprint of the exact raw manifest bytes; CI independently checks it against `sha256sum`. This identifies the assignment file but is not an authenticity proof or evidence of data rights/admission. The auditor is a leakage-contract preflight, not proof that downstream feature engineering, embeddings, preprocessing, tuning or model selection are leakage-free. Choose grouping variables appropriate to each lane and retain the digest in the external result packet.
