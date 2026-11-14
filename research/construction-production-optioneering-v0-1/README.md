# Construction Production Optioneering Core — v0.1

**Status:** synthetic stage-gate, robust-feasibility and multiobjective Pareto reference. Not a structural designer, safety authority, procurement system or field productivity optimizer.

Parent: [CONSTRUCTION-PRODUCTION-OPTIONEERING-001 #5187](https://github.com/Luminous-Dynamics/symtropy/issues/5187).

## Purpose

Compare different ways of delivering the same declared functional outcome before locking in a building method, equipment fleet and schedule. The first evaluated tier must follow the explicit evidence-bearing order Avoid → Switch → Improve.

## Implemented v0.1

- Homogeneous functional-unit contract with required units, acceptance-contract digest, declared currency, carbon assessment-boundary reference, assessment horizon, hard-gate IDs and fixed-point performance requirements with explicit integer scales.
- Stage decisions bound to accountable owner, project context, evidence references and rationale.
- `Unknown` stops the hierarchy; it never means “no avoidance option exists”.
- Closed/not-applicable stages cannot still contain candidates in that tier; later-stage decisions cannot be predeclared after a tier is open or unknown.
- Closed/no-applicable stages require evidence before moving to the next tier.
- Every candidate binds the same functional contract ID and digest while carrying its own design revision digest. Each scenario's independent acceptance evidence binds both the exact functional-contract digest and exact option design digest.
- Each candidate must provide one result for every frozen scenario; missing/extra scenario results are typed errors.
- Every required scenario must contain independent acceptance, every required hard gate, minimum accepted units and required performance bounds.
- Robust objectives are per-metric worst-case across the full scenario set; no scenario probabilities are accepted or invented. This is a conservative objective vector, not a joint single-scenario prediction.
- Whole-life cost is direct + shared + maintenance + repair + end-of-life cost within the same declared assessment horizon.
- Pareto dominance considers whole-life cost, elapsed time, workflow crew time, energy, carbon, material mass and rework separately.
- Explicit matched pairs can identify a baseline option and report which dimensions changed: design, method, scheduler or fleet. A single-factor controlled contrast changes exactly one dimension; multi-axis contrasts are labelled combined and are not presented as isolated robot or method effects.
- Matched-pair deltas are computed only when both options are robust-feasible under the same functional-unit contract and frozen scenario set. The benchmark reports candidate-minus-reference deltas, so negative cost/time/crew/energy/carbon values represent reductions.
- No weighted scalar trades safety/quality hard gates against cost, speed or carbon.

## Local commands

```sh
cargo fmt --manifest-path research/construction-production-optioneering-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-production-optioneering-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-production-optioneering-v0-1/Cargo.toml --locked --all-targets -- -D warnings
cargo bench --manifest-path research/construction-production-optioneering-v0-1/Cargo.toml --locked --bench frontier_scale
```

## Machine-readable input contract and frozen reference corpus

- JSON Schema: [optioneering-input-v0.1.schema.json](schemas/optioneering-input-v0.1.schema.json), Git blob SHA-1 `1d82c52d65f88e7786df6805ed21b6c37de8339d`.
- Frozen reference input: [reference-floor-matched-contrasts-v0.1.json](fixtures/reference-floor-matched-contrasts-v0.1.json), Git blob SHA-1 `3fc8b2a83ac7051276240e32abc2b76c3e04fe48`.
- The fixture contains one 100 m² homogeneous accepted-output contract, two deterministic scenarios (nominal and outage/rework), and four options: in-situ baseline, scheduler-only improvement, prefabricated method, and prefabricated-plus-robot combined intervention.
- The option profiles identify design, method, scheduler and fleet revisions. The three alternatives explicitly reference the in-situ baseline for matched comparison.
- These are Git blob SHA-1 object IDs, not SHA-256 content hashes. The current Rust crate evaluates typed in-memory structures; a JSON ingestion/Schema-validation adapter is not yet included. External adapters should validate against the published schema before translating to the Rust API.

## Synthetic benchmark harness

The dependency-free frontier benchmark evaluates 10, 100, 500, 1,000 and 2,000 alternatives across two frozen scenarios, with 11 repeated runs and p50/p95/p99 evaluation latency. The Linux CI job runs this harness after metadata/lockfile, format, tests and strict Clippy. These are local evaluator timings only; they do not represent construction-hours, schedule quality or field productivity. The scaling workload deliberately has no matched pairs so its result isolates feasibility/Pareto evaluation overhead from optional attribution work.

## Claim ceiling

This crate evaluates caller-supplied synthetic scenario records and evidence references. It does not authenticate those references, calculate actual life-cycle assessment, establish sensor/inspection accuracy, prove structural or fire compliance, design a building, authorize work or demonstrate construction productivity/cost/carbon savings. All metrics are only comparable inside the exact same functional-unit contract and assessment boundary.
