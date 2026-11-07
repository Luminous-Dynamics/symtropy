# Symtropy Sensor Observability Bench

Standalone, read-only evaluator for the **fault-pair diagnosability** of a declared excavation/pneumatic-conveying sensor layout.

Implements [issue #3993](https://github.com/Luminous-Dynamics/symtropy/issues/3993), composing with #3877/#3913 event truth and #3987/#3991 metrology qualification.

## Run

    cargo test --manifest-path crates/domains/symtropy-sensor-observability-bench/Cargo.toml

## v0.1 method

For each required pair of physical/sensor-fault states:
- every selected channel must have exactly one response record for each state;
- a channel discriminates the pair only when both response intervals are valid, independently evidenced and strictly non-overlapping;
- channels sharing an independence group count as one measurement principle;
- DEGRADED / MISSING / UNKNOWN responses never count as healthy physical signatures;
- model estimates and operator-only reports cannot be physical response truth;
- the pair is IDENTIFIABLE only when the declared minimum number of independent principles discriminate it;
- missing/invalid evidence yields INSUFFICIENT_EVIDENCE; a complete matrix with too few discriminating principles yields NOT_IDENTIFIABLE.

This v0.1 crate scores an explicitly proposed layout. It does not automatically select a minimal-cost sensor layout; cost-aware placement optimization belongs to a later version.

## Research motivation

- Borg et al. (2026), Measurement 258A, 118992, evaluate measurement-based fault monitoring across pneumatic system-complexity and fault-profile levels; pressure-drop statistics and cycle time showed cross-configuration utility in their compressed-air test systems. This is adjacent pneumatic evidence, not coarse rock-conveying qualification.
- Zhang et al. (2026), Measurement 272, 121117, report >96.4% current-only recognition for very dilute-phase conveying. That domain limitation makes pairwise fault isolation and matched transfer more important than headline accuracy.
- Achouch et al. (2026), Sensors 26(21), 6741, review conveyor fault-detection literature and identify limited prognosis, principled fusion, uncertainty and cross-site adaptation.
- Kościelny & Sztyber-Betley (2025), MSSP 236, 112979, provide explicit definitions and assessment of fault-pair isolability.

## Non-claims

Response intervals in the fixture are synthetic. A pair reported as structurally identifiable under a synthetic fixture is not physically diagnosable. Real response envelopes must come from independent calibrated measurements on the declared material/path domain, with sensor dependencies and common-mode failures reviewed.
