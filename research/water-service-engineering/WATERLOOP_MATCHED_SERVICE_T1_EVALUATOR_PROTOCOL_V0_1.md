# WaterLoop matched-service T1 evaluator protocol v0.1

**Tracking issue:** #5911 WATERLOOP-MATCHED-SERVICE-T1-EXECUTOR-001  
**Physical test-design parent:** #5908 / draft PR #5909  
**Accounting precursor:** #5859 / PR #5862  
**Date:** 17 November 2026  
**Evidence class:** synthetic-only known-answer evaluator.

## 1. Paradigm: credit the service, not the machine output

A treatment module can produce water without delivering a useful litre to a fixture, and a recovered fixture litre can still fail to displace mains water. The evaluator therefore distinguishes:

- gross treatment product;
- product volume with a current quality release for the exact end-use profile;
- recovered volume actually measured at the fixture;
- treatment-arm mains fallback;
- mains volume measured at the same fixture in a matched control;
- reject, purge/backwash, overflow and energy.

The control and treatment arms must refer to one frozen demand trace, same use profile, comparable demand and the same service envelope. Gross product, stored-but-unused water and residual streams never become service credit.

## 2. Per-block credit

Point credit is bounded by:

`min(quality_qualified_product_l, recovered_fixture_use_l, max(0, normalized_matched_control_mains_fixture_l - treatment_mains_fallback_l))`

The control mains volume is normalized to the treatment arm's measured fixture demand using `control_mains_l × treatment_demand_l / control_demand_l`; a zero or non-comparable control demand is indeterminate. This is a declared deterministic normalization, not a causal adjustment model.

Quality release must be current, independently reviewed and match both the exact declared use profile and source profile. A release for toilet flushing cannot qualify irrigation or potable use.

An expanded-uncertainty screen is reported separately. The normalized control-meter uncertainty is scaled by the same demand ratio. For the two-meter mains difference, the conservative uncertainty allowance adds standard uncertainties rather than assuming independent calibration errors:

`displacement_lower_l = max(0, displacement_l - k * (u_control_mains_l × treatment_demand_l / control_demand_l + u_treatment_fallback_l))`

`lower_credit_l = min(max(0, qualified_l - k*u_qualified), max(0, fixture_use_l - k*u_use), displacement_lower_l)`

v0.1 uses `k=2` only as a declared uncertainty expansion factor. It is not a 95% confidence interval, probability, significance test or population inference. Statistical inference remains `NotRun`. Demand-measurement uncertainty is not modeled in v0.1 and must be added before field evidence qualification.

## 3. Evidence/validity gates

A block receives zero credit or an explicit indeterminate disposition when it contains:

- missing control or treatment run;
- wrong arm or use/source profile;
- mismatched demand-trace digests or excessive paired-demand difference;
- overlapping or invalid run windows;
- measurement windows that do not match the parent run's frozen time window;
- quality releases outside their validity period or missing the independent reviewer, sample-set digest or method revision;
- service pressure/flow outside the frozen envelope;
- missing, stale, mismatched or non-independent quality release;
- invalid units, meter calibration/method identity, raw-record digest or measurement validity;
- duplicate instrument, channel or raw-record identity;
- measurement at the wrong physical boundary;
- reuse of a recovered allocation ID across multiple paired blocks;
- reuse of the same raw meter-record digest across different paired blocks.

All paired blocks sharing a duplicate recovered-allocation ID or replayed raw-record digest are rejected together. The evaluator does not choose which duplicate claim was legitimate.

## 4. Frozen synthetic challenge pack

Fifteen six-block cases cover:

1. valid paired credit;
2. high gross output but zero fixture use;
3. end-use quality-profile mismatch;
4. stale quality release;
5. missing matched control;
6. treatment mains fallback reduces displacement;
7. duplicate instrument identity;
8. one recovered allocation counted in two blocks;
9. reject stream relabelled as fixture use;
10. pressure outside the declared service envelope;
11. overlapping control/treatment windows;
12. non-comparable matched demand;
13. uncertainty expansion consumes the apparent credit margin;
14. raw meter record replayed across two paired blocks;
15. expired quality release.

Each case has six paired blocks. A negative-control event is isolated to one block except the duplicate-allocation case, which rejects every block reusing the same allocation ID.

## 5. Outputs

Per block: point credit, uncertainty-screened lower credit, normalized control mains, displacement, lower displacement, control energy, treatment energy and incremental energy delta. Treatment and incremental kWh per 1,000 L of lower-bound credit remain separate and are null when no lower-bound credit exists. Energy observations are retained for zero-credit/indeterminate blocks when the energy channel itself is independently valid.

Per case: paired/credited block counts, aggregate point and lower-bound credit, control/treatment/incremental energy and observation coverage, both kWh intensities, dispositions and counts.

Every synthetic case carries:

- `physical_qualification_status: NotRun`;
- `statistical_inference_status: NotRun`;
- `claim_eligibility: NotEligible`.

## 6. Reproducibility and claim ceiling

The read-only Rust binary reuses the existing `water-service-t0` crate and pinned dependencies. Its receipt binds the exact input, input schema, separate oracle, frozen design-plan bytes, plan Git blob identity, Cargo manifest/lock, implementation and exact checkout revision. CI separately validates both Draft 2020-12 schemas, runs the known-answer oracle and enforces an exact-revision receipt.

A PASS proves only that the software applies this accounting/evidence contract to synthetic fixtures. It does not prove real water savings, treatment efficacy, analyte removal, quality release, potability, SANS 241 or NSF/ANSI 350 compliance, regulatory approval, lifecycle superiority or field deployment. No physical trial, procurement, occupied use, public-network connection or actuation is authorized.