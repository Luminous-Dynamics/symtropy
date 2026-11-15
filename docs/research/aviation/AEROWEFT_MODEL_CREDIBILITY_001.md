# AeroWeft — Model Credibility and Simulation-to-Flight Transfer

**Tracking issue:** #5362  
**Parent capability:** Track R / #5354 performance envelope  
**Status:** synthetic-first evidence-envelope checker  
**Claim ceiling:** structural admissibility of a model-validation package for further review only; no proof of physical model validity, aircraft certification or operational safety.

## Research question

Within which aircraft, sensor, environment, traffic and control-response domains are modelled results credible enough to support a stated performance claim?

A simulator can generate millions of trajectories and still be systematically wrong. Synthetic known-truth tests validate code behaviour against the synthetic model. They do not independently validate the synthetic model against physical aircraft, sensors, weather or control response.

## Paradigm shift: model identity plus operating envelope

Move from "the simulator ran N scenarios" to:

> This exact model build is supported by independently adjudicated evidence in this named operating domain; prediction error and uncertainty are characterized by horizon and condition; and claims outside that domain receive no performance credit.

A model claim binds exact:
- model artifact;
- aircraft configuration, mass/CG, payload and propulsion;
- control/guidance software;
- sensor/receiver/calibration/placement profile;
- operating-domain and traffic/encounter profile;
- solver/integration/numerical configuration;
- coordinate frame, vertical datum, time basis and prediction horizon.

Changing any of these means the old performance credit cannot be silently inherited. Similar labels or a shared aircraft family are not equivalence evidence.

## Ownership

Track T owns validation-package structure, calibration/confirmatory split checks, applicability-envelope identity, metric/stratum policy, model-to-evidence traceability and synthetic adversarial fixtures.

It composes:
- Track H / #5163 reachable occupancy;
- Track L / #5270 PNT and dual-clock admission;
- Track M / #5275 surveillance observability;
- Track N / #5284 intent conformance;
- Track O / #5304 cross-layer composition;
- Track P / #5337 causal receipt binding;
- Track Q / #5351 authenticated trust custody;
- Track R / #5354 performance envelopes;
- Track S / #5359 multi-subject evidence scope;
- Symthaea's aircraft-model, control-response and DAA owners.

Track T does not duplicate aircraft dynamics, PNT, surveillance, conflict or conformance algorithms. Its Rust crate checks package structure and preregistered result gates only.

## Evidence modalities are not a universal ladder

Keep these modalities distinct: synthetic known-truth, analytic reference, software-in-the-loop, hardware-in-the-loop, calibrated component bench, controlled flight test and shadow-mode field replay.

They are not totally ordered. A flight test may validate one component and not another; HIL can validate interfaces that a flight test does not isolate. An evidence profile must state the modality required for each metric and stratum. No automatic promotion is allowed.

Operational acceptance is an external authority decision. AeroWeft records it as supplied evidence, not as an inference from this crate.

## Validation protocol

1. Freeze calibration, development and confirmatory dataset manifests before viewing confirmatory outcomes.
2. Split by flight, aircraft configuration, route/geometry, environment and encounter family—not just random rows—to prevent leakage.
3. Compare predictions with independent reference measurements that carry identity, calibration, time, frame, uncertainty and adjudication lineage.
4. Report point error and interval/occupancy containment. A narrow interval that misses truth is not a success.
5. Separate observation noise, parameter uncertainty and structural model discrepancy.
6. Test extrapolation and distribution shift. Out-of-domain requests are NOT_COVERED or INDETERMINATE.
7. Validate numerical convergence and solver sensitivity independently of physical-model fidelity.
8. Compare full time histories and event ordering, not only endpoints or pooled averages.
9. Preserve missing, failed and censored adverse windows; never silently remove them.
10. Any retuning after confirmatory-set inspection invalidates that set for confirmatory credit.

## Metrics: no aggregate score

Report by horizon, aircraft/configuration, environment, traffic and encounter stratum:
- position, velocity, turn-rate and energy residual distributions;
- p95/p99 and worst observed residuals;
- prediction-interval coverage and width;
- reachable-set/occupancy containment;
- false-safe and missed-event outcomes;
- out-of-domain detection recall;
- model discrepancy and sensitivity to initial-state, parameter and environmental uncertainty;
- numerical convergence and deterministic replay;
- unsupported-stratum exposure and coverage;
- independent sample counts and confidence bounds.

A failed containment, missed event or unsupported stratum cannot be offset by a better RMSE, average alert latency, throughput or energy metric.

## Hard targets

Prospective targets—not results:
- 100% exact agreement with independent analytic oracles on bounded deterministic cases.
- Zero hard-bound containment failures on the frozen known-truth validation corpus.
- 100% report of interval coverage, denominator and confidence bounds for every required metric/stratum.
- Zero calibration/confirmatory group leakage.
- 100% explicit NOT_COVERED or INDETERMINATE for unsupported configurations or environments.
- Zero transfer of credit across changed model, aircraft, sensor, software, control or solver identity without owner-defined requalification.
- Exact deterministic replay for fixed model build, input manifest and solver configuration.
- Independent evaluator reproduces published residual and coverage metrics.
- Confirmatory results remain separate from development data.

Zero observed adverse events are not a zero-risk result. For rare-event metrics, report the exposure denominator and finite-sample upper confidence bound.

## Adversarial cases

The frozen fixture catalogue covers biased but low-RMSE calibration, high-tail errors hidden by pooled averages, crosswind-response misspecification, wrong transient dynamics with correct endpoints, flight/scenario leakage, mass/CG/payload changes, firmware/control-gain changes, sensor re-calibration/placement changes, frame/time mismatch, non-Gaussian residuals, correlated samples, omitted adverse windows, solver-step sensitivity, changed model build under an old identity, corridor-specific maneuverability failure and out-of-domain requests.

## Industry and research comparison

- **NASA-STD-7009B (active, dated 5 March 2024)** is a cross-domain model-and-simulation lifecycle standard. It requires project/program acceptance criteria to be set and approved by delegated technical authority and explicitly treats credibility products in development and use phases. Track T should map to its model credibility, uncertainty, sensitivity and use-assessment concepts; it must not describe its synthetic checker as NASA-standard compliant.
- **NASA-HDBK-7009B (active, dated 3 February 2026)** is the newly issued implementation handbook companion to NASA-STD-7009B. It gives practices, examples and implementation clarification beyond the standard's requirements. This is now a primary current comparator for the Track T evidence package and intended-use assessment.
- **AIAA R-154-2021** provides six recommended tasks where flight modelling is proposed to reduce flight testing for certification. It provides a useful engineering-task framework for model verification, application correctness, validation, uncertainty/analysis adequacy and applicability; it does not prescribe one universal model or simulation method.
- **ASTM F3705-25** is a testing guide aligned with F3442. It describes test methodologies/data collection and use of physical testing and statistical modelling; its scope notes that flight tests may be needed to validate fast-time simulation models, and describes verifying simulation models against flight-test efforts. It is limited to its stated small-UA/lower-risk crewed-traffic DAA applicability; it is not a universal standard for UA-to-UA, terrain, weather or all autonomous aviation.
- **RTCA DO-365C Change 1 (published 13 March 2025)** is a DAA MOPS comparator for UAS transiting Classes B/C/D/E/G and specified extended operations above 400 ft AGL, up to FL180 in D/E/G. It does not apply to small UAS below 400 ft or segmented areas. The RTCA/FAA status material located for this review still described DO-365D as in development in December 2025; a projected March 2026 publication date is not proof that it was published. Pin the exact revision before benchmarking.
- **EUROCAE ED-271A (published 22 November 2024)** is a proportionate-risk-based DAA Traffic MASPS for certified-category RPAS under IFR in Classes A–G. It is a distinct operating/assurance context from ASTM F3442; compare only matched assumptions.
- **EUROCAE WG-105 SG-1 public status** lists ED-329 (DAA MOPS A–G IFR) and ED-330 (very-low-level DAA MOPS) as drafts with target publication dates in March and June 2026 respectively. The targets have passed, but the public status still says draft; do not treat either as published requirements until the status changes.
- **ASTM F3442-26** is the DAA performance specification, but its stated scope is materially bounded: smaller UA maximum dimension ≤25 ft, airspeed below 100 kt, lower-risk airspace, and mixed cooperative/non-cooperative crewed-aircraft encounters. It excludes ARC-d and does not address bird or natural hazards such as weather, clouds, terrain/obstacles. Use it only as a matched baseline; it is not a general standard for all UAM/eVTOL, UA-to-UA, terrain-aware or high-risk operations.
- **ASTM F3201-26** is a software-dependability practice for UAS software and associated elements, and may also be used by third-party providers including UTM service providers and ADSPs. It expects lifecycle-wide configuration management and verification/validation; examples of methods include Monte Carlo, SIL, HIL and flight testing. It expressly does not establish technical or performance requirements for software components or third-party services. Use it as a lifecycle dependability/process comparator, not as a DAA threshold source or a claim that this crate or a green CI run is compliant.
- **FAA DAA Criteria for Making 44807 Determinations (DAA CMD, revision 11 March 2026)** is a voluntary manufacturer-submitted change/documentation framework that includes equipment and software alterations, complete test data and possible case-by-case operational evaluation. This supports explicit change-impact and requalification records, but does not turn a completed form into operational authority.
- **NASA/TP-20250003243 (1 July 2025)** reports a strategic flight-path-management evaluation with two helicopters and up to 330 virtual aircraft. The report describes successful core intent-based conflict functions but less successful flow-corridor maneuvers where tighter volumes reduced maneuvering degrees of freedom; it also reports an initial simulation validation using flight data.
- **NASA/TM-20240015613 (October 2024)** uses Monte Carlo scenarios and modeled collision probability per flight hour to evaluate strategic deconfliction and CMSA. It is a safety-model methodology comparator, not physical ground truth for AeroWeft.
- **ISO/TR 23310:2025** surveys UTM actors, services, functions, data and system-level functional/performance requirements. It is an ecosystem baseline, not a DAA model-validation standard.

References:
- RTCA DO-365C Change 1 (issued 2025-03-13): https://my.rtca.org/productdetails?id=a1BPP00000DpH5l2AF
- EUROCAE ED-271A (published 2024-11-22): https://www.eurocae.net/product/ed-271a-minimum-aviation-system-performance-standards-for-detect-and-avoid-traffic-for-remotely-piloted-aircraft-systems-in-airspace-classes-a-g-under-ifr/
- EUROCAE WG-105 SG-1 deliverable status (ED-329/ED-330 drafts): https://www.eurocae.net/working-group/WG-105-SG-1/
- RTCA SC-228 / FAA status report context for DO-365D in-development state: https://www.rtca.org/wp-content/uploads/2026/01/7-A-December-2025-PMC-Report_Final_12.12.25.pdf
- NASA-STD-7009B: https://standards.nasa.gov/standard/nasa/nasa-std-7009
- NASA-HDBK-7009B (2026-02-03): https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009
- AIAA R-154-2021: https://webstore.ansi.org/standards/aiaa/AIAA1542021
- FAA DAA CMD (Revision 2026-03-11): https://www.faa.gov/uas/public_safety_gov/public_safety_toolkit/All_Purpose_CMD-DAA.pdf
- ASTM F3442-26: https://store.astm.org/f3442-26.html
- ASTM F3201-26 (scope, significance/use and current edition): https://store.astm.org/f3201-26.html
- ASTM F3705-25: https://store.astm.org/f3705-25.html
- ASTM F3442-26: https://store.astm.org/f3442-26.html
- ASTM F3201-26: https://store.astm.org/f3201-26.html
- NASA flight evaluation, NASA/TP-20250003243: https://ntrs.nasa.gov/citations/20250003243
- NASA CMSA safety assessment, NASA/TM-20240015613: https://ntrs.nasa.gov/citations/20240015613
- ISO/TR 23310:2025: https://www.iso.org/standard/87392.html

## Implementation

The dependency-free Rust crate is `tools/aeroweift-model-credibility/`. It checks exact scope, manifest split and group disjointness, metric-to-confirmatory-manifest binding, preregistered metric gates, conservative confidence-bound checks, independent sample-count floors and required-stratum coverage.

The positive disposition is `AdmissibleForDeclaredDomainReviewOnly`. The crate does not calculate residuals, inspect dataset bytes, verify sensor calibration, authenticate the reference evaluator or establish physical truth. Track Q must provide authenticated provenance where real evidence is introduced.

## Claim ceiling

A passing result means only that the submitted validation package is structurally admissible under the declared frozen profile. It does not prove model fidelity, universal predictive accuracy, safety, aircraft certification, regulatory compliance, flight authorization, or validity outside the named model and operating-domain scope.
