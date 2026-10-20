# EV factory event trace contract and validator v0.1

**Benchmark:** `EV-FACTORY-TRACE-001`  
**Status:** research prototype; example fixture is synthetic  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_trace_validate.rs`

## Paradigm shift: evidence is part of the factory model

A "digital twin" is not trustworthy merely because its output looks realistic. A simulator calibrated on duplicate scan events, missing station completions, mismatched clocks, wrong job-to-station transitions or hidden holdout leakage can produce reproducible but false confidence.

This validator creates an evidence-ingestion boundary between future Manufacturing Execution System (MES), programmable-logic-controller (PLC), warehouse and quality event logs and the simulation/optimization model. It checks structural consistency, declared provenance, time windows, route order, finite buffer occupancy, resource utilization states, rework semantics and paired breakdown/repair events. It cannot prove that all real events were observed or that a sensor, operator scan or MES record is truthful; that is a separate data-quality and assurance problem.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_trace_validate -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_trace_v0_1.json
```

The versioned JSON contract requires:
- Trace ID, collection window, UTC start/end labels, timezone, clock basis, time unit and shift-calendar identifier.
- UTC timestamps are strict whole-second `YYYY-MM-DDTHH:MM:SSZ`; the UTC interval must equal `collection_window_seconds`. The all-zero digest sentinel is accepted only for explicitly synthetic fixtures, never measured/public evidence.
- An explicit evidence class and 64-hex-character source SHA-256 field.
- Missingness declarations including an explanation even when none is known.
- Station/resource identifiers, rework limits and one declared finite buffer between every sequential station.
- Globally unique event IDs and nondecreasing monotonic timestamps within the declared collection window.
- Separate calibration and holdout windows with no time overlap.
- Job release → ordered station start/end → quality outcome → buffer enter/exit / bounded rework → acceptance or scrap; active station work must match a resource ID and breakdowns must pair with repairs.

Unknown top-level fields fail closed. Event types have an explicit allowed field shape; a malformed event cannot silently become a different event. Time windows use half-open semantics `[start_seconds, end_seconds)`; equal adjacent boundaries do not overlap.

## Benchmark design and comparators

A useful comparison is not just JSON schema performance. Compare trace structural pass/fail recall against injected defects; event reconstruction coverage; activity coverage; trace conformance/similarity; event-to-event timestamp residuals; calibration/holdout leakage detection; and downstream held-out simulation fidelity.

A 2026 *Decision Analytics Journal* paper introduces WEFTSIM, a process-mining-based manufacturing simulation framework. Its publisher page reports around 80% activity-level coverage and 90% trace similarity on its evaluation. These numbers are **paper- and dataset-specific reference points**, not direct comparable results for this validator. WEFTSIM builds simulation models from process data; this Rust prototype focuses first on strict ingress validation and trace consistency, not automated process discovery or a competitive trace-similarity score. Source: https://doi.org/10.1016/j.dajour.2026.100729.

A 2026 Procedia CIRP paper studies joint online validation and parameter calibration for manufacturing DES using bootstrap particle filtering, including comparison of event sequences and performance indicators to detect model drift. That is the right later direction after a trace contract and known-good synthetic tests exist: https://doi.org/10.1016/j.procir.2026.03.284.

NIST's July 2026 manufacturing digital-twin workshop report identifies interoperability, verification/validation/uncertainty quantification, cybersecurity and workforce readiness as persistent priorities. The validator treats these as continuing work rather than equating structural validation with trust: https://doi.org/10.6028/NIST.IR.8620. NIST's October 2026 uncertainty guidance covers consistency between simulation uncertainty and measurement/metrology practice: https://doi.org/10.6028/NIST.AMS.100-85.

## Interoperability path: do not invent an island standard

The contract is an internal canonical trace envelope for validation; it should be connected to, not substituted for, established machine semantics:

- **ISO 23247** defines the manufacturing digital-twin framework. ISO's framework overview describes how twins support event detection, predictive maintenance, adaptation and analytics, while the framework does not mandate a single data format or communication protocol. Keep canonical job/station/buffer state separate from transport and source adapters: https://www.iso.org/standard/75066.html and https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A23247%3A-6%3Aed-1%3Av1%3Aen.
- **OPC UA for Machinery** publishes reusable building blocks for machine identification, component identification and monitoring/operating state; the wider roadmap includes job management, result transfer and energy monitoring. A future adapter should preserve source node IDs, engineering units, source timestamps/status/quality, model version and mapping provenance rather than flattening these into anonymous counters: https://opcfoundation.org/markets-collaboration/opc-ua-for-machinery/ and https://reference.opcfoundation.org/specs/OPC-40001-1.
- **OPC UA for Machine Tools** has a companion model intended to exchange machine monitoring/job information with MES, SCADA, ERP and analytics systems: https://reference.opcfoundation.org/specs/OPC-40501-1.
- **NIST manufacturing digital twins** identifies VVUQ and interoperability as technical priorities; its 2026 workshop report adds cybersecurity and workforce readiness as cross-cutting concerns: https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing and https://doi.org/10.6028/NIST.IR.8620.

Next step: define a source-neutral normalized event envelope plus versioned adapters from actual standards/exports. Maintain both original source event bytes/digest and normalized event digest with a deterministic transformation version. Never claim conformance to ISO 23247 or OPC UA just because the internal fields have similar names; formal mapping and conformance testing are separate deliverables.

## Provisional gates to measure

| Metric | Initial acceptance target | Interpretation |
|---|---|---|
| Injected-contradiction detection | 100% of enumerated negative controls rejected | A finite mutation suite, not proof of all possible error detection |
| Valid-trace acceptance | 100% of known-valid contract fixtures accepted | Depends on the declared contract; unknown/unobserved events remain out of scope |
| Provenance completeness | Every trace has source digest, evidence class, clock/time unit, collection interval, timezone, shift calendar and missingness declaration | Metadata completeness is not proof of source authenticity |
| Calibration/holdout leakage | 100% of overlapping windows rejected | Does not prevent leakage introduced outside this file or during analyst behavior |
| Real-event coverage | Measure unknown; target to be set after a rights-cleared trace is characterized | Never infer from synthetic fixture results |
| Downstream forecast fidelity | Do not set a SOTA claim before independent temporal holdout evaluation | Report throughput/WIP/downtime residuals and uncertainty coverage |
| Release authorization | Always false | Factory, product quality, electrical/battery safety and jurisdictional compliance remain separate approvals |

These are internal engineering targets, not established global industry standards.

## Evidence and security limits

- The supplied trace includes two synthetic jobs, one synthetic rework loop and one synthetic breakdown/repair pair. Its all-zero SHA-256 value is a sentinel for the synthetic fixture, not a digest of a real source file.
- A measured trace should hash the exact source export before normalization, keep raw inputs access-controlled, and separately hash the normalized trace and transformation software. Avoid putting names, serial numbers, employee IDs or other direct identifiers into job/resource IDs.
- A structurally valid trace can still be incomplete, biased, fabricated, clock-misaligned or unrepresentative. Collection coverage, sensor health, scan compliance, access controls and independent reconciliation must be assessed separately.
- Trace timestamps use a declared relative monotonic clock. Cross-system clock reconciliation and time synchronization evidence are future work; do not combine unsynchronized systems without explicit uncertainty.
- The v0.1 contract expects breakdowns between completed station operations. It does not model a breakdown interrupting an active operation or track the remainder of its service time.
- Calibration and holdout windows are nonoverlapping, but the validator does not fit parameters or evaluate simulation prediction accuracy. It merely guards one form of leakage.
- The `calibration_review_candidate` flag means only that basic evidence-class and critical missingness checks are met. It is not a decision to calibrate a model, and never authorizes production release.

No external event data was collected, no vendor/integrator/plant/agency was contacted and no quote was requested.
