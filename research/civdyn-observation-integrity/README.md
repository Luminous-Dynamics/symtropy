# CIVDYN observation-integrity reference v0.1

This standalone Rust reference asks whether a present measurement is admissible for a current physical-state decision, and what causal conclusions are justified by the available evidence.

## Core distinction

A verified receipt establishes provenance relative to its declared trust root. It does not prove that the numeric value is true of the physical system. A validly authenticated reading can still be stale, misconfigured, derived, biased by its instrument, or altered before it reached the signing gateway.

The reference therefore keeps three claims separate:
1. receipt provenance;
2. measurement applicability and independence;
3. physical truth and cause attribution.

No unique physical, instrument or in-path-integrity cause is inferred from one plausible numeric value.

## Candidate and evaluator separation

The candidate API accepts typed evidence records only. It receives no latent world ID or hidden cause. A separate runner holds the evaluator's truth labels and writes them only to the evaluator CSV.

The candidate-only command emits:
scenario ID, disposition, action, unresolved hypotheses, and reason codes.
It does not emit the hidden evaluator cause.

## Frozen cases

Twelve cases are included:
- identical primary measurement value under hidden physical degradation, instrument bias and in-path alteration;
- stale but authenticated reading;
- a secondary measurement sharing gateway and time root with the primary;
- derived reconstruction that must not count as independent physical evidence;
- matching independent physical corroboration;
- independent physical disagreement, producing EVIDENCE_CONFLICT rather than a vote;
- instrument self-test failure without unique cause attribution;
- independent forensic integrity indicator routed to investigation;
- calibration revision mismatch;
- unverified receipt.

The same three equivalence cases use identical candidate evidence records, and the stale-but-authenticated control is a separate fourth case. They must produce identical candidate dispositions and unresolved hypothesis sets.

## Independence and freshness

Evidence independence requires distinct source ID, acquisition path, gateway, trust root, time root and calibration authority. Merely using two device IDs is insufficient.

Freshness is evaluated at the current assessment tick. The measurement and ingestion ticks may not be in the future, event time must not follow measurement time, and both the policy age limit and per-record age limit apply.

Units, calibration revision and asset configuration must match the claim's admission policy. Derived and reconstructed records do not become direct physical measurements.

## Decision rules

- Equivalent telemetry without admissible independent evidence -> abstain and preserve physical/instrument/in-path hypotheses.
- A matching independent physical measurement -> corroborate the physical state, but do not attribute the residual fault cause.
- Independent physical disagreement -> EVIDENCE_CONFLICT.
- Stale, unverified or mismatched primary evidence -> reject for current-state use.
- Instrument self-test failure -> request independent physical measurement; it does not rule out an in-path alteration.
- An independent forensic integrity indicator -> integrity investigation; no actor identity, motive or cyber-causation is emitted.

## Research positioning

Ma et al. (2026), "Identifiability-Aware, Cost-Aware Triage of Physical Faults and Measurement-Integrity Anomalies in Energy Cyber-Physical Systems", Sensors 26(17), 5585, evaluates physical-consistency triage on Tennessee Eastman Process and wind-farm simulations. It reports conditional AUC 0.873 and 0.895, but on exactly matched sensor-fault/attack observation pairs the measurement-only cause classifier is at chance (AUC 0.5). This is an important identifiability limit: more model capacity cannot separate causes that produce the same received observation. The paper also notes raw-statistics XGBoost can score higher on clean data; physical-consistency and abstention should be compared on auditability and safe response, not claimed as universal clean-data accuracy superiority.
https://doi.org/10.3390/s26175585

NIST SP 800-82 Rev. 4 Initial Public Draft (21 September 2026) expands OT-security guidance across building automation, water/wastewater, food/agriculture, freight rail, maritime, IIoT/cloud convergence, asset management and monitoring. It remains a draft with comments open through 30 November 2026.
https://csrc.nist.gov/pubs/sp/800/82/r4/ipd

IBM Maximo APM illustrates the enterprise baseline: condition monitoring, IoT/inspection/maintenance data integration, anomaly detection and predictive/prescriptive maintenance recommendations. CIVDYN should complement these workflows with a strict measurement-admissibility and ambiguity contract, not compete on dashboard breadth.
https://www.ibm.com/products/maximo/asset-performance-management

## Run

cargo test --locked --manifest-path research/civdyn-observation-integrity/Cargo.toml
cargo run --locked --manifest-path research/civdyn-observation-integrity/Cargo.toml --bin observation-integrity-sweep
cargo run --locked --manifest-path research/civdyn-observation-integrity/Cargo.toml --bin observation-integrity-sweep -- --candidate-only

## Claim ceiling

This is a deterministic finite evidence-admission reference. Receipt status is supplied by an upstream verifier; this crate does not implement signature cryptography or key management. It does not establish a production cyber detector, real sensor accuracy, causal identification outside its frozen cases or attacker identity/intent.
