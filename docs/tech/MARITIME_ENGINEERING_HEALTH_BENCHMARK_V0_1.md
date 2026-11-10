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
- ISO 17359 provides general condition-monitoring programme guidance across vibration, temperature, tribology, flow, contamination, power and speed. ISO 13374-1 specifies general guidelines for condition-monitoring diagnostic data processing, communication and presentation; its 2003 edition was confirmed current in 2025. These are process/data baselines, not ready-made MASS performance leaderboards. [ISO 17359](https://www.iso.org/standard/711.html) · [ISO 13374-1](https://www.iso.org/standard/21832.html)
- IACS UR E26/E27 define ship and onboard-system cyber-resilience requirements, effective for applicable newbuild contracts from 1 July 2024. They are not autonomous-ship-specific; cybersecurity is a necessary constraint on the evidence path, not proof that a diagnostic inference is physically correct. [ClassNK UR E26/E27](https://www.classnk.or.jp/hp/en/activities/cybersecurity/ur-e26e27.html)

## V0 data contract

Each case binds a vessel, asset, component, configuration revision and capability. Each observation retains:
- its own identifier, signal, monotonic observation offset and installed configuration revision;
- source role: measured sensor, native equipment alarm, operator report, maintenance record, model residual, external diagnostic or simulator-only;
- quality state: current, stale, wrong configuration, wrong unit, expired calibration, unauthenticated, conflicted, simulator-only or missing lineage;
- a declared shared-dependency group (sensor, power rail, controller, clock, network, or another named common cause).

Only current, correctly configured, non-simulator observations are admitted. Operator reports and maintenance records remain supporting evidence; they are not silently promoted into direct telemetry or equipment measurements. Decision evidence is limited to measured sensors, native equipment alarms, model residuals and external diagnostics, and still carries its original source role.

Distinct named dependency groups are counted as **declared dependency separation only**. That count is not a statistical independence proof. Unknown or blank dependency lineage contributes zero declared separation.

A recovery attempt requires a repair record, inspection, proof test, scoped authorization, reconciled configuration and current decision evidence. A complete record only produces restoration_evidence_complete for independent review. It never actuates machinery or clears a restriction by itself.

## Synthetic corpus

The frozen corpus contains 36 positive, negative, ambiguity, common-cause, recovery and metamorphic fixtures across propulsion, steering, generation, switchboards, battery, cooling, lubrication, fuel, bilge, flood, fire, ballast and HVAC/electronics cooling.

The fixtures test contract semantics, not distributions of real ship faults. They deliberately do not invent pressure, temperature, vibration, bilge, insulation, combustion, fire-growth, flood-propagation, stability or electrical-transient thresholds.

## Evaluator and gates

The standalone Rust evaluator checks strict typed parsing, corpus identity/version, minimum size, unique case and observation IDs, non-empty identity/lineage fields, monotonic observation offsets, source-role boundaries, configuration match, quality admission, dependency-group accounting, fault/unknown dispositions and recovery evidence completeness.

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

The evaluator emits a per-case, asset- and configuration-bound capability-impact projection for downstream owners:

| Engineering-health result | Projection to the ODD/mode owner |
|---|---|
| RestrictCapability | RestrictionRecommended; may contribute to LimitExceeded / degraded mode review |
| ContinueWithWatch | NoRestrictionIndicated for this evidence only; never upgrades the whole vessel to WithinDeclaredEnvelope |
| UnknownEscalate | UnknownNeedsEvidence; maps to NeedsEvidence or Unsupported |
| RestorationBlocked | RecoveryBlocked; restriction remains in force |
| RestorationEvidenceComplete | RecoveryEvidenceReadyForIndependentReview; maps to HumanDecisionRequired, never automatic restoration |

The output carries vessel, asset, component, exact configuration revision, capability, evidence counts and declared dependency-group count. It declares NoAuthorityGranted, may_authorize_transition=false and may_clear_restriction=false. Symtropy #3893 and Symthaea #11185 remain the decision-support/mode owners; Mycelix #5974 pins the exact report and provenance. This is an input projection, not a duplicate ODD evaluator or an authorization.

## Validation ladder

- E0: schema, identity, corpus uniqueness and monotonicity.
- E1: deterministic evaluator plus mutation tests.
- E2: exact-head CI, frozen corpus and independent replay.
- E3: owner-native asset, ODD, mode and provenance artifacts.
- E4: permissioned real telemetry and qualified reference labels, with data rights and independent review.
- E5: validated component/physical models and controlled bench trials.
- E6: vessel-specific approval and class/flag/operator processes.

Synthetic PASS is E1/E2 only. It is not diagnostic accuracy, machinery-health evidence, seaworthiness, class/statutory compliance, maintenance authorization, MASS Code compliance or permission to operate.
