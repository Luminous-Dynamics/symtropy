# Actuator Calibration Chain Audit v0.1

**Status:** M1 ordered-chain structural audit and deterministic-bound ledger with a synthetic known-answer fixture. This reference does not authenticate certificates, establish laboratory competence or independently create SI traceability.

**Research owner:** [ACTUATOR-CALIBRATION-CHAIN-AUDIT-001 #6349](https://github.com/Luminous-Dynamics/symtropy/issues/6349)  
**Synchronized actuator metrology:** [#6266](https://github.com/Luminous-Dynamics/symtropy/issues/6266) / [PR #6272](https://github.com/Luminous-Dynamics/symtropy/pull/6272)  
**Grouped actuator model calibration:** [#6262](https://github.com/Luminous-Dynamics/symtropy/issues/6262) / [PR #6263](https://github.com/Luminous-Dynamics/symtropy/pull/6263)

## 1. Why a chain digest is not a calibration chain

An opaque chain digest is useful for integrity and change detection, but it cannot show:
- which item was calibrated at each step;
- which reference standard was used;
- whether each intermediate reference matches the next link;
- whether the last link reaches the declared terminal reference;
- whether certificates, methods and validity windows actually exist in the record;
- how each link contributes to the uncertainty of the final measurand;
- whether pre-use and post-use checks show drift, overload or out-of-tolerance behaviour.

NIST's traceability policy makes traceability a property of a **measurement result**, not an instrument, laboratory or organisation. It describes a documented unbroken chain of calibrations, with each link contributing to the measurement uncertainty, and requires a defined measurand, described measurement system, result with evaluated uncertainty, specified calibration reference and measurement-assurance evidence.

Source: https://www.nist.gov/metrology/metrological-traceability

NIST also notes that it does not directly provide torque calibration, but torque may be derived from traceable force and length component quantities when the measurement model and uncertainty are properly evaluated.

## 2. GUM distinction: deterministic bounds are not standard uncertainties

The BIPM/JCGM catalogue lists:
- JCGM 100:2008, *Guide to the Expression of Uncertainty in Measurement*;
- JCGM 100:2008/Amd.1:2026, *Nonlinearity in measurement models*;
- JCGM 101:2008, Monte Carlo propagation of distributions;
- JCGM 102:2011, extension to multiple output quantities;
- JCGM 106:2012, measurement uncertainty in conformity assessment.

Source: https://www.bipm.org/en/web/guest/publications/guides

The data contract separates four link uncertainty kinds:

1. **Validated absolute bound:** a deterministic absolute-error bound in the final measurand unit. M1 aggregates only these values, conservatively, using the triangle inequality.
2. **Standard uncertainty:** retains its value, Type A/Type B/combined source and distribution/model reference. It is not a deterministic hard bound.
3. **Expanded uncertainty:** retains its value, Type A/Type B/combined source, coverage factor, coverage probability and distribution/model reference. It is not a deterministic hard bound.
4. **Unknown:** no numeric contribution is accepted.

A chain containing standard or expanded uncertainty is reported as **StatisticalUncertaintyOnly**, without a deterministic aggregate bound. M2 will implement nonlinear/correlated uncertainty propagation (including GUM Monte Carlo where appropriate) against independent reference calculations.

The M1 triangle-inequality sum assumes every deterministic link contribution has already been mapped into the final measurand unit by its declared conversion/measurement model. The auditor records the model reference but does not validate that model or prove that the contribution is correct.

## 3. Ordered chain contract

For a torque sensor chain, the expected structure is:

    installed torque transducer
      -> transfer standard
      -> reference standard
      -> declared terminal reference

Each ordered link includes:
- unique link ID;
- calibrated item ID and reference-standard ID;
- calibration certificate and reference-standard references;
- calibration procedure and technical-competence evidence references;
- calibration date and validity interval;
- uncertainty kind, value, unit, Type A/Type B/combined source, and applicable coverage/distribution metadata;
- conversion/measurement-model reference mapping the contribution into the final measurand unit;
- certificate digest.

The M1 auditor requires:
- the first calibrated item to equal the installed sensor serial;
- each link's reference-standard ID to equal the next link's calibrated-item ID;
- the last reference-standard ID to equal the declared terminal reference ID;
- the last link's reference-standard source to equal the chain's declared terminal-reference source;
- no duplicate link IDs, repeated calibrated items, self-reference or terminal cycle;
- compatible uncertainty units and valid calibration dates/ranges;
- a nonempty chain digest and terminal reference source.

Malformed structure, broken links, mismatched terminal references, duplicate nodes, malformed dates, unit mismatch or statistical metadata inconsistent with the stated uncertainty kind are rejected rather than downgraded to a numeric result.

## 4. Before/after measurement assurance

The chain record includes both:
- **Pre-use check:** must occur on or before the measurement day.
- **Post-use check:** must occur on or after the measurement day.

Each check records status, day, method, reference artifact, raw-check digest, observed absolute drift, declared drift limit and unit.

A check is not accepted merely because its status says WithinLimit: drift and limit must be present, finite, nonnegative, in the measurand unit and observed drift must not exceed the declared limit. Out-of-tolerance or unknown assurance results make the chain **Unknown**. The full before/after records are preserved in the report, not reduced to a pass/fail label.

This supports later invalidation after overload, shock, remounting, temperature excursion, service or post-run drift.

## 5. Result dispositions

- **BoundQualified:** structurally continuous chain; all links are valid for the measurement day; every link provides a validated absolute-error bound; both assurance checks are within declared limits. The report exposes the conservative aggregate bound.
- **StatisticalUncertaintyOnly:** chain and assurance checks are complete, but at least one link supplies standard/expanded uncertainty. No deterministic hard-bound total is emitted.
- **Unknown:** an expired link, unknown uncertainty, failed/unknown assurance check or another unqualified condition exists. No deterministic total is emitted.
- **Invalid input:** broken chain topology, duplicate/cyclic references, missing required evidence fields, malformed uncertainty metadata, incompatible unit, invalid date ordering or arithmetic overflow is rejected with an error.

Structural validity is required before any disposition is returned. A structurally malformed chain does not receive a BoundQualified, StatisticalUncertaintyOnly or Unknown report.

## 6. Known-answer fixture

The file examples/calibration-chain-known-answer.json contains three synthetic scenarios:
1. **Deterministic bound:** two links contribute 0.02 and 0.03 N·m; the conservative aggregate is 0.05 N·m, provided both before/after checks pass.
2. **Statistical only:** the first link carries a Type B standard uncertainty and a distribution/model reference. The chain is structurally valid but returns StatisticalUncertaintyOnly, with no hard-bound total.
3. **Expired link:** the first link validity expires before the measurement day. The chain returns Unknown and no bound.

All IDs, URLs, certificates, dates, drift values, digest strings and uncertainty values are synthetic placeholders. They are not actual calibration certificates, measured traceability chains or physical results.

## 7. Run

Dependency-free Rust crate, Rust 1.96:

    cargo fmt --manifest-path benchmarks/synthetic-joints/calibration-chain-audit/v0.1/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/synthetic-joints/calibration-chain-audit/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/synthetic-joints/calibration-chain-audit/v0.1/Cargo.toml --all-targets --locked -- -D warnings

Dedicated workflow: .github/workflows/actuator-calibration-chain-audit.yml. The authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is the compile/format/Clippy gate.

Twenty-one Rust test definitions cover ordered deterministic chain aggregation, broken intermediate/terminal references, sensor serial matching, duplicate/self/terminal cycles, expired windows, standard/expanded/unknown uncertainty semantics, coverage factor/probability, unit mismatch, pre/post timing, failed assurance, required provenance, empty chain, overflow, terminal source matching, and full calibration/assurance receipt preservation.

## 8. M0–M5 promotion

- **M0:** source and reference vocabulary, link topology and traceability requirements.
- **M1:** dependency-free graph audit, deterministic-bound aggregation, statistical-only/unknown dispositions and negative controls.
- **M2:** nonlinear/correlated uncertainty propagation using frozen GUM Monte Carlo inputs and independent reference calculations.
- **M3:** instrumented force/length/angle/torque calibration chain with before/after checks and environmental controls.
- **M4:** link chain receipts to synchronized metrology, mission capability and energy/passivity evidence.
- **M5:** calibration drift, overload, remounting, service and post-service requalification.

## 9. Claim ceiling

M1 validates supplied records and arithmetic only. It does not authenticate certificates, verify reference-standard values, establish SI traceability, accredit a laboratory, calibrate an instrument, prove covariance assumptions or authorize physical testing. Those require external reference evidence, competent calibration services, documented measurement assurance, independent instruments and competent laboratory procedures.
