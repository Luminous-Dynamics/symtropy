# ENERGY-UNCERTAINTY-001: scenario-set omission and reserve-policy fragility v0.1

## Why this is a separate benchmark

A controller can be robust relative to a declared scenario set while the set itself omits the branch that should have forced reserve for a later critical load. Existing out-of-set tests detect unknown observable prefixes. This benchmark additionally tests **counterfactual set completeness**: what decisions and service outcomes change when any subset of the declared training scenarios is removed?

It is not another physical dispatch solver. It is a small exact model-completeness diagnostic that must remain distinct from the separate Sol-Atlas physical/dispatch audit.

## Frozen experiment

- Source panel: `energy-robust-policy-stress-panel-v0.1.json`, four training futures, four held-out stress futures, three intervals.
- Storage: 2 initial/capacity quanta, one discharge quantum per interval.
- Enumerate all `2^4 = 16` training subsets, including empty and full.
- Evaluate all four held-out scenarios under every subset: exactly `16 × 4 = 64` subset/scenario pairs.
- Compare each reduced-set controller with the full-set controller on the *same held-out scenario*.
- Preserve exact action sequences, essential and flexible unserved quanta, observable-prefix misses, first-action changes, source hashes, and per-scenario metric deltas.
- Hash the **raw bytes** of the source and report. Do not hash a re-serialized or normalized representation in place of the file bytes.

## Frozen synthetic observation

The independent local reference lab exhaustively evaluated the subset lattice and produced these results for this fixture:

- 16 subsets and 64 subset/scenario pairs.
- 7 pairs have increased essential-unserved demand relative to the full-set reference.
- 14 pairs change the first action.
- The worst subset increases total held-out essential shortfall from 3 to 4 quanta and worst-case shortfall from 1 to 2.
- The direct counterexample is omission of `early_outage_persists`: the policy no longer reserves against a persistent outage branch and essential service gets worse on the held-out panel.
- Flexible-service deltas are reported separately; this frozen fixture has zero pairs with increased flexible shortfall relative to the full-set reference.

These are finite-panel synthetic observations—not probabilities, independent trials, confidence estimates, or real-site performance.

## Acceptance criteria for the Rust implementation

1. Exact agreement with the reference across all 64 pairs, including action traces and each metric delta.
2. Exactly 16 subsets and 64 subset/scenario pairs; missing, duplicate, unknown or reordered identities fail closed.
3. The full training set has zero action and objective deltas against itself.
4. The empty set fails closed on discretionary flexible service for an unknown prefix.
5. The `early_outage_persists` omission is detected as a critical-service regression and minimized to the smallest explaining subset.
6. Source/report digest mismatch, omitted-row, duplicated-row and altered-source negative controls are rejected.
7. No expected cost, EVPI, VSS, CVaR, confidence intervals, or probability-weighted score is emitted without a justified probability model.
8. Couple resulting schedules to the independent Sol-Atlas dispatch auditor, preserving distinct result fields for information-set validity, scenario-set sensitivity, physical dispatch feasibility, and economics.

## Paradigm shifts

- **Treat the uncertainty set as a versioned model input**, not an unquestioned truth.
- **Make omission regret observable** by comparing every subset to a fixed full-set reference on each same held-out future.
- **Use counterexample-guided scenario curation**: minimize the missing-branch set, add the minimized case to the permanent regression corpus, then rerun.
- **Keep critical service non-compensatory**. Report essential and flexible shortfall separately rather than allowing a weighted scalar to conceal degradation.
- **Fail closed on unknown histories**, but do not mistake conservative behavior for evidence that the registered scenario set is complete.

## Industry comparison ladder

1. Exact finite subset oracle and mutation suite.
2. Larger held-out outage/load/PV scenario families with forecast publication times and correlated failures.
3. Independent physical/dispatch auditing in Sol-Atlas for model-supported constraints.
4. Matched facility sizing/dispatch studies against REopt/SAM-like cases; stochastic/non-anticipative comparisons against PyPSA only when scenario information, model boundary, constraints, horizon and objective semantics match.
5. Report service gaps, feasibility, regret, runtime, peak memory, solver/toolchain versions, and input/output digests separately.

A successful synthetic result establishes only this frozen experiment's algorithmic agreement. It is not a calibrated uncertainty model, physical safety certificate, field result, or claim of superiority over industry tools.

## Implementation status

This commit registers the experiment input and benchmark protocol on a research branch. The Python standard-library companion lab is a local reference artifact only; it is not part of the production path. A Rust port and exact-head CI qualification remain required.
