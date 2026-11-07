# Symtropy Measurement System Bench

Standalone validator for metrology records supporting the excavation/transport pilot. It validates instrument identity, calibration traceability, declared material/path domain, sampling provenance, shared timebase, measurement-model linearity and uncertainty-budget evidence.

It is a **qualification-record admission gate**, not a calibration engine, digital DAQ, conformity assessment body or instrument certification.

## Run

    cargo test --manifest-path crates/domains/symtropy-measurement-system-bench/Cargo.toml

## Reference architecture

- Primary reference: independent gravimetric weighing / calibrated balance or load-cell collection appropriate to the station and collection boundary.
- Secondary estimator: online electrostatic/capacitance or other non-intrusive solids-flow estimator; calibrated and compared against the primary reference.
- Diagnostics: differential/absolute pressure, airflow, current/voltage/real power, temperature and vibration.
- Particle characterization: qualified sample + split procedure; sieve and dynamic image analysis; 3D methods only with a separately qualified reconstruction.
- Timebase: common DAQ clock, synchronization/latency measurement and explicit time uncertainty.
- Every result binds to a material cohort, installed path/configuration, sampling protocol, calibration certificate, raw artifact and measurement model.

## v0.1 validator gates

- Unique instrument/channel/calibration/measurement identities.
- Explicit measurement role; an online estimator cannot be promoted to primary reference by label alone.
- Calibration certificate, laboratory, method, traceability, as-found/as-left status, range and validity interval.
- Calibration domain must contain the declared material, path and geometry domain of the measurement.
- Measurement date must fall within calibration validity.
- All measurement records bind to an exact common timebase and synchronization protocol.
- Particle sample measurements require an explicit sampling protocol/version and source lineage.
- Derived measurement models declare equation/model digest and LINEAR / NONLINEAR / UNKNOWN status.
- NONLINEAR models cannot use LINEARIZED_GUM as their only uncertainty method; a Monte Carlo result/distribution artifact or explicit conservative interval approach is required.
- Every uncertainty component has finite non-negative standard uncertainty, unit, source and distribution. Linearized components require sensitivity coefficients.
- Correlated components require declared dependence/covariance evidence. No silent zero-correlation assumption.
- Reference comparisons bind a secondary estimator to an independently measured reference with matching material/path/window context and separate calibration/validation/holdout partitions.
- Synthetic evidence remains SYNTHETIC_ONLY. Calibration-only comparison never establishes validation qualification.

The validator does not calculate a combined standard uncertainty. That requires a separately specified measurement equation and an approved propagation procedure, including covariance/nonlinearity treatment.

## Candidate online sensor evidence

Li et al. (2025), *Sensors* 25(22), 6850, DOI 10.3390/s25226850, report an electrostatic-coupled-capacitance estimator:
- velocity from upstream/downstream signal cross-correlation;
- concentration from a calibrated capacitance response;
- mass flow computed as cross-sectional area × velocity × concentration;
- 5.5% maximum relative mass-flow error in a 50 mm lab rig with polypropylene pellets;
- 3.6% maximum relative concentration error in a 100 mm pilot rig with polypropylene pellets.

These are different measurands. They do not qualify hard-rock accuracy. The sensor remains a secondary estimator until independently calibrated and validated on the declared rock/path domain.

## Important standards context

- BIPM/JCGM currently lists JCGM 100:2008 Amendment 1:2026 on nonlinearity, JCGM GUM-5:2026 examples, JCGM 101 Monte Carlo propagation and JCGM 102 multiple outputs.
- ISO 14488:2007 + Amd 1:2019 remains the current published sampling standard (confirmed in 2023); ISO/DIS 14488 is a replacement draft in the enquiry phase as of November 2026.
- ISO 13322-2:2021 covers dynamic image analysis qualification and orientation, motion-blur and depth-of-field effects.
- ISO 9276-6:2008 requires shape-characteristic distributions, not just a mean shape.
- ISO/IEC 17025:2017 is the laboratory competence reference; verify the actual accreditation scope and method.

This crate stores source/version references; it does not reproduce proprietary ISO text.
