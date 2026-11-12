# ROAD-OPTION-EXPERIMENT-001 — analytic crossing fixture v0.1

**Status:** synthetic analytic conformance only  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4451  
**Consumer:** Symthaea SAM option-preservation evaluator (#11291 / PR #11299)  
**Execution owner:** Symtropy reciprocal loop (#3532)

## Purpose

This is the first machine-readable analytic oracle for the three-arm crossing experiment. It is deliberately not a physical pedestrian or vehicle model. It makes the three policy strategies distinguishable using a finite table whose expected option sets can be calculated exactly.

## Expected results

- **Myopic arm A** selects `commit` because its immediate-only score is 10 rather than 6 for `early_yield`. Both are immediately feasible, but `commit` has no common continuation under the shared observation.
- **Option-preserving arm B** selects `early_yield`. Without an extra measurement, the only common continuation is `hold`.
- **Sensing arm C** also preserves `early_yield`, requests a one-tick actor-clearance measurement, and can choose `brake` under hypothesis A or `accelerate` under B only after distinct observation receipts arrive before tick 2. `hold` remains a common fallback in both cases. The measurement cost is one declared resource unit. This analytic oracle assumes a deterministic, perfectly discriminative measurement when timely; late, dropped, stale and unsupported outcomes are explicitly represented and preserve the shared hold fallback.

## Set calculations

For the `commit` parent action, A supports only `brake` and B supports only `accelerate`, while both receive `commit-shared-view`:

- robust lower set = `{}`;
- possible upper set = `{}`;
- clairvoyant union = `{brake, accelerate}`;
- clairvoyance gap = 2.

For `early_yield` without measurement, both hidden states share `yield-shared-view`:

- A feasible = `{brake, hold}`;
- B feasible = `{accelerate, hold}`;
- robust lower set = possible upper set = `{hold}`;
- clairvoyant union = `{brake, accelerate, hold}`;
- clairvoyance gap = 2.

With timely query delivery:

- A receives class `query-result-A`: robust set = `{brake, hold}`;
- B receives class `query-result-B`: robust set = `{accelerate, hold}`;
- fallback `hold` is preserved in both classes.

A late, stale, dropped or unsupported query cannot change the tick-2 decision. An irrelevant query gets no decision-value credit even if it reduces generic uncertainty.

## File validation

The fixture schema declares JSON Schema Draft 2020-12. The repository also includes a dependency-free SAM-specific semantic validator and mutation tests:

```bash
python -m unittest discover -s tools/road-option-fixture-validator/tests -v
python tools/road-option-fixture-validator/validate_fixture.py
```

The semantic validator checks exact action-domain coverage, hypothesis references, receipt/class consistency, outcome coverage, request/delivery/deadline timing, non-timely query rejection and the expected lower/upper/clairvoyant set calculations. It is a test-only Python standard-library tool, not a production SAM component or general JSON Schema engine.

Receipt identifiers in this fixture are symbolic IDs, not cryptographic digests or independently authenticated evidence.

## Claim boundary

A successful schema/semantic validation is only an analytic conformance result. It does not establish physical vehicle dynamics, human actor realism, sensor accuracy, causal validity in the real world, collision risk, roadworthiness or autonomous-driving safety.
