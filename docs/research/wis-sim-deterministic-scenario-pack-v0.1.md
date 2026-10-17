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

- JSONL fixture SHA-256: `4eea722417dc14e361fe4786a09f1857980ea92268fda1d5caaaafc35fae3d59`
- Strict schema file SHA-256: `ab9765e6a10c9215a23aa2f3d4e0dc326f4e88dddeaf4cb9e47c37cca5c7ce46`
- Fetched-back structural audit: 28 records, 7 scenario families, 4 variants per family, 7 baseline / 21 adversarial, 28 unique case+variant IDs, 28 unique deterministic seeds, no missing top-level required fields, no undeclared top-level fields, and no missing invariant/observable lists.
- Hashes were recomputed over the fetched-back bytes and match the manifest. No full JSON Schema engine or Symtropy simulation has been run; this is not a simulation pass.
