# Task-scoped manufacturing digital-twin commissioning v0.1

Tracking: [MANUFACTURING-DT-COMMISSIONING-001 #4982](https://github.com/Luminous-Dynamics/symtropy/issues/4982)

## Standards maturity and alignment

- ISO 23247-5:2026 (published June 2026) covers the digital thread across the manufacturing lifecycle.
- ISO 23247-6:2026 (published July 2026) covers digital-twin composition, including integrated, unified and federated forms.
- ISO/AWI 23247-7 is approved and under development for commissioning, including update frequency, latency, fidelity, operational readiness and cooperation.
- ISO/AWI 23247-8 is approved and under development for digital-twin VVUQ across the lifecycle.

This implementation uses those scopes as architectural context only. It does not implement a normative conformance assessment and does not claim compliance with ISO 23247-7 or -8.

## Paradigm

A manufacturing twin must not carry one global green badge. Readiness is specific to:
- the task being performed;
- the output being claimed;
- the exact component/profile revisions;
- the time the evidence was evaluated.

The same power-source component can be adequate for inter-pass planning but too slow for a fast process-control task. Geometry evidence can qualify a scan-based inter-pass recommendation while remaining insufficient for weld-acceptance or NDT claims. The evaluator therefore returns READY, DEGRADED or NOT_READY separately for each task profile.

## V0 evaluator

The dependency-free Rust library accepts typed component evidence and a task profile. It checks:
- source owner, authority, output scope and exact revision identity;
- non-zero source-byte and transform/registration receipt digests;
- observation age and future timestamps;
- per-channel sampling and update interval requirements;
- p99 end-to-end latency requirements (unknown latency is not zero);
- clock-domain mapping uncertainty;
- coordinate frame, registration error, coverage and occlusion;
- output fidelity error;
- common-cause domains when independent sources are required;
- duplicate, missing and unbound components.

Required evidence failure yields NOT_READY. An optional component failure yields DEGRADED. Unknown mandatory values never satisfy a requirement. Unlisted components are rejected rather than silently incorporated.

The receipt contains the task/profile identity, evaluation timestamp, evaluated components, failed requirements and a deterministic FNV-1a 64-bit reproducibility fingerprint. The fingerprint is explicitly **not cryptographic** and is not a signature or attestation; source/transform integrity still depends on the exact SHA-256 values supplied by authorized adapters.

## BAM reference timing profile

The companion Tandem-GMAW paper describes different source cadences:
- voltage/current: 0.1 ms;
- robot TCP coordinates: 8 ms;
- power-source OPC-UA values: 100 ms;
- control-system power/gas/thermocouples: 1 ms;
- surface scan profiles: 50 Hz;
- video triggers: 40 Hz.

It documents UTC timestamps on a shared TwinCAT3/EtherCAT master clock and <10 µs drift for the relevant Beckhoff analog terminals only. These values are preserved per modality; they are not normalized into one global sample rate. Raw Zenodo data remain rights-pending under #4643.

The synthetic inter-pass profile treats the 100 ms power-source stream as adequate for a medium-rate inter-pass task but rejects that same stream when the task demands 1 ms sampling. This is a contract example, not a physical control-system qualification.

## Tests

The Rust suite covers:
- successful inter-pass readiness;
- task-specific rate rejection for fast control;
- unknown latency, stale/future observations and unknown/excessive clock uncertainty;
- wrong coordinate frame, low scan coverage, excessive occlusion;
- missing transform receipt and authority mismatch;
- optional-component degradation and missing mandatory components;
- common-cause source dependency;
- unbound/duplicate components;
- deterministic receipt fingerprint and profile-revision binding;
- explicitly unsupported post-weld quality/NDT task.

## Benchmark targets

Software contract:
- 100% agreement with independent finite-corpus known answers;
- zero false READY on frozen invalid/stale/contradictory inputs;
- zero authority upgrades;
- deterministic receipt reproduction for identical frozen inputs;
- complete negative-control coverage.

Operational qualification, once real sources are lawfully admitted:
- per-channel sample/update intervals and end-to-end p99 latency;
- clock mapping residual/uncertainty by source pair;
- data-age and dropout distributions;
- spatial registration and coverage;
- output-specific fidelity and uncertainty coverage;
- detection time for stale/misaligned inputs;
- false-ready/false-not-ready rates against independent task qualification.

Real thresholds must be derived from the risk envelope of the task. A single global update rate is not a meaningful readiness metric.

## Claim ceiling

A Rust PASS establishes only that the typed synthetic components satisfy the declared task profile. It does not establish sensor calibration, physical twin fidelity, model validation, robot safety, WPS/PQR/WPQT qualification, weld acceptance, NDT, or ISO conformance.
