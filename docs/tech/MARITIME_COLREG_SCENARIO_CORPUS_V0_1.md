# Maritime COLREG Encounter Scenario Corpus v0.1

**Status:** synthetic scenario-catalogue seed for deterministic testing. Not a collision-avoidance controller, COLREG legal opinion, navigation system, manoeuvring model, MASS certification or proof of safe operation.

## Owners and boundaries

- Symthaea [TRANSPORT-MARINE-BENCH-001 #6260](https://github.com/Luminous-Dynamics/symthaea/issues/6260) owns the external benchmark registry, encounter prediction/evaluation protocol and claim ceilings.
- Symtropy [MARITIME-SIM-001 #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612) owns deterministic, effects-disabled scenario execution and replay.
- Symtropy [MARITIME-MANOEUVRING-001 #3865](https://github.com/Luminous-Dynamics/symtropy/issues/3865) owns qualified manoeuvring trajectory evidence.
- Symtropy [MARITIME-ODD-001 #3893](https://github.com/Luminous-Dynamics/symtropy/issues/3893) owns vessel-specific ODD/mode-transition profile.
- Mycelix [MARITIME-CROSS-STACK-GOLDEN-TRACE #5974](https://github.com/Luminous-Dynamics/mycelix/issues/5974) owns cross-owner authority / provenance negative controls.

This seed defines scenario identity, encounter-episode structure, temporal phases, track provenance, graph topology and synthetic expected labels. It does not implement the simulator, a predictor, a COLREG rule engine or physical manoeuvring.

## Research and benchmark anchors

- Wang et al., *Dynamic traffic encountering scenario generation for testing of ship collision avoidance algorithm*, Ocean Engineering 345, article 123691 (30 January 2026), DOI 10.1016/j.oceaneng.2025.123691. It proposes AIS encounter extraction using spatial and temporal dependence and scenario-analysis indicators: https://doi.org/10.1016/j.oceaneng.2025.123691
- Chen et al., *Spatiotemporal graph-based identification and classification of dynamic multi-ship encounter scenarios*, Ocean Engineering 356, article 125410 (30 May 2026), DOI 10.1016/j.oceaneng.2026.125410. It models encounter lifecycle from approach through CPA to departure and multi-ship interaction graphs: https://doi.org/10.1016/j.oceaneng.2026.125410
- Li et al., *A novel collision risk assessment method with spatiotemporal and COLREGs constraints*, Ocean Engineering 356, article 125202 (30 May 2026), DOI 10.1016/j.oceaneng.2026.125202. It compares velocity-obstacle / ship-domain, CPA and COLREG constraints in time-varying risk assessment: https://doi.org/10.1016/j.oceaneng.2026.125202
- ISO/AWI 25927 (MASS collision-avoidance scenario-generation guidance) is an approved work item under development, not a published performance standard: https://www.iso.org/standard/92071.html
- ISO/CD 25934.2 is a committee draft for autonomous-navigation verification procedures; ISO/AWI 25930 is an ODD/OE taxonomy work item. Neither is a published final standard: https://www.iso.org/standard/92078.html ; https://www.iso.org/standard/92074.html

The scenario corpus is a reproducible seed and must not be described as conforming to work items or draft standards under development.

## Paradigm shift

Move from a list of isolated "near-miss" snapshots to **complete, graph-structured encounter episodes**:
- approach;
- closest point of approach (CPA);
- departure;
- simultaneous/multi-vessel interactions;
- uncertainty, sensor observation gaps and rule-applicability ambiguity.

This prevents benchmark leakage from training only on the most visually obvious CPA moment and tests whether systems retain temporal and interaction context.

Keep distinct:
1. scenario generation and coverage;
2. encounter classification;
3. trajectory/intent prediction;
4. collision-risk ranking and calibration;
5. COLREG applicability and rule interpretation;
6. candidate manoeuvre generation;
7. closed-loop outcome under a qualified manoeuvring model;
8. human/authority decision and physical trial evidence.

A scenario replay PASS does not prove prediction skill. Prediction accuracy does not prove compliant or safe manoeuvring. A closed-loop simulation does not establish physical navigation safety or regulatory approval.

## Contract v0.1

Each synthetic episode binds:
- stable scenario ID, synthetic fixture identity, held-out geography/split group and exact episode revision;
- ownship and contact vessel identities/configuration classes;
- encounter class and declared rule-context label (or explicit ambiguous/unknown state);
- a complete sequence of approach, CPA and departure phases with increasing source times;
- source-owned track/observation records, sensor/profile identity, sample timestamps, position/speed/course units and missingness;
- encounter interaction graph nodes/edges with no dangling or duplicate identities;
- operating area, visibility, traffic density and declared environmental profile;
- time standard, horizontal CRS and position reference;
- exact generation seed, generator/model identity and scenario corpus version;
- synthetic expected disposition; no actual vessel, voyage or authority claim.

Source observation time, message/transaction time, retrieval time and evaluator cutoff must remain distinct. Repeated or delayed observations do not become current merely because they were ingested recently.

## Frozen v0.1 corpus

The corpus contains 30 cases:
- positive two-vessel head-on episode;
- crossing give-way / stand-on and overtaking label variants;
- restricted-channel and multi-vessel graph variants;
- ambiguous rule/encounter state;
- missing approach/CPA/departure phase;
- non-monotonic phase time;
- identity reuse, duplicate contact, duplicate track and dangling graph edges;
- missing source timestamp and stale contact;
- out-of-order samples and conflicting target tracks;
- sensor degradation hidden as complete data;
- missing geometry reference / CRS and unsupported operating domain;
- wrong split-group leakage across vessel/waterway;
- false confidence in rule context or authority;
- simulation/counterfactual scenario mislabeled as physical observation.

Three cases are expected to remain `SCENARIO_CONTRACT_VALID_SYNTHETIC_ONLY`; 27 negative cases have expected contract codes. The crossing/overtaking label variants are deliberately negative controls until their track geometry is changed to match the label; the ambiguous rule-applicability case preserves the clear head-on class while marking rule context/role Unknown. The corpus is a specification/fixture seed until an independent Rust oracle and mutation runner exist and pass exact-head checks.

## Benchmark metrics

- scenario-family coverage and complete-episode coverage (approach/CPA/departure);
- graph interaction coverage: pairwise vs multi-ship, degree distribution, conflict motifs and number of simultaneous encounters;
- encounter classification macro-F1/per-class recall and confusion by encounter type;
- trajectory prediction ADE/FDE by horizon and vessel class;
- uncertainty interval coverage/sharpness, Brier/log score and calibration by horizon;
- collision-risk ranking metrics and false-negative rate, kept separate from probability calibration;
- COLREG applicability abstention / ambiguous-rule disposition;
- closed-loop minimum separation, boundary/grounding outcomes and maneuver trajectory only when a qualified manoeuvring model is in scope;
- out-of-domain / missing-sensor detection and false-permissive rates;
- deterministic replay and split leakage detection.

Group splits by complete voyage, vessel identity/family, geography/waterway and time. Do not randomly split adjacent AIS points or let the same encounter episode enter more than one split. Hold out entire waterways and traffic-density regimes.

## Industry comparison

Compare matched scopes against:
- the reproducible 2,400-episode AIS-like Mendeley benchmark (10-minute observation history, 15-minute horizon, 4 encounter types), after exact version/licence review: https://data.mendeley.com/datasets/zbn3b9ppn2/1
- the COLREGs-aware encounter benchmark describing 146,246 encounters from two waterways; its raw Danish Maritime Authority AIS is not redistributed, so raw-data licensing and source availability must be separately resolved: https://github.com/ais-encounter-anon/colregs-encounter-benchmark
- analytic kinematic / hand-checkable encounter cases;
- declared velocity-obstacle, CPA, ship-domain and rule-based baseline implementations;
- ML trajectory baselines with matched input horizon, training split, compute and prediction horizon.

Do not claim "better than SOTA" from pooled accuracy. Publish source data version, filtering, label policy, voyage/vessel/waterway splits, training compute and all per-class calibration/error results. Any real AIS ingestion requires rights and privacy/security review. No raw operational AIS is included in this synthetic corpus.

## Claim ceiling

A passing corpus/evaluator establishes only that the selected synthetic scenario contract is internally valid and adversarial cases receive their expected structural dispositions. It does not establish correct COLREG interpretation, collision-avoidance competence, physical model validity, seaworthiness, MASS Code compliance, MASS Safety Certificate, real-vessel safety or navigation authority.
