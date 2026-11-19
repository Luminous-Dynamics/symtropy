# Actuator Grouped Split-Conformal Calibration v0.1

**Status:** M1 grouped statistical reference with synthetic known-answer fixture. This is not a calibrated actuator model, an empirical coverage claim or a safety qualification.

**Research owner:** [ACTUATOR-SURROGATE-CALIBRATION-001 #6262](https://github.com/Luminous-Dynamics/symtropy/issues/6262)  
**Robust multi-mission co-design:** [#6234](https://github.com/Luminous-Dynamics/symtropy/issues/6234)  
**Mission-conditioned evaluator:** [#6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230) / [PR #6231](https://github.com/Luminous-Dynamics/symtropy/pull/6231)  
**Evidence-lane Pareto filter:** [PR #6260](https://github.com/Luminous-Dynamics/symtropy/pull/6260)

## 1. Why a training fit is not a capability guarantee

Two 2026 approaches make actuator co-design substantially more practical:
- Nasonov et al., *Task-Aware Actuator Parameter Allocation for Multibody Robots*, IEEE RA-L 2026, DOI https://doi.org/10.1109/LRA.2026.3674006. The project reports regressions fitted to manufacturer data for 119 motors and 58 gearboxes, CMA-ES design search and full-order constrained trajectory optimization across jump, loaded stairs and whole-body manipulation tasks. Project page: https://mkakanov.github.io/actorob/
- *Towards the best robot for the job: Optimising actuation design through multi-task co-design and component selection*, IEEE ICRA 2026, DOI https://doi.org/10.1109/ICRA57385.2026.11697232. Its abstract describes task-set co-design followed by selecting real catalogue components.

These are important baselines. But manufacturer data and an optimizer's own regression fit are not independent evidence that its torque-speed prediction bounds cover real actuator specimens. ActuatorList's 3 November 2026 snapshot reports 22 runs across 17 catalogue actuators, with zero lab and zero community submissions; it distinguishes raw traces, tables, digitized plots and text-reported claims. https://actuatorlist.com/benchmarks

This package supplies a small statistical component for the missing validation layer: frozen model/split identities, independent-specimen grouping, held-out interval coverage and explicit abstention.

## 2. Statistical unit: the specimen, not the sample

One actuator specimen may produce hundreds or thousands of trajectory observations. Those observations share the same hardware, assembly, manufacturing history, calibration and degradation state. Treating them as independent conformal calibration units would artificially inflate n and produce an unjustifiably narrow correction.

For each independent calibration specimen g, aggregate all its frozen mission observations using the **maximum interval miss**:

```text
s_g = max_i max(0, L_i - y_i, y_i - U_i)
```

where L_i and U_i are the base model's predicted lower and upper torque bounds and y_i is independently observed torque.

For n independent calibration specimens and target miscoverage alpha, use the finite-sample order statistic:

```text
k = ceil((n + 1) * (1 - alpha))
q = kth smallest specimen-level score
```

The calibrated interval expands both ends:

```text
[L_cal, U_cal] = [L - q, U + q]
```

If k exceeds n, the result is **InsufficientCalibrationGroups**. No finite q is invented, no test-coverage result is issued and no candidate is promoted.

With alpha = 0.05, at least 19 independent calibration specimens are required for a finite order-statistic rank. More specimens are normally necessary for useful precision, manufacturer/architecture strata and a credible physical study.

## 3. What the group-level claim means

Under the declared exchangeability assumption at the independent-specimen group unit, a fixed base model and frozen train/calibration/test splits, the maximum-score construction targets **marginal coverage of an entire sampled specimen trajectory**.

This is not:
- conditional coverage at every speed, torque, temperature or manufacturer;
- a guarantee under distribution shift or an unrepresented actuator architecture;
- proof that the bounded domain is physically complete;
- a replacement for an independent test campaign.

A specimen is counted Covered only when every in-domain observation in its frozen trajectory lies inside the expanded interval. If any observation is outside the declared domain, the entire specimen is OutOfDomain and abstains. This prevents the evaluator from discarding only the difficult samples and reporting optimistic coverage.

## 4. Frozen identities and evidence

The calibration plan binds:
- base model digest;
- distinct training, calibration and held-out test split digests;
- mission-set digest and exact trajectory sampling contract;
- complete system-boundary digest;
- operating-condition digest and model-domain digest;
- target alpha and physical measurement evidence class;
- training specimen IDs when known.

Each dataset and each specimen record preserves:
- specimen, manufacturer, actuator model and architecture IDs;
- source, measurement method, calibration digest and evidence class;
- sample IDs and ordered trajectory sample indices;
- predicted lower/upper torque and observed output torque;
- explicit in-domain/abstain state.

Training specimen IDs cannot occur in calibration or test; calibration and test specimen IDs cannot overlap; sample IDs cannot repeat across the two datasets. Model, mission, boundary, operating-condition and domain digests must match the frozen plan. Calibration and test use the same evidence lane so a vendor-reported calibration set cannot silently qualify an independent-physical claim.

This M1 reference requires the same number and ordering of trajectory observations for every specimen. A mission-set digest binds the actual sample schedule; variable mission lengths should be separate cohorts or use an explicitly designed trajectory-level scoring contract.

## 5. Reported metrics

The Rust report includes:
- independent calibration specimen count and one-based conformal rank;
- calibrated correction q in N·m, or explicit insufficient-calibration state;
- test specimen count, in-domain count, covered/uncovered count and OOD abstentions;
- empirical group-level test coverage;
- descriptive 95% Wilson interval for the observed test coverage;
- mean raw and adjusted prediction interval width;
- per-specimen coverage/abstention disposition, manufacturer/model/architecture, evidence source and measurement method;
- calibration group scores and model/mission/boundary/condition/domain digests.

The Wilson interval summarizes sampling uncertainty in observed test coverage; it does not replace the split-conformal marginal statement and does not create subgroup guarantees.

## 6. Synthetic known-answer fixture

`examples/grouped-conformal-known-answer.json` is entirely synthetic:
- three calibration specimens, each with two observations;
- specimen maximum scores 1, 0 and 1 N·m;
- alpha = 0.25, so k = ceil(4 × 0.75) = 3 and q = 1 N·m;
- three held-out specimens: one fully covered, one uncovered, one OOD-abstained;
- empirical in-domain specimen coverage is 1/2 = 0.5, with the corresponding Wilson uncertainty.

The fixture does not contain real vendor, physical, laboratory or independent measurements.

## 7. Run

Dependency-free Rust crate, Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/actuator-calibration/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/actuator-calibration/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/actuator-calibration/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

Dedicated workflow: `.github/workflows/actuator-grouped-conformal-calibration.yml`. The authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is the compilation/formatting gate.

Twenty-one Rust tests cover exact grouped scores/ranks, group-level test coverage, insufficient n, all split/leakage checks, duplicate samples, OOD calibration rejection and whole-specimen abstention, evidence-lane matching, model/mission/domain/boundary/condition identity, invalid intervals/non-finite data, trajectory index/length identity, no fake zero coverage, Wilson bounds and empty data.

## 8. M0–M5 promotion

- **M0:** transcribe public actuator data with source class, units, missingness and exact test/system boundaries.
- **M1:** grouped conformal arithmetic and fail-closed split/OOD controls.
- **M2:** grouped leave-one-actuator-model-out, leave-one-manufacturer-out and where available leave-one-architecture-out validation; publish residual/coverage strata.
- **M3:** independent synchronized torque/speed/electrical/thermal instrumentation with calibration and raw-trace digests.
- **M4:** held-out whole-robot missions and measured task outcomes; calculate false-feasible decisions.
- **M5:** repeat after wear, service, remounting and recalibration; stale model applicability is revoked.

## 9. Claim ceiling

Synthetic tests validate the arithmetic and fail-closed data semantics only. Split conformal relies on exchangeability of independent specimen groups, a fixed model and the frozen domain/mission; it does not guarantee coverage under arbitrary shift or conditional subgroups. The protocol does not make manufacturer specifications independent, remove model-form error or certify safety. Physical acquisition requires separate hazard analysis, competent operators, rated guarded fixtures, independent calibrated instrumentation and preregistered stop limits.
