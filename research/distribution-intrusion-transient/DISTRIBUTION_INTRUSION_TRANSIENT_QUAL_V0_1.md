# Distribution Intrusion Under Pressure Transients — Research Design v0.1

**Tracking issue:** [#5145 DISTRIBUTION-INTRUSION-TRANSIENT-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5145)  
**Status:** research design and synthetic scenario contract  
**Evidence class:** design / synthetic-only  
**Evidence ceiling:** no physical ingress measurement, pathogen result, potability, health-risk, regulatory-compliance or field-performance claim  
**Snapshot:** 14 November 2026

## 1. Research question

Can a water-distribution operator bound the risk and extent of external contaminant ingress when an internal pressure transient coincides with a physical opening and external water head, and can the operator distinguish pressure recovery from verified restoration of water quality?

The causal mechanism is:

**external water + connected ingress pathway + external head exceeding internal pressure → ingress during the event → transport with subsequent hydraulic recovery**

The benchmark must not equate a pressure depression with confirmed ingress, nor equate the absence of a detected tracer with proof of no ingress when sampling coverage, calibration, detection limits or pathway observability are inadequate.

## 2. The gap this programme owns

The adjacent portfolio already owns pressure/leak diagnostics, pump-system service, sparse-sensor control, treatment barrier assurance, whole-chain sanitation, and independent restoration verification. This work owns the physical intrusion experiment and its bridge between hydraulic events and water-quality evidence.

| Existing owner | Reused capability | This programme adds |
|---|---|---|
| #4963 leak diagnostics | Detection, localization, defect/event labels | Ingress through a known defect and tracer transport |
| #4959 / #4994 pump service and T1 | Clean-water pumping rig and duty boundary | Transient event generation and external-head defect fixture |
| #5018 sparse-sensor control | Control baselines and abstention | Ground-truth link between pressure history and measured ingress |
| #5136 treatment assurance | Claim-scoped water-quality evidence | Event-specific sampling trigger and prevention of premature clearance |
| #5069 recovery verification | Stable dwell and independent restoration evidence | Distinguish pressure restoration from water-quality recommissioning |
| #4398 EPANET qualification | Network steady-state baseline within its feature limits | Require separately qualified transient analysis; no steady-state surrogate |
| Mycelix metrology / water-quality | Instrument, calibration, observation and assessment contracts | Reuse their IDs and provenance, not new registries |

## 3. Evidence from prior experimental work

The phenomenon is established experimentally. Laboratory work has used an engineered pipe defect surrounded by porous media, imposed pressure transients, and measured external tracer entry and downstream transport. Related studies have quantified intrusion volume under different transient conditions and tested intermittent-supply cases.

Primary comparators:

- Fox et al. (2014), *Experimental Proof of Contaminant Ingress into a Leaking Pipe During a Transient Event*, DOI [10.1016/j.proeng.2014.02.073](https://doi.org/10.1016/j.proeng.2014.02.073).
- Fox et al. (2014), *Experimental Quantification of Intrusion Due to Transients in Distribution Systems*, DOI [10.1016/j.proeng.2014.11.444](https://doi.org/10.1016/j.proeng.2014.11.444).
- Fox et al. (2015), *Experimental Quantification of Contaminant Ingress into a Buried Leaking Pipe during Transient Events*, DOI [10.1061/(ASCE)HY.1943-7900.0001040](https://doi.org/10.1061/(ASCE)HY.1943-7900.0001040).
- Fontanazza et al. (2015), *Contaminant Intrusion through Leaks in Water Distribution System: Experimental Analysis*, DOI [10.1016/j.proeng.2015.08.904](https://doi.org/10.1016/j.proeng.2015.08.904).
- US EPA, *Distribution System Indicators of Drinking Water Quality*, section on pressure: [official fact sheet](https://www.epa.gov/sites/default/files/2021-05/documents/issuepaper_tcr_indicators_posted.pdf).

These are evidence of the mechanism under the authors' test conditions, not a transferable performance guarantee. Reproduction and uncertainty characterization precede any claimed improvement.

## 4. Testable causal structure

Each scenario must represent five separate things:

1. **Event:** pump trip, power loss, valve operation, main break, demand change, intermittent supply or controlled restart.
2. **Hydraulic boundary:** internal pressure trace and external water head at the defect, each on a compatible time base.
3. **Pathway:** known/unknown defect geometry, connectivity, pipe material and external porous-media condition.
4. **Ingress observation:** measured tracer concentration/mass and flow/volume with calibration and uncertainty.
5. **Transport observation:** time-stamped downstream samples at identified locations, with method, detection/quantification limits and coverage.

No output may infer a positive water-quality disposition from hydraulic pressure alone.

### 4.1 First-pass factor matrix

| Factor | Initial levels to freeze after rig review |
|---|---|
| Pressure event | pump trip; controlled speed change; fast/slow valve operation; intermittent supply; pressure recovery |
| Pressure relation | internal pressure always above external head; brief crossing; sustained crossing; sensor evidence incomplete |
| Defect | no defect; calibrated orifice; repeatable crack surrogate; appurtenance fixture |
| External medium | no contact; saturated porous pack; controlled external tracer reservoir |
| Pipe configuration | declared material/diameter/wall; replaceable defect spool; independent fixture revision |
| Sampling | upstream; immediately downstream; far-end; point-of-use analogue; missing/stale/insufficient-coverage |
| Instrument status | valid; drifted; clipped; stale calibration; time offset; common reference failure |

These are experimental dimensions, not final numeric operating limits. The rig design must determine pressure ranges, pipe class, transient bandwidth and sensor range from the selected hardware and competent review.

### 4.2 Minimum negative controls

- Pressure transient with no defect and no external tracer.
- Defect with no external liquid contact.
- External tracer and defect, but internal pressure remains above external head.
- Positive-pressure leak-out without a pressure deficit.
- Similar pressure signature from two different pump/valve events.
- A sample below detection/quantification limit with insufficient sensitivity to rule out the target tracer mass.
- Missing downstream sample despite a valid local pressure record.
- Pressure restored and maintenance ticket closed, but no independent post-event water-quality evidence.

## 5. T1 cell concept

**T1 is a closed, non-potable, contained clean-water tracer experiment.** It is not a drinking-water supply or treatment trial.

Proposed flow path:

non-potable reservoir → controlled pressure source → instrumented pipe loop → interchangeable defect spool → downstream sample section → captured return

The defect spool sits in a physically segregated external-head / porous-media chamber. The external tracer reservoir must not have any hydraulic connection to a potable supply. All effluent is captured and disposed of under the approved test plan.

### 5.1 Required instrumentation

- Internal pressure at defined upstream/downstream points.
- External pressure/head adjacent to the defect.
- Reference flow and totalized volume.
- External tracer concentration and lot identity.
- Tracer sample identity, point, time and concentration downstream.
- Reservoir level/volume and water temperature.
- Pump/drive/valve command and independently observed state.
- Instrument calibration, valid range, response/bandwidth, uncertainty and clock synchronization.

Pressure sampling frequency must follow the expected transient bandwidth and the installed sensor/DAQ response. Demonstrate anti-aliasing and time alignment; do not select a rate by convention alone.

### 5.2 Mechanical/electrical safety hold points

Before procurement, fabrication, filling or energization, a competent reviewer must approve the pressure boundary, pump shut-off conditions, relief capacity and return path, restraints, trapped-pressure release, guards, wet-zone electrical segregation, overcurrent protection, earthing, emergency stop and containment. A software interlock is not a pressure relief device. This document does not set a relief pressure, pipe pressure class, motor size or safe-to-energize state.

Do not use live pathogens, wastewater, sewage, floodwater or hazardous chemicals in this initial T1 cell. Use only a non-hazardous tracer compatible with the materials and approved waste route.

## 6. Measurements and metrics

### Primary physical measurements

- Minimum internal pressure and pressure-time trajectory.
- Duration and magnitude of external-head-over-internal-pressure conditions.
- Measured tracer ingress volume/mass and uncertainty.
- Tracer mass-balance closure across measured streams.
- First downstream detection time and concentration-time curve at each sample location.
- Residual tracer mass and transport completeness over the declared observation window.

### Diagnostic and operational metrics

- Intrusion detection precision/recall only where direct experimental ground truth exists.
- False-safe rate on the frozen critical controls.
- Abstention rate and unresolved-case rate.
- Ingress-volume interval coverage and width.
- Downstream arrival-time error and concentration prediction interval coverage.
- False alarm burden per declared test exposure, not only accuracy.
- Time to identify affected pipe segment and notify the responsible operator.
- Time to independently verified recommissioning; pressure return is a separate metric.
- Affected population/service duration and alternative-supply coverage when a site study is eventually authorized.

Tracer concentration is not pathogen concentration or health risk. A clean tracer test does not certify microbiological or chemical safety.

## 7. Model comparison and fairness

Compare methods against identical frozen cases and deployment information:

1. steady-state hydraulic model, restricted to the questions it can answer;
2. separately qualified transient-hydraulic reference method;
3. pressure-threshold / rule baseline;
4. pressure plus known pathway and external-head context;
5. direct tracer sampling as experimental ground truth;
6. candidate inference system with explicit abstention.

The transient model must first reproduce the measured pressure trace within a pre-registered, uncertainty-aware tolerance. It may not be used for ingress prediction until this upstream physical validation is satisfied. Do not implement a new transient solver in this workstream; select and pin an independently qualified tool/method.

Use the shared matched-trial contract (#5075) for sensor parity, time grid, topology, demand trace, actuator envelope and resource disclosure. Split by physical event, defect fixture, media packing and hardware revision, not by randomly selected adjacent samples. Publish failures, censored measurements and no-results.

## 8. T0 scenario contract

The synthetic inputs are in [intrusion-t0-scenarios-v0.1.json](intrusion-t0-scenarios-v0.1.json), and the expected outputs are frozen separately in [intrusion-t0-oracle-v0.1.json](intrusion-t0-oracle-v0.1.json). The manifest binds the two files and their ordered case identities.

It is deliberately a **state/evidence contract**, not a pressure-transient simulator. Each vector tests a distinct decision boundary: pressure event without evidence of ingress; direct tracer evidence; unknown pathway; missing sample; stale calibration; insufficient detection limit; mass-balance gap; downstream detection; and pressure-restoration-only claims.

The T0 evaluator should later:
- validate exact scenario IDs and evidence-class ceilings;
- recompute declared tracer mass and mass-balance arithmetic only;
- reject missing, duplicated or misbound sample and instrument identities;
- preserve INDETERMINATE when coverage, calibration, method or detection limit cannot support a negative conclusion;
- distinguish INGRESS_CONFIRMED_IN_TEST, INGRESS_NOT_DEMONSTRATED, and INDETERMINATE;
- never emit potability, health-risk, compliance, release or operation authority;
- produce a deterministic receipt bound to input, oracle, schema, implementation revision and run.

## 9. Evidence tiers and gates

| Tier | Permitted claim | Gate |
|---|---|---|
| T0 | Synthetic arithmetic and evidence semantics | Exact IDs, independent oracle, negative controls, deterministic receipt |
| T1 | Measured tracer behaviour in a named closed rig | Approved design, instrument calibration, repeated runs, mass balance and uncertainty |
| T2 | Named non-hazardous or biological/chemical lab method result | Qualified laboratory, validated method, chain of custody and scope-matched competence |
| T3 | Named, supervised site observation | Asset-owner and authority approval, site-specific water safety plan, safe alternative supply and independent surveillance |
| T4 | Replicated transfer claim | Independent sites, materials, conditions, operators and seasonal profiles |

### Hard qualification gates

- Zero false-safe outcomes in the frozen critical T0 corpus.
- No ingress/no-ingress decision from pressure alone when pathway, external head, sample coverage or method validity is missing.
- Every physical ingress quantity binds to instruments, calibration, geometry, external tracer lot/concentration, sample points and uncertainty.
- Tracer mass balance closes or is explicitly unresolved.
- No steady-state solver is used as transient safety evidence.
- No T0/T1 result becomes a microbial, chemical, potability, health, regulatory or field claim.
- No model authorizes isolation, public notification, restart or water release.

### Research objectives (not achieved)

1. Reproduce one published pressure-transient ingress experiment with a tolerance pre-registered from the reference data and the rig's uncertainty budget.
2. Establish repeatability across at least three independently reset runs per frozen condition.
3. Identify the minimum sensor and sampling layout needed to bound ingress and downstream arrival.
4. Demonstrate whether pressure + pathway + external-head evidence materially improves ingress-volume interval coverage over pressure-only rules.
5. Demonstrate that any reduction in response latency does not increase false-safe outcomes.

Do not set a percentage “better than SOTA” target until the physical reproduction and uncertainty budget are accepted.

## 10. South African deployment boundary

For a South African site-specific profile, use the applicable current SANS 241 edition, the controlled SABS text and exact Water Services Institution obligations. Rand Water has published a 2026 migration presentation specifically addressing SANS 241:2015 to SANS 241:2025 and describes the change as an integrated risk-assessment, monitoring, response and evidence model. Obtain the controlled SABS standard and verify its current edition, clauses and applicability before encoding numerical water-quality limits.

The Revised Compulsory National Water and Sanitation Services Standards (Government Gazette 52814, 6 June 2025) provide quantitative service-floor anchors for South African scenarios, in the scope set by the Regulations:

- Basic water service: at least 6 kL per household per month, at no less than 10 L/min, available at least 358 days per year, and not interrupted for more than 48 consecutive hours; supplied water must meet the applicable SANS 241 prescripts.
- Where supply is interrupted for more than 48 hours: alternative potable supply of at least 15 L per person per day at strategically selected, relatively convenient and safe delivery points.
- Water Services Institutions must maintain water safety plans, review them at least annually and when risk changes, and maintain a monitoring programme specifying sampling locations, frequency and parameters. Laboratory analysis must use ISO/IEC 17025-accredited water-analysis methods or another qualifying credible/reliable system; incidents and advisories have defined notification and response duties.
- The Regulations also require provision for generators or similar at critical water and wastewater infrastructure during power disruptions, development of alternative electricity sources, and protection of that infrastructure.

These are South African regulatory service obligations, not universal global design values and not proof that a specific local service is currently achieving them. The benchmark should represent them as hard, versioned scenario constraints, keep quality separate from quantity/continuity, and measure the worst-served area rather than allow high service elsewhere to compensate. The WHO Guidelines for Drinking-water Quality 2026 provide the current international health-based / water-safety-plan framework. None of these obligations is replaced by this research software.

- [Rand Water: Migrating from SANS 241:2015 to SANS 241:2025](https://www.randwater.co.za/media/forums/presentations/RW%20-%20Migrating%20from%20SANS%20241%202015%20to%20SANS%20241%202025.pdf)
- [Government Gazette 52814 (2025)](https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf)
- [WHO Guidelines for Drinking-water Quality (2026)](https://www.who.int/publications/i/item/9789240121225)

## 11. Nonclaims

This design does not establish occurrence or prevalence of intrusion in any utility, health risk, pathogen transport, chemical quality, SANS compliance, water potability, treatment efficacy, transient model validity, equipment safety, repair success, or field performance. No physical test has yet been conducted.