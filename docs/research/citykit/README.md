# CityKit synthetic corpus v0.1

Parent: Symtropy [CITYKIT-SYNTH-001 #3768](https://github.com/Luminous-Dynamics/symtropy/issues/3768) and Sol-Atlas [CITY-MAKER-001 #549](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/549).

## Status

This is a candidate synthetic fixture set for implementation and independent-oracle review, not a measured city, supplier, utility, factory or robot data set. All numeric values are deliberately synthetic and cannot be used as project estimates or engineering inputs.

The corpus contains 18 bounded cases covering service capacity/evidence, utility/resource balance, AND/OR productive routes, unseeded cycles, imported versus local production, shared workcell capacity, supplier failure and alternate routes, stale/revoked evidence, and incompatible units.

## Semantics

- A route's `requires` list is conjunctive (AND).
- Routes under one capability are alternatives (OR).
- A required ID resolves to a declared primitive or another declared capability. A missing required record is Unknown, not unavailable.
- A primitive has an origin (`local` or `imported`), availability (`available` or `unavailable`) and evidence state.
- An imported primitive can support operation, but never local reproductive closure.
- A local replacement component is not proof that the productive machine can be reproduced.
- Capability dependency cycles without a reachable primitive seed do not bootstrap themselves.
- Service capacity is supported only by evidence in the declared current scope.
- Resource balances require compatible units and satisfy `available >= allocated + reserve`.
- Work-package time intervals are half-open `[start_tick, end_tick)`; simultaneous demand is summed against declared capacity.
- Stale, revoked, unknown and missing evidence cannot be promoted to favorable states.
- Dispositions remain distinct: `FeasibleForSyntheticCase`, `Infeasible`, `Unknown`, `Unsupported`.

## Qualification boundary

The `expected` fields are test-vector expectations, not authority. The implementation must recompute disposition and reason codes from `input` using a separately authored reference oracle. The production evaluator must not read expected results to decide its own disposition. The differential harness compares independently computed outputs against these expectations.

Proposed hostile controls to add in the evaluator test harness: duplicate IDs, dangling references, missing units, semantically irrelevant input permutations, tampered digests and expected-label substitution.

Before claiming corpus qualification, resolve schema ambiguities, run exact-head tests and independently review every expected output. A successful JSON parse is not evaluator validation.
