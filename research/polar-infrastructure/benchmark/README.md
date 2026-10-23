# Polar resilience scorer v0

**Evidence ceiling: synthetic-only. Not a physical simulator, site assessment, safety case, or controller certification.**

This small Rust crate scores already-produced time-series outcomes for a deliberately tiny synthetic episode. It does not generate dispatch decisions, model thermodynamics/electrical dynamics, model actual polar weather, or recommend equipment. The four policy traces in the fixture are hand-authored examples for checking metric calculations and hard-gate behavior; they are not benchmark results.

## Paradigm shift: compare delivered services, not nominal asset uptime

A generator, battery, wind turbine, or network can report available while residents still lack heat, potable water, communications, or sanitation. The evaluation unit here is a declared **service contract over an episode**: required service, an owner-specified minimum fraction, a criticality weight, and delivered service at each time step. The scorer reports each policy's outcomes separately and never lets lower fuel use compensate for a service-floor breach.

The service weights and minimum fractions in the fixture are invented for software tests. They are not engineering thresholds. A real study must obtain limits from the accountable system owner and qualified domain reviewers, justify units and measurement boundaries, and use a site-specific design basis.

## Run

From the repository root, with a Rust toolchain and dependency access:

\`\`\`sh
cargo test --manifest-path research/polar-infrastructure/benchmark/Cargo.toml
cargo run --quiet --manifest-path research/polar-infrastructure/benchmark/Cargo.toml -- research/polar-infrastructure/benchmark/fixtures/toy-episode.json
\`\`\`

The CLI emits a JSON report. Invalid input fails closed: unknown service identifiers, incomplete step series, duplicate policy/service identifiers, negative/non-finite measurements, absent demands, and unsupported evidence claims are errors.

## Metrics

For each declared service s at step t:

- served units are capped at required demand; over-delivery cannot inflate the score;
- criticality-weighted service fraction is weighted delivered units × timestep divided by weighted demanded units × timestep;
- weighted unmet service is weighted unmet units × timestep;
- floor-breach service-hours sum timestep duration for each demanded service below its fixture-declared floor;
- gate_pass is false if any service-floor breach occurs;
- fuel intensity is total declared fuel units divided by criticality-weighted delivered service-hours; it is undefined if no service is delivered.

All outputs remain a vector. The crate does not collapse continuity, unmet demand, fuel, and admissibility into one score, does not choose a winner, and does not infer life-safety status from a floor.

## Benchmark protocol before making SOTA claims

1. **Freeze the design basis:** geography (Arctic land, Antarctic station, and mobile ice platforms stay distinct), service contracts, units, episode boundaries, data versions, thresholds, missing-data policy, and hard gates.
2. **Pre-register the experiment:** primary outcomes, candidate and baseline families, scenario splits, tuning budgets, compute budgets, retries, exclusions, and uncertainty method before looking at candidate results.
3. **Use fair comparison arms:** documented incumbent; transparent rules baseline; LP/MILP or model-predictive control where a valid model supports it; robust/stochastic optimization where uncertainty is characterized; proposed controller; and ablations. Include defective controls to prove the harness detects known failures.
4. **Hold out conditions:** do not tune on final storm, cold-start, icing, prolonged resupply delay, correlated component failure, communications loss, and sensor-drift episodes. Use only episodes with a defensible data/model basis.
5. **Report a frontier:** critical-service delivery and shortfall; floor breaches; recovery time; fuel and resupply mass; water/sanitation continuity; lifecycle cost/emissions; repair time/spares burden; runtime, memory and platform energy; and uncertainty. Publish per-episode results and tail outcomes, not only an average.
6. **Advance evidence in stages:** scorer/fixture tests → deterministic model tests → calibrated scenario sweeps and sensitivity analyses → software/power/controller HIL as applicable → independently reviewed site data. Synthetic scorer success proves only software behavior.
7. **Fail closed on claims:** queued/skipped checks are not passes. Standards are references for designing tests, not badges of compliance. A standard-conformance claim requires the appropriate suite, a justified test boundary, and qualified review.

Industry anchors for protocol design:
- [IEEE 2030.8-2018: Standard for the Testing of Microgrid Controllers](https://standards.ieee.org/ieee/2030.8/6169/) and the [active P2030.8 revision project](https://standards.ieee.org/ieee/2030.8/12543/).
- [NREL/NLR, Hardware-in-the-Loop Test Bed and Test Methodology for Microgrid Controller Evaluation](https://www.nrel.gov/docs/fy20osti/71338.pdf).
- [MLCommons benchmark programme](https://mlcommons.org/benchmarks/) for fair, reproducible comparisons.
- [UFC 3-130-05 Arctic and Subarctic Utilities (2025)](https://www.wbdg.org/FFC/DOD/UFC/ufc_3_130_05_2025.pdf) for cold-region engineering context.

These are method/design references, not evidence that this crate is compliant with them or that any policy is better than state of the art.
