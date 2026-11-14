# Maritime Engineering Health Benchmark v0.1

**Tracking:** [MARITIME-ENGINEERING-HEALTH-001 #4444](https://github.com/Luminous-Dynamics/symtropy/issues/4444)  
**Owners composed:** Symtropy #1612 / #3893 / #4406; Symthaea #11008 / #11185; Mycelix #5974  
**Status:** synthetic contract benchmark and offline evaluator; no ship, machinery bench, OT integration or human trial

## Objective

Qualify the evidence used to determine whether shipboard engineering functions remain available, degraded, unknown or eligible for independent restoration review. This is not a machinery diagnostic product or a physical model.

The benchmark moves from "one alarm / one health score" to a **proof-carrying capability state**:

asset identity → configuration → observations → source/dependency lineage → fault hypotheses → affected capability → bounded disposition → recovery evidence

A condition score or model posterior is not a safety proof. A missing alarm is not evidence of health. Multiple derived signals sharing a sensor, power rail, controller, clock or network are not independent corroboration.

## Regulatory and industry context

- The IMO adopted the non-mandatory MASS Code (MSC.595(111)) on 22 May 2026; it took effect on 1 July 2026. Its goal-based framework covers design and operation, including machinery/electrical installations, connectivity, remote operations, fire safety, risk assessment and human oversight. The mandatory-code roadmap is later; this benchmark is not a compliance implementation. [IMO announcement](https://www.imo.org/en/mediacentre/pressbriefings/pages/imo-adopts-mass-code.aspx)
- DNV AROS treats navigation, engineering, operational and safety functions separately, and distinguishes remote control, decision support, supervised autonomy and full autonomy, with onboard/off-ship/hybrid control location. This supports per-function capability qualification rather than one scalar autonomy score. [DNV AROS](https://www.dnv.com/maritime/autonomous-remotely-operated-ships/aros-class-notation/)
- ISO 17359 provides general condition-monitoring programme guidance across vibration, temperature, tribology, flow, contamination, power and speed. ISO 13374-1 specifies general guidelines for condition-monitoring diagnostic data processing, communication and presentation; its 2003 edition was confirmed current in 2025. These are process/data baselines, not ready-made MASS performance leaderboards. [ISO 17359](https://www.iso.org/standard/71194.html) · [ISO 13374-1](https://www.iso.org/standard/21832.html)
- IACS UR E26/E27 define ship-wide and onboard-system/equipment cyber-resilience requirements. Revised requirements apply to ships contracted for construction on or after 1 July 2024 within the applicable vessel/system scope; ClassNK notes that scope conditions matter and that the URs are not designed specifically for advanced/autonomous technologies. Cybersecurity is a necessary constraint on the evidence path, not proof that a diagnostic inference is physically correct. [ClassNK UR E26/E27](https://www.classnk.or.jp/hp/en/activities/cybersecurity/ur-e26e27.html)

## V0 data contract

Each case binds a vessel, asset, component, configuration revision and capability. Each observation retains:
- its own identifier, signal, monotonic observation offset and installed configuration revision;
- source role and issuer-scoped source reference: measured sensor, native equipment alarm, operator report, maintenance record, model residual, external diagnostic or simulator-only;
- quantity unit, calibration/verification reference, installed configuration revision and declared validity-profile reference;
- observation clock domain and quality (monotonic + UTC bound, UTC-only, conflicted or unknown), plus the case's decision clock domain;
- source observation UTC-mapped timestamp, case decision timestamp and maximum admissible age, so the verifier computes age as decision time minus source observation time rather than trusting a producer-supplied age;
- uncertainty reference (a pointer to the uncertainty method/artifact; v0.1 does not invent a numeric uncertainty model);
- quality state: current, stale, wrong configuration, wrong unit, expired calibration, unauthenticated, conflicted, simulator-only or missing lineage;
- a declared shared-dependency group (sensor, power rail, controller, clock, network, or another named common cause).

Admission requires current quality, exact configuration match, non-simulator source role, a monotonic+UTC-bound clock, exact observation/decision clock-domain match and age no greater than the declared maximum. Operator reports and maintenance records remain supporting evidence; they are not silently promoted into direct telemetry or equipment measurements. Decision evidence is limited to measured sensors, native equipment alarms, model residuals and external diagnostics, and still carries its original source role. A `current` label cannot override an expired age or conflicted/unknown clock.

Distinct named dependency groups are counted as **declared dependency separation only**. That count is not a statistical independence proof. Unknown or blank dependency lineage contributes zero declared separation.

A recovery attempt requires a repair record, inspection, proof test, scoped authorization, reconciled configuration and current decision evidence. A complete record only produces restoration_evidence_complete for independent review. It never actuates machinery or clears a restriction by itself.

## Synthetic corpus

The frozen corpus contains 41 positive, negative, ambiguity, common-cause, recovery and metamorphic fixtures across propulsion, steering, generation, switchboards, battery, cooling, lubrication, fuel, bilge, flood, fire, ballast and HVAC/electronics cooling.

Each case carries function-specific vessel asset, component, installed-configuration and capability identity; the current synthetic set covers 11 capability classes rather than assigning every casualty to a generic propulsion asset. These are fixture identities, not production vessel asset records.

The fixtures test contract semantics, not distributions of real ship faults. They deliberately do not invent pressure, temperature, vibration, bilge, insulation, combustion, fire-growth, flood-propagation, stability or electrical-transient thresholds.

## Evaluator and gates

The standalone Rust evaluator checks strict typed parsing, corpus identity/version, minimum size, unique case and observation IDs, non-empty identity/lineage fields, monotonic observation offsets, source-role boundaries, exact configuration match, explicit clock quality and exact observation/decision clock-domain match, recomputed age from decision_time_ms - observed_at_ms versus maximum age, including future-dated source rejection, dependency-group accounting, fault/unknown dispositions and recovery evidence completeness.

Non-compensatory gates:
- zero admission of stale, wrong-configuration, wrong-unit, expired-calibration, unauthenticated, conflicted or simulator-only observations as current decision evidence;
- zero source-role promotion of operator reports or maintenance records into direct telemetry;
- zero false independence from multiple observations sharing a declared dependency group;
- unknown, unsupported or evidence-free cases never default to healthy;
- zero restoration evidence completeness without all required recovery records;
- positive controls must continue to exercise the non-restrictive path;
- the tool has no OT network, controller, protection system, actuator or equipment side effects.

The CLI emits the SHA-256 of the exact input bytes, expected-versus-actual outcomes and aggregate case counts. A PASS means only that this reference evaluator reproduces the declared outcomes for the frozen synthetic input.

## Baseline plan and target metrics

Compare all arms on the same time-aligned observations, asset configuration, evidence availability, event tape and compute budget:

1. documented native alarm threshold/logic (when semantics and permission to represent it are available);
2. rule-based fault tree or FMEA-derived diagnostic workflow;
3. qualified physics/model residual (only where a validated model and reference data exist);
4. anomaly detector with explicit unknown/OOD output;
5. Symthaea evidence synthesis on the identical admitted evidence;
6. ablations removing dependency lineage, calibration freshness, configuration identity, source roles or temporal order.

Report, by asset class, fault family, severity and operating regime:
- missed-fault rate, detection latency and lead time;
- false alarms per asset-hour and nuisance-alert burden;
- isolation precision/recall and unresolved-hypothesis coverage;
- calibration (Brier score / reliability curves / interval coverage where probabilities are emitted);
- unknown/OOD detection and false-safe rate;
- false-independence rate under common-cause injection;
- time to capability restriction recommendation;
- false restriction and restricted-capability duration;
- recovery evidence completeness and false restoration rate;
- event/configuration/source trace completeness and deterministic replay agreement.

No single weighted score is acceptable: false-safe, invalid evidence admission, source-role promotion and false restoration are hard gates. False restriction, coverage, latency and operator workload remain visible utility metrics. A system that abstains on everything fails positive and maintain-capability controls.

No defensible numerical SOTA claim exists from this synthetic corpus. Numerical comparison begins only when an independently curated ship/bench dataset, lawful data rights, component-specific labels, qualified reference instrumentation and matched baselines are available. Publish confidence intervals and per-domain breakdowns; do not pool unlike vessels or operating regimes.

## Integration boundary

- Symtropy #1612 supplies deterministic, effects-disabled scenario replay; this evaluator does not create another runtime.
- Symtropy #4406 supplies asset/configuration identity and commissioning lineage.
- Symtropy #3893 consumes capability impacts in its ODD and mode logic; this crate does not redefine the ODD.
- Symthaea #11185 consumes bounded read-only dispositions; #11008 consumes service availability/reserve states. Neither is granted machinery or navigation authority.
- Mycelix #5974 records provenance, issuer authority, revocation and owner handoffs; it does not certify a physical fault.
- A qualified engine-room or machinery-bench data partner and independent marine engineer must review any later physical benchmark.

### ODD capability-impact handoff

The evaluator emits a per-case, asset- and configuration-bound **engineering evidence contribution**. It does not emit an ODD disposition. There is deliberately no one-to-one mapping from a machinery-health result to `WithinDeclaredEnvelope`, `LimitApproaching`, `LimitExceeded`, `NeedsEvidence`, `EvidenceConflict`, `Unsupported` or `HumanDecisionRequired`.

| Engineering-health result | Emitted capability status | Required treatment by the ODD/mode owner |
|---|---|---|
| RestrictCapability | restriction_recommended | Preserve as a scoped constraint/recommendation for the named asset, component, configuration revision and capability. The ODD owner decides whether it violates a declared function prerequisite or mode guard; it is not automatically a vessel-level LimitExceeded result. |
| ContinueWithWatch | no_restriction_indicated | Means only that this case's admitted evidence did not produce a new restriction recommendation. It is not positive proof of the full operating envelope, does not produce WithinDeclaredEnvelope and does not clear a previously latched restriction. |
| UnknownEscalate | unknown_needs_evidence | Preserve the evidence gap. The ODD owner determines NeedsEvidence versus Unsupported from its own profile, applicability and required-source contract; this status is not a permissive default. |
| RestorationBlocked | recovery_blocked | Keep the named capability restricted. Do not infer restoration from a partial repair, inspection or test record. |
| RestorationEvidenceComplete | recovery_evidence_ready_for_independent_review | Exposes a review packet only. The ODD owner may require HumanDecisionRequired under its own transition policy, but this status never performs a mode transition or clears a restriction. |

Each emitted capability-impact projection now carries the exact `corpus_sha256` itself, in addition to its case and asset/configuration identity. The report root repeats the same digest; the evaluator asserts this equality in its test suite. The handoff key is therefore the corpus digest plus `case_id`, `vessel_id`, `asset_id`, `component_id`, `config_revision` and `capability`. The projection contract is frozen in [`MARITIME_ENGINEERING_HEALTH_CAPABILITY_IMPACT_V0_1.schema.json`](./MARITIME_ENGINEERING_HEALTH_CAPABILITY_IMPACT_V0_1.schema.json). A consumer must join this evidence to an explicit vessel-specific function allocation and ODD profile. Missing or ambiguous joins remain unsupported/unknown; do not infer a capability mapping from a component name alone.

The projection carries `no_authority_granted`, `may_authorize_transition=false` and `may_clear_restriction=false`. It intentionally contains no ODD disposition or transition command. Symtropy #3893 owns the ODD decision; Symthaea #11185 owns mode/authority/contingency semantics; Mycelix #5974 pins provenance and exact owner-native artifacts. This is an evidence handoff, not a second ODD evaluator or an authorization.
 
## Validation ladder

- E0: schema, identity, corpus uniqueness and monotonicity.
- E1: deterministic evaluator plus mutation tests.
- E2: exact-head CI, frozen corpus and independent replay.
- E3: owner-native asset, ODD, mode and provenance artifacts.
- E4: permissioned real telemetry and qualified reference labels, with data rights and independent review.
- E5: validated component/physical models and controlled bench trials.
- E6: vessel-specific approval and class/flag/operator processes.

Synthetic PASS is E1/E2 only. It is not diagnostic accuracy, machinery-health evidence, seaworthiness, class/statutory compliance, maintenance authorization, MASS Code compliance or permission to operate.


## External operational-vessel comparator — reproduce first, then challenge the split

The registry contains a 2025/2026 operational-vessel study that reports 99.72% balanced accuracy for a seven-class SVM fault-severity diagnosis. It is a meaningful comparator: the engine was a 1.89 MW auxiliary generator on a cargo/container vessel, and the two fault mechanisms were physically induced onboard. It is not a fleet result: interventions were on one auxiliary engine, at 825/900/975 kW steady load points, with no dynamic load campaign or independent-vessel holdout described.

The paper reports an 80/20 split and 10-fold cross-validation but does not establish whether samples were grouped by intervention, run or chronology. The reproduction sequence is:
- recover the original data contract, signal list, class counts, split grouping, preprocessing and threshold-fit scope;
- reproduce SVM 99.72% balanced accuracy and the SAE/UAE anomaly baselines under the stated setup;
- run chronological block holdout and complete intervention/run-held-out confirmation, plus leave-one-load-point-out within the three tested steady loads;
- retain class imbalance (about 1.3% fault observations) and report macro-F1, per-severity recall, calibration, event recall, false alarms per valid engine-hour and detection latency;
- reproduce the weak intercooler-only detection case (SAE 75.0% balanced accuracy / 63.1% F1; UAE 51.55% / 6.04%) rather than reporting only the combined task headline;
- evaluate display-extracted thermodynamics, vibration-only and fused inputs separately, with acquisition uncertainty and sensor dependency provenance.

The abstract labels the SAE combined result 87.6% accuracy, while the detailed table reports 98.15% ordinary accuracy and 87.57% balanced accuracy; retain the table's metric definitions. Data access is restricted to institutional/partner agreements and a request process. No dataset SHA or result reproduction is claimed here.
