# WPT bounded exact multi-objective dispatch oracle v0

This package is the small-instance objective oracle under [WPT robust dispatch #3559](https://github.com/Luminous-Dynamics/symtropy/issues/3559). It complements the exact max-flow service-feasibility oracle in [PR #3828](https://github.com/Luminous-Dynamics/symtropy/pull/3828).

## Paradigm shift: compare the Pareto set, not a weighted score

A single weighted objective can hide receiver starvation or silently exchange useful energy for lower stress. This oracle exhaustively enumerates a deliberately small integer feasible set and returns every non-dominated point across four separately reported objectives:

1. Total useful receiver energy delivered — maximize.
2. Worst receiver request-satisfaction ratio — maximize.
3. Jain fairness of request-normalized satisfaction — maximize.
4. Asset-stress surrogate units — minimize.

Each frontier point carries the receiver-level outcome and a canonical link-allocation witness. No scalar weighted score is used to collapse those trade-offs.

## Joint constraints

Every candidate must satisfy all declared constraints simultaneously:

- source/site deliverable-energy budget by interval;
- segment deliverable-energy budget by interval;
- per-receiver acceptance by interval;
- individual receiver requested-energy upper bound and minimum-service floor;
- every declared shared linear resource envelope, with integer units-per-energy-quantum coefficients;
- receiver arrival/departure windows.

The shared-resource interface can represent linearized shared converter, thermal or coupling budgets where a validated physical reduction exists. It does **not** prove that nonlinear, state-dependent or non-convex multiport physics are representable by these linear resource groups. Such physics require an appropriate exact MILP/MIQP or nonlinear oracle.

## Bounded exactness

- All energy is integer mWh and must be divisible by the scenario's declared `energy_quantum_mwh`.
- At most 8 transfer links, 8 receivers and 20 total requested energy quanta are admitted.
- Enumeration is capped at 5,000,000 search nodes and the Pareto output at 256 points.
- Exceeding either bound returns an error; the oracle never silently truncates or returns an approximate frontier.
- Input link ordering is canonicalized by link identity, and equal objective points retain a deterministic lexicographically smallest witness.

## Known-truth fixture

The two-receiver fixture has 10 mWh requested, 8 mWh source capacity, two segment capacities, and one shared segment-A coupling-envelope budget. It produces 89 feasible integer allocations and a 6-point Pareto frontier.

Two interpretable frontier endpoints are:
- Lower-stress solution: 8 mWh total, worst satisfaction 0.60, Jain fairness 0.941176, stress 8 units.
- Maximum-fairness solution: 8 mWh total, worst satisfaction 0.80, Jain fairness 1.0, stress 11 units.

Neither point dominates the other. This is the intended output: expose the actual trade-off and witnesses rather than assert one weighted optimum.

## Validation

`cargo test --locked -p wpt-dispatch-pareto-oracle`

The 11 Rust tests cover the hand-computable frontier, shared-resource enforcement and witnesses, impossible individual service floors, energy-quantum integrity, unknown resources, half-open departure deadlines, duplicate IDs, unknown fields, synthetic-only evidence, input-row permutation invariance and pairwise frontier non-dominance.

## Claim ceiling

This is an exact oracle for bounded synthetic integer-capacity instances and the explicitly declared linear shared-resource constraints only. It does not infer physical coupling, validate a multiport model, prove robust feasibility under uncertainty, replace nonlinear AC/EMT analysis, establish safety/EMC compliance or authorize energization. Synthetic algorithm parity is not measured field WPT performance.

### Complete exact frontier

The six objective vectors (total delivered mWh, worst satisfaction ppm, Jain fairness ppm, stress units) are:

| Total mWh | Worst satisfaction | Jain fairness | Stress units |
|---:|---:|---:|---:|
| 8 | 800,000 | 1,000,000 | 11 |
| 8 | 600,000 | 941,176 | 8 |
| 7 | 600,000 | 980,000 | 7 |
| 6 | 600,000 | 1,000,000 | 6 |
| 5 | 400,000 | 961,538 | 5 |
| 4 | 400,000 | 1,000,000 | 4 |

Satisfaction is quantized to parts per million before Jain's index is calculated. These are exact outputs for the integer fixture, not estimates of real vehicle service or physical asset stress.

The reference count was corrected after a separate enumeration: there are 89 feasible link-allocation vectors, not 122; the frontier has six unique objective vectors, not seven. The test also checks pairwise non-dominance and invariance under reordering receivers, segment slots, receiver slots, shared resources and links.

