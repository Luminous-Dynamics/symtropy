# Orbital Event-Capture Observability Benchmark v0.1

**Status:** synthetic reference implementation; exact-head hosted validation pending.  
**Owner:** [ORBITAL-RELIABILITY-OBSERVATION-COMPLETENESS-001 #6659](https://github.com/Luminous-Dynamics/symtropy/issues/6659).  
**Claim ceiling:** synthetic event-capture reconciliation only.  
**Physical completeness / reliability / qualification / authority claims:** always false.

## Why this is a separate assurance layer

Reliability analysis operates on the events that were observed and recorded. It does not directly observe the latent physical process. A perfectly replayable event log can be incomplete if an upstream sensor, threshold, counter, DAQ, buffer, transport, clock or storage path lost events before admission.

For a nonhomogeneous Poisson process with physical intensity \(\lambda(t)\), independent event capture with probability \(p(t)\) produces observed intensity:

\[
\lambda_{\mathrm{obs}}(t)=p(t)\lambda(t).
\]

For a power-law process, \(\lambda(t)=\eta\beta t^{\beta-1}\). If capture probability is constant \(p\), then:

\[
\lambda_{\mathrm{obs}}(t)=(p\eta)\beta t^{\beta-1}.
\]

The observed process is still a PLP with the same shape \(\beta\) and a lower scale. Conditional on the observed event count, the timestamp-shape distribution is unchanged. Thus a constant fraction of missed events cannot be identified from the event timestamps or a PLP goodness-of-fit test alone. Time-varying capture can create, hide or distort apparent trends.

This is an identifiability boundary, not a weakness that can be fixed by a more complicated reliability estimator. Event-capture completeness must be supported by independent event references and measurement-path evidence.

## V0.1 scope

This standalone Rust 2024 crate implements a deterministic synthetic reconciliation contract. It accepts:
- an exact qualified sensor / firmware / acquisition-configuration / event-class / mode / environment scope;
- an independently declared reference-event stream with unique IDs, contiguous sequence numbers, monotonic event times and payload digests;
- observed records linked to reference-event IDs;
- capture-loss counters for overflow, dropped events, dropped packets, resets and sequence gaps;
- calibration validity and scope matching;
- clock synchronization, monotonicity and measured-vs-allowed error;
- reference/acquisition failure-domain declarations;
- a declared operating event-rate envelope; the tested rate is derived as independent reference-event count divided by the declared test-window duration, rather than trusted as a caller-supplied rate.

It reports one of:
- \`RECONCILED_FIXTURE_ONLY\` — every known reference injection is matched one-to-one, with no positive loss counter or observed conflict; synthetic test fixture only.
- \`INCOMPLETE\` — known injections are missing or a capture-loss counter is nonzero.
- \`INDETERMINATE\` — independent reference, calibration, clock or required loss-counter evidence is absent, invalid or common-cause dependent.
- \`OUT_OF_ENVELOPE\` — test configuration or event rate differs from the declared qualified scope.
- \`REJECTED\` — malformed scope, digest, independent reference sequence or input.
- \`DISPUTED\` — duplicate record identity, duplicate capture, unexpected event, payload disagreement or timestamp disagreement.

The evaluator never returns a global \`COMPLETE\`, \`LOSSLESS\`, \`RELIABILITY_PROVEN\`, \`QUALIFIED\` or \`AUTHORIZED\` state. \`RECONCILED_FIXTURE_ONLY\` is deliberately weaker than a hardware capture-qualification result.

## Exact zero-miss bound

For \(k\) missed events among \(N\) independent, representative Bernoulli injection trials, the one-sided exact 95% Clopper–Pearson upper confidence bound on miss probability \(q\) is the solution to \(P_{q_U}(X\\leq k)=0.05\), where \(X\\sim\\mathrm{Binomial}(N,q_U)\). For zero misses this reduces to:

\[
q_U=1-0.05^{1/N}.
\]

Illustrative sample sizes:
- 299 zero-miss injections put this bound below 1%.
- 2,995 zero-miss injections put it below 0.1%.
- 29,956 zero-miss injections put it below 0.01%.

The crate reports the exact one-sided upper bound for zero or nonzero misses when the reference/observation reconciliation is otherwise determinate. Duplicate captures, unmatched observations, or payload/time disputes suppress the bound because the Bernoulli capture outcome cannot be reliably adjudicated. A positive loss counter or a missing injection produces \`INCOMPLETE\`; a confidence bound does not turn that result into a pass.

These sample-size illustrations assume independent, representative trials from one unchanged detection regime. Burst loss, common-cause faults, event-rate dependence, time-varying capture probability or untested event classes invalidate a naïve iid interpretation. The responsible technical authority must allocate risk and define strata; these values are not universal qualification requirements.

## Independent reference and common-cause guard

The reference evidence must declare failure domains for both the independent reference system and the acquisition path. Any overlap blocks reconciliation as \`INDETERMINATE\`. An empty failure-domain declaration also blocks. These are declarations in this synthetic profile; the evaluator does not authenticate an actual reference instrument or prove physical independence.

The same scope rule applies to calibration and time:
- calibration must be valid at test time and apply to the exact tested scope;
- the synchronized clock must be monotonic;
- measured clock error must be present and no greater than the predeclared allowed error;
- observed timestamp differences must remain within the measured bound;
- required acquisition loss counters must all be present.

A reference path sharing a clock, power source, sensor front-end, firmware, transport or storage failure mode with the acquisition path is not independent corroboration.

## Frozen adversarial corpus

The v0.1 corpus contains 22 synthetic cases:
1. all known injections reconciled;
2. one and multiple missing injections with exact binomial upper bounds;
3. duplicate capture of one reference event;
4. observation with no reference event;
5. missing independent reference;
6. shared reference/acquisition clock failure domain;
7. calibration scope mismatch;
8. firmware configuration mismatch;
9. DAQ overflow counter positive;
10. missing packet-loss counter;
11. measured clock error exceeds its limit;
12. malformed firmware digest;
13. gap in the independent reference sequence;
14. payload mismatch;
15. timestamp outside the clock error bound;
16. event rate outside the declared envelope;
17. duplicate observed record identity;
18. all reference events missing;
19. shared firmware failure domain;
20. non-monotonic clock declaration.

The independent JavaScript oracle implements the reconciliation decisions and binomial upper-bound inversion separately from Rust. Integer event clocks, sequence values and counters are restricted to the exact interoperable range shared by Rust and JavaScript numbers. The frozen corpus is an expected-result fixture, not a statistical validation of a physical sensor or a statistically representative capture campaign.

## Standards and industry positioning

- **IEC 61710:2013** — PLP estimation, confidence intervals, prediction intervals and goodness-of-fit for repairable-item event histories. Its stated common-condition data scope makes event capture and operating-regime evidence important inputs, not something the model fit can certify. [IEC publication](https://webstore.iec.ch/en/publication/5732).
- **IEC 61124:2023** — constant failure-rate / constant failure-intensity compliance test plans under the independent exponential-time assumptions in the standard. [IEC publication](https://webstore.iec.ch/en/publication/65838).
- **MIL-HDBK-781 Rev B** — reliability test methods, plans and environmental profiles for engineering, development, qualification and production; DLA lists it Active with a 29 July 2026 validation date. [DLA record](https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=54097).
- **NASA-STD-5017B** — physical mechanism requirements and qualification guidance, including service-life environments, all mechanism functions, energy extremes and project-tailored sequence decisions. [NASA standard record](https://standards.nasa.gov/standard/NASA/NASA-STD-5017).
- **NASA-STD-7009B / NASA-HDBK-7009B** — model credibility and intended-use discipline. The companion handbook is Version B dated 3 February 2026 and listed active. [NASA handbook record](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009).
- **ISO 14224:2016** — structured reliability and maintenance data collection/exchange for petroleum, natural gas and petrochemical equipment; it is a data taxonomy comparator, not an analysis method or a general aerospace standard. [ISO record](https://www.iso.org/standard/64076.html).
- **JMP 19.2 Crow-AMSAA** — commercial PLP / NHPP estimation and diagnostics baseline, not an event-capture-completeness qualification system. [JMP documentation](https://www.jmp.com/support/help/en/19.2/jmp/crow-amsaa.shtml).

This work is deliberately complementary. It does not replace IEC reliability methods, ISO data taxonomies, NASA physical qualification, MIL-HDBK test planning or commercial reliability software. Its target differentiation is explicit observation-channel evidence, exact subject/configuration binding, independent event reconciliation and non-compensatory \`INDETERMINATE\` outcomes when the capture path cannot be bounded.

## Ownership / composition

- #6659 owns event-capture observability and completeness evidence.
- #6617/#6618 own model-input admissibility and consume capture evidence as a prerequisite; they do not duplicate this evaluator.
- #6515/#6516 own immutable byte-exact event-stream admission and replay.
- #3987/#5894/#5062 own measurement traceability, calibration freeze and uncertainty architecture.
- #6540 owns risk-bearing qualification test coverage, interactions and sequence constraints.
- #6256/#6303 own physical ground test-cell planning, safety review, independent metrology and campaign execution.
- #6593/#6622/#6636 own statistical reliability estimators and diagnostics; none may self-certify the completeness of their input observations.
- #6598 composes independently produced evidence without promoting this synthetic receipt into physical truth or authority.

## Commands

~~~bash
cargo fmt --manifest-path tools/orbital-event-capture-observability-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-event-capture-observability-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-event-capture-observability-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-event-capture-observability-bench/Cargo.toml -- \
  tools/orbital-event-capture-observability-bench/fixtures/v0.1/capture-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-event-capture-observability-bench/tests/reference_oracle.mjs \
  tools/orbital-event-capture-observability-bench/fixtures/v0.1/capture-cases.json
~~~

## Claim ceiling and next phase

This benchmark can establish only that the declared synthetic evaluator reconciles frozen reference/observation records and rejects its encoded negative controls. It does not establish that the injected events represent all physical event classes, that the reference instrument is actually independent, that a real DAQ has a bounded miss probability, or that a hardware reliability model is valid.

Next increments should add:
1. an independently implemented one-sided exact binomial upper bound for nonzero misses;
2. burst / correlated-loss strata and detection-probability curves;
3. signal amplitude, threshold, event rate, dead-time and environmental sweeps;
4. raw waveform / counter / packet / persisted-record cross-reconciliation;
5. hardware-in-loop fault injection on the exact #6256 test-cell sensor/DAQ configuration;
6. a typed, signed, validity-window-bound capture receipt for consumption by #6617/#6598.

Physical validation requires independently instrumented hardware and a reviewed campaign. A passing synthetic workflow is not a physical qualification result.
