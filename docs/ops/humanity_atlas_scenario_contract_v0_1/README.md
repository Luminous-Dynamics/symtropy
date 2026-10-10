# Humanity Atlas scenario contract v0.1

**Status:** proposal plus synthetic contract smoke fixture. This directory defines a transport contract; it does not add a production adapter or claim that a calibrated civilization simulator exists.

## Why Symtropy belongs in this system

Symtropy can be the deterministic, replayable simulation substrate for questions with explicit state-transition models: energy balance, physical limits, storage and production throughput, resource constraints, transport, and infrastructure evolution. The existing symtropy-world bridge already separates simulation snapshots from presentation, and its world-scale abstraction gives us a natural place to define simulation boundaries. For the first integration, keep that runtime architecture intact: add an adapter that accepts a versioned scenario input and emits a versioned run envelope. Do not make Sol-Atlas depend directly on private simulator internals.

This contract separates roles:

- **Symtropy:** execute supported, explicit dynamics; preserve seeded reproducibility where promised; report model and run provenance.
- **Symthaea:** propose candidate causal structures, search scenario space, and formulate hypotheses. Suggestions remain proposals until tested against identified models and evidence.
- **Mycelix:** preserve source attribution, claim history, contestation, review and governance receipts. Consensus is not a substitute for empirical validation.
- **Sol-Atlas:** render graph, location, time, branch comparisons and scenario outputs while preserving epistemic classification.

## Contract files

- **schemas/scenario-v0.1.schema.json** — input scenario envelope.
- **schemas/run-v0.1.schema.json** — output envelope.
- **fixtures/earlier_storage_rollout.scenario.json** — a design-study fixture.
- **fixtures/synthetic_grid_baseline.json** and **fixtures/synthetic_grid_model.json** — explicitly synthetic, digest-bound inputs.
- **fixtures/synthetic_smoke_run.v0.1.json** — expected deterministic output.
- **validate_contract.py** — standard-library structural and semantic smoke test; optionally performs full JSON Schema Draft 2020-12 validation when Python's jsonschema package is installed.

## Run the process gate locally

From this directory, run **python3 validate_contract.py**. To exercise fail-closed behavior, run **python3 test_contract_fail_closed.py**. To print the generated result envelope, run **python3 validate_contract.py --emit-run**.

The smoke test verifies JSON parseability, top-level schema envelope shape, exact-byte SHA-256 bindings, valid horizon and intervention ranges, unique IDs, finite outputs, and deterministic replay by running the fixture twice and comparing results. This gate can run independently of GitHub Actions.

The smoke run is purposefully **not** the Symtropy engine. It uses a tiny transparent toy equation solely to prove that an input can be bound to a model/data digest and produce a replayable output envelope. The result is marked **synthetic_fixture**; nothing in it should be cited as real energy-system behavior.

## Trust and validation boundaries

Keep three state dimensions distinct:

1. **Data provenance:** what dataset and exact bytes were used?
2. **Model validity:** what equations, assumptions, units and ranges were used, and against what observations were they tested?
3. **Outcome uncertainty:** how variable are outputs across stochastic draws or model alternatives?

A deterministic run proves repeatability for specified inputs, not that the model represents reality. One run cannot establish uncertainty bounds. A scenario output is not a calibrated probability. **prospective_forecast** should only be emitted by a separately governed forecast workflow with a resolved outcome definition, deadline, calibration history, scoring rule and archived prediction receipt. Historical counterfactuals need sensitivity analysis across alternative causal models; their unseen outcomes are not directly verifiable.

Do not collapse source reliability, causal-edge confidence, forecast probability, normative preference and model completeness into one score. Do not infer that a technology is feasible solely because an unconstrained simulation reaches it: supported models must account for conservation laws, units, resource and energy budgets, manufacturing throughput, deployment lead times and physical constraints relevant to the claim.

## Planned integration sequence

1. **Contract seam (this patch):** versioned schemas, digest-bound fixtures, deterministic local smoke gate.
2. **Symtropy adapter:** translate a supported scenario into engine-native initial state/parameters and return the same run envelope. Preserve deterministic-seed and snapshot/replay semantics. Unsupported variables fail closed rather than being silently ignored.
3. **Sol-Atlas consumer:** render observed facts, estimates, scenario branches and calibrated forecasts differently; expose input/model digests in the detail panel.
4. **Symthaea analysis adapter:** propose scenario variations and causal hypotheses; serialize every intervention and model assumption for replay.
5. **Mycelix evidence workflow:** record source, attribution, dispute, and review receipts; preserve superseded claims rather than erasing their history.
6. **Qualification:** run unit checks locally, then historical hindcasts with time-cutoff data, sensitivity and stress tests, calibration checks, and independent model review for consequential claims.

## v0.1 limitations

- Numeric scalar interventions only (set, add, multiply); categorical and graph mutations require a future schema version.
- Annual integer-year horizon and scalar output series; subannual and spatial field outputs require a future extension.
- No cryptographic signature format is specified yet. SHA-256 identifies exact bytes but does not establish who published them or whether the content is true.
- No probability field is defined in v0.1. Add event-forecast receipts only after the scoring, resolution and calibration contract is specified.
- Schema changes require a new version and fixture migration; do not silently reinterpret old payloads.
