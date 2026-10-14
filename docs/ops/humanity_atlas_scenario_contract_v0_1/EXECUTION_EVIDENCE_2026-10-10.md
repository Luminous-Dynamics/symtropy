# Humanity Atlas execution evidence — 2026-10-10

**Purpose:** preserve what was exercised locally and what remains unverified. This record is not scientific validation.

## Source snapshot

The repository-side contract work is proposed in [PR #1571](https://github.com/Luminous-Dynamics/symtropy/pull/1571). The source state exercised by the local mirror was based on the v0.1 contract, deterministic synthetic runner, evidence-gated run classifications, and failure-injection tests. This evidence packet is added after that execution; its own addition does not retroactively change what was run.

A separate Sol-Atlas consumer is proposed in [PR #79](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/79).

## Commands executed outside GitHub Actions

From the local contract directory:

- \`python3 -m py_compile validate_contract.py test_contract_fail_closed.py\` — passed.
- \`python3 validate_contract.py\` — passed.
- \`python3 test_contract_fail_closed.py\` — passed.

The standard-library smoke script also used jsonschema 4.26.0 to meta-validate both Draft 2020-12 schemas and validate the synthetic scenario and run fixtures.

## Results

- 12 metric rows over 6 years; the checked-in synthetic run matched a fresh replay.
- Exact-byte SHA-256 bindings for the scenario, baseline dataset, and model fixture matched.
- Deterministic repeated outputs were identical.
- Failure-injection tests rejected baseline/model byte tampering, an out-of-horizon intervention, an unsupported intervention parameter, and an invalid time step.
- Classification checks rejected: calibrated label without retrospective evaluation; independent-review receipt without retrospective evaluation; forecast label without a forecast receipt; and calibrated/forecast labels on failed runs.
- JSON Schema tests rejected evidence-free or review-only promotion and a failed run labeled as a prospective forecast.
- Well-formed evidence receipt examples were accepted only as validator-shape tests. Their content and publishers were not authenticated.

## Fixture input digests (SHA-256)

| Artifact | Digest |
|---|---|
| earlier_storage_rollout.scenario.json | \`8eb7cf4fe48bd5ed4c2e3afb29872e5eff9874651f58ca80d7a1247774c5ef1a\` |
| synthetic_grid_baseline.json | \`a976ef2368057b40265ee5b054ee3311bb22000dbac10edd57e3fb9919fe246e\` |
| synthetic_grid_model.json | \`62656555cd45a4395bfba6c73706100572edd4f5aed84eb30164564de1b251c0\` |

## Explicit non-results

- The runner is a tiny transparent synthetic equation; it is not the Symtropy physics engine.
- It does not provide evidence about real storage deployment, energy-system behavior, or future events.
- No historical hindcast or prospective forecast was performed.
- The Rust consumer's unit tests were authored, but could not be compiled/run in the local environment because Cargo and rustc are absent.
- No claim is made that GitHub CI was completed or that its result substitutes for these process gates.
- SHA-256 values bind artifact bytes; they do not prove publisher identity, validity of the contents, or scientific soundness.

## Research references informing process

- NASA, [NASA-STD-7009: Standard for Models and Simulations](https://standards.nasa.gov/standard/NASA/NASA-STD-7009): intended-use acceptance and model/simulation credibility practices.
- NASA Software Engineering Handbook, [Models, Simulations, Tools](https://swehb.nasa.gov/spaces/SWEHBVC/pages/50888929/SWE-070%2B-%2BModels%2BSimulations%2BTools): document the domain, validation metrics/data and results.
- Lee, Merrill and Karger, [ForecastBench-Sim: A Simulated-World Forecasting Benchmark](https://arxiv.org/abs/2606.18686), 2026: simulated futures can create quickly resolved forecasts and paired intervention worlds; performance in a simulated world does not establish transfer to reality.

## Next qualification gate

Do not promote a run from \`unvalidated_simulation\` to \`calibrated_simulation\` until there is a versioned retrospective-evaluation artifact tied to the exact model/input/run digests, an explicitly defined intended-use domain, a cutoff-safe evaluation dataset, declared metrics and baselines, uncertainty/sensitivity reporting, and review of the failure cases. A future forecast must have a frozen, resolvable forecast receipt before its outcome occurs.
