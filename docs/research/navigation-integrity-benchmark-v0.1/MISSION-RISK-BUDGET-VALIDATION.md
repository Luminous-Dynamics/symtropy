# Mission-level risk budget validation receipt

**Related PR:** [#6760](https://github.com/Luminous-Dynamics/symtropy/pull/6760)
**Exact head:** `2a66095600e45fcea1783221496dce6a546b9ed0`
**Workflow:** [run #162](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45007002717)
**Status:** passed — isolated software validation only

## Exact-head result

- Formatting and whitespace check: passed.
- Clippy: passed.
- Tests: 88 passed, 0 failed, across `nav-transition-validator` (31), `nav-observation-toy-fim` (45), and `miluv-ingest` (12).
- Student-t, adaptive PID, PL trace, zero-failure and mission-risk demos: passed.
- Generalization schema/split and dataset-admission CLIs: passed.
- Dataset registry: 6 candidates, 0 admitted.

## Mission-risk demo

- Total mission budget: `1e-3`.
- Predeclared discrete checks: `785`.
- Equal per-check allocation: `1.273885350318e-6`.
- Summed union-bound allocation: `1.000000000000e-3`.
- Unallocated remainder: `0`.

## Claim ceiling

This validates the allocation implementation, not the calibration or correctness of any per-check failure probability. The union bound covers only the finite declared discrete checks. It makes no continuous-time claim between checks and does not independently validate mission scope, event identity, source independence, truth lineage or representativeness. No UrbanNav payload is admitted and no physical route result is reproduced.