# WIS-SIM deterministic scenario pack v0.1

**Status:** synthetic scenario contract; not a simulation result or human-study evidence.  
**Design issue:** [WIS-SIM-001](https://github.com/Luminous-Dynamics/symtropy/issues/1731).  
**Fixture:** [28 scenarios, JSONL](wis-sim-scenarios-v0.1.jsonl).  
**Manifest:** [pinned hashes, seeds and gates](wis-sim-scenarios-v0.1.manifest.json).  
**Strict schema:** [wis-sim-scenarios-v0.1.schema.json](wis-sim-scenarios-v0.1.schema.json).

## What is new

The first frozen pack has seven scenario families and exactly four deterministic variants for each: one reference case and three adversarial variants. It declares a no-intervention comparator, greedy individual policy, majority-vote policy, and cooperative-deliberation policy. Every case pins a seed, execution profile, policy version, exogenous events, expected invariants, observable quantities, and explicit model limitations.

| Family | Main failure mode | Must not hide |
|---|---|---|
| Commons allocation | Scarcity and heterogeneous needs | Unmet need by agent |
| Contested evidence | Conflicting claims | Provenance and unresolved uncertainty |
| Minority objection | Aggregate decision harms a subgroup | Unresolved dissent and worst-group outcome |
| Repair after conflict | New evidence changes an old decision | Immutable history and repair cost |
| Adversarial authority | High-status false claim | Status-driven false acceptance |
| Distributional regression | Weighted average improves while one group loses | Worst-group regression and missing data |
| Resource shock | Plan becomes brittle after a shock | Recovery, reversibility and reserve failures |

## Paradigm shift

Do not ask the simulator to emit one “wisdom” or “consciousness” number. Treat it as a controlled laboratory for **policy invariants under adversarial perturbations**. Evaluate task success, resource efficiency, uncertainty handling, objection preservation, recovery cost, worst-group results, and replay identity separately.

The simulation can tell us how a specified synthetic system behaves under specified assumptions. It cannot establish how real people experience a practice, whether a tradition is spiritually superior, or whether an intervention reduces loneliness.

## Qualification gates

- All seven scenario families are present with three adversarial variants each.
- Same fixture, policy, seed, engine revision and execution profile produce replay-identical results, or a predeclared numerical tolerance is reported.
- A majority vote never erases a minority objection.
- Source status or repetition never substitutes for evidence.
- New evidence can revise a decision version but never rewrite the historical decision record.
- No aggregate statistic can mask a predeclared subgroup regression.
- Unknown or missing input remains unknown, not a positive claim.
- Outputs expose case-level evidence and identify every violated invariant.
- No global consciousness, enlightenment, virtue, or wisdom score is emitted.

## Limits and next work

This PR only authors and pins scenario input contracts. It does not add a second simulator, validate the policy model, or run an experiment. The next code step is to adapt the existing Symtropy deterministic replay substrate, add a fixture loader and per-invariant checker, and emit a machine-readable report bound to fixture hash, policy version, seed, engine revision and toolchain. After that, compare the four declared policies on the same scenarios and adversarial variants; keep scenario assumptions, simulator correctness, and external validity as separate claims.


## Frozen integrity values

- JSONL fixture SHA-256: `f3dade9be39258ad3d801230af91a212a454cb2272ba72f0f241dfbd4051c0a6`
- Strict schema file SHA-256: `a5e3dc5010c4126e93e3d144dd52a7895fac5314540033f3038fca33565ac6aa`
- Fetched-back structural audit: 28 records, 7 scenario families, 4 variants per family, 7 baseline / 21 adversarial, 28 unique case+variant IDs, 28 unique deterministic seeds, no missing top-level required fields, no undeclared top-level fields, and no missing invariant/observable lists.
- Hashes were recomputed over the fetched-back bytes and match the manifest. No full JSON Schema engine or Symtropy simulation has been run; this is not a simulation pass.


## Structural verifier

A standalone Rust checker now lives at [tools/wis-sim-fixture-verifier](../../tools/wis-sim-fixture-verifier/README.md). It is intentionally not a second simulator. It verifies the fixture/schema digests and selected structural contract: 28 records, seven families, four declared variants per family, deterministic seed uniqueness, policy-set consistency, invariant and observable lists, and model-limit/claim-ceiling fields. It emits `STRUCTURAL_FIXTURE_MATCH_NOT_SIMULATION` and explicitly does not claim full JSON Schema conformance or policy-outcome validity.

The crate has 35 authored tests. They and the targeted GitHub Actions workflow have **not yet produced an executed passing receipt**. Fixture SHA-256 is `f3dade9be39258ad3d801230af91a212a454cb2272ba72f0f241dfbd4051c0a6`; schema SHA-256 remains `a5e3dc5010c4126e93e3d144dd52a7895fac5314540033f3038fca33565ac6aa`.


Variant naming note: the fourth variant is `adversarial-role-or-value-reversal`, because some domains reverse stakeholder roles while others reverse value priorities.


## Executable assertion and report contract

The scenario corpus now carries **144 typed assertions** across its 28 records. The contract requires no scalar wisdom/consciousness metric and binds checks to JSON Pointer paths in either the exact scenario input or the candidate run report. Supported assertion kinds include numeric upper bound, collection ID coverage, field presence, metric equals zero, value equals the scenario input, boolean true, and field absence.

- [Strict run-report schema](wis-sim-run-report-v0.1.schema.json), SHA-256 `cc3e80c4c140062c0ad76b318693c1a5feb13e5686bcd86c8be9ee10a45e47b8`.
- The report binds corpus and schema digests; scenario + variant; policy ID/version; seed; execution profile; engine revision; toolchain profile; target triple; environment fingerprint; and an exact input-line digest.
- The Rust verifier's `verify-run` path recomputes each typed assertion from report data and scenario inputs, checks the runner's declared per-assertion status against its own evaluation, and emits a failure/unknown status if a hard assertion fails or is unresolved. A runner cannot claim pass merely by putting "pass" in its report.
- The verifier now has **35 authored Rust tests**, including synthetic report fixtures across all 28 scenario/variant records, wrong-seed rejection, an intentionally violated resource bound, false-pass rejection, and forbidden scalar-score detection.

These 35 tests have **not been compiled or executed**. The synthetic report objects are verifier unit-test inputs, not outputs of the Symtropy simulator. This adds an executable evaluation contract; it does not yet execute or validate any policy model.

## Existing substrate boundary

See the [source-audit memo](wis-sim-existing-substrate-audit-v0.1.md) and [machine-readable substrate map](fixtures/wis-sim-substrate-map-v0.1.json). The physics replay tape is physics-command replay, not a governance log; the current Village demo's expected stewardship decrease is written into its own penalty function; and `CognitiveBrain::phi()` currently aliases `epistemic_integrity`. Issue [WIS-SIM-002 / #1798](https://github.com/Luminous-Dynamics/symtropy/issues/1798) tracks that repair separately. No scenario engine or external-validity claim is inferred from these source findings.
