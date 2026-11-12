# High-rise reference-floor local validation record v0.1

**Status:** local fixture and witness checks completed; full schema-validator, Rust integration and hosted CI remain pending.

## Exact inputs

- Fixture: `docs/research/construction/HIGHRISE_REFERENCE_FLOOR_SCHEDULE_V0_1.json`
- Fixture Git blob SHA-1: `25862337341f4c5ceb216e24bc49f6926082c1c6`
- Schema: `docs/research/construction/HIGHRISE_REFERENCE_FLOOR_SCHEDULE_SCHEMA_V0_1.json`
- Schema Git blob SHA-1: `48ca954575ab43d7d99bccba7d44e50ddb40b4f8` (exact schema bytes fetched after the schema correction)
- Generic #1902 adapter fixture: `docs/research/fixtures/HIGHRISE_REFERENCE_FLOOR_MICRO_ORACLE_V0_1.json`
- Adapter Git blob SHA-1: `9db3b46b8eb81d302f44b9bac9a515d4206dae23`
- Branch: `research/highrise-reference-floor-v0.1`
- Date: 2026-11-12

The blob identifiers pin the exact Git objects fetched during this local check; they are not content SHA-256 digests.

## Per-case local check

| Case | Nominal makespan | Upper-duration makespan | Precedence orders enumerated | Expected outcome |
|---|---:|---:|---:|---|
| HF01 — one abstract hoist | 17 | 26 | 2 | Match |
| HF02 — two abstract hoist-capacity units | 15 | 23 | 2 | Match |
| HF03 — missing pre-pour hold-point gate | Infeasible | Infeasible | 0 | Match |
| HF04 — inspection rejection / corrective work / reinspection | 20 | 31 | 2 | Match |

For the three feasible cases, the local schedule checker enumerated the precedence-valid task orders and assigned each task to its earliest resource-feasible start under the declared nonpreemptive, half-open interval semantics. It independently recomputed nominal and upper-duration makespans and checked the canonical witness for:
- task duration consistency;
- predecessor finish-before-successor-start;
- resource capacity at each occupied tick;
- required gates being available;
- hard-deadline robustness.

The blocked case was rejected because a required pre-pour hold-point gate was unavailable. No schedule or accepted-output credit was generated.

## Check summary

- JSON parse of fixture: **PASS**
- JSON parse of companion schema: **PASS**
- Local structural walk over the authored schema's declared required/type/const/enum/format/pattern/array/object constraints: **PASS**
- Four case outcomes against local expected-value recomputation: **4/4 MATCH**
- Canonical witness task-duration, precedence and resource-capacity checks: **PASS**
- Upper-duration deadline/robustness checks: **PASS**
- Generic #1902 case-field envelope and header checks: **PASS**
- Adapter's four case objects preserved byte-semantically after JSON parse/stringification: **PASS**
- Required-gate negative control: **PASS**

## What this does not establish

1. The local structural walk is not a complete implementation of JSON Schema Draft 2020-12. It covers the keywords used by this profile, but is not a substitute for an independent standard validator.
2. The cases have now been projected into the exact generic micro-oracle fixture envelope and the four case objects were checked for exact semantic preservation. They have **not** yet been executed through the separate Rust #1902 oracle/planner toolchain.
3. No exact-head Rust compilation, unit tests, strict Clippy, Nix evaluation or hosted CI result is claimed.
4. The schedule values are abstract ticks and synthetic capacities, not field hours, floor-cycle estimates, crew rates, equipment specifications or procurement recommendations.
5. No physical work, robot dispatch, lifting, structural assessment, safety qualification, code approval or independent physical acceptance occurred.

## Next integration gate

Run the exact generic adapter fixture through the existing #1902 exact oracle and hybrid planner without copying their solver or validator, preserve the construction-specific functional-output mapping as a separate adapter, and capture exact-head receipts. Extend with #1843 disturbance/recovery cases only after the nominal adapter matches the reference outcomes.
