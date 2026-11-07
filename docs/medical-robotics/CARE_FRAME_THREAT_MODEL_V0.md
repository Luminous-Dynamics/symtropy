# Careframe Threat Model v0 — Tool and Task Authority

**Scope:** concept-stage architecture threat model for synthetic simulation and later design review. It is not a formal cybersecurity risk assessment, a medical-device safety case, or authorization to power a robot.

## Security objective

A compromise, stale datum, ambiguous permission or planner mistake must not silently promote a proposed tool/action into physical authority. **Planning proposes; a separate policy gate authorizes the software-level simulation action; an independent safety controller owns physical stop/limits; accountable humans and the applicable governance process authorize research/clinical use.** No single model, plugin, credential or remote service should be able to collapse those layers.

## Assets to protect

- Tool identity, revision, geometry and interface digest.
- Task scope, contact class, force/payload/velocity limits and resource envelopes.
- World, calibration, localization and obstacle-state revisions.
- Human approvals, consent duration, privacy-shutter/capture state and restricted-zone permissions.
- Safety-controller configuration, stop chain, watchdogs, firmware and signed software updates.
- Qualification receipts, test corpus, provenance and claim ceiling.
- Patient/client data, recordings and operational telemetry.
- Availability of emergency stopping, local degraded behavior and a human handoff.

## Trust boundaries

1. **Symthaea planner** — untrusted for direct actuation. May submit a proposal and uncertainty; cannot change the gate or safety envelope.
2. **Tool designer / generator / plugins** — untrusted inputs. Manifest is parsed strictly; unknown fields and unsupported kinds reject.
3. **Clinical workflow adapter** — can provide scoped, time-bounded permissions only through reviewed interfaces. Natural-language instructions are not authority tokens.
4. **Independent task gate** — validates task/tool scope, exact revisions, limits, consent, location, permissions and evidence. It can reject or require a fresh plan.
5. **Low-level safety subsystem** — independently enforces stops and physical limits; must remain functional when the planner/network is unavailable.
6. **Build and update infrastructure** — provenance, dependency integrity, signature verification, rollback and least privilege. CI evidence applies only to the exact tested revision.
7. **Human operator / clinician** — receives unambiguous mode and task state, has an accessible stop/override path, and is never told that a synthetic receipt implies medical authorization.

## Threats and required responses

| Threat / fault | Consequence | Required control | Synthetic test or later evidence |
|---|---|---|---|
| Manifest adds an unknown autonomy or clinical-mode field | Hidden capability expansion | Reject unknown fields recursively; version the schema | Add unknown fields at top-level and every nested object |
| Tool label says “gripper” while identity resolves to a different end effector | Wrong motion/force plan | Keyed mechanical identity; bind plan to exact tool revision/digest | Swap tool ID/digest after validation; acceptance blocked |
| Calibration, localization or world state changes after planning | Collision, missed obstacle or misaligned handoff | Revision-bound plans expire on any dependency change; require replan and revalidation | Mutate each revision after validation; restored flag must not revive old plan |
| Payload is heavier/larger than declared | Drop, tip-over or actuator overload | Verify payload where feasible, conservative envelope, stop on uncertainty | In-range/out-of-range boundaries and sensor disagreement |
| Planner requests disallowed task or patient/invasive contact | Unauthorized contact or care action | Contact class and task/tool allowlist; deny unknown capability; no model self-authorization | Negative corpus for patient transfer, medication, invasive and unsupported tasks |
| Privacy capture remains enabled after consent expires | Unapproved capture or disclosure | Hardware shutter/default-disabled state; time-bounded consent; visible indicator; local enforcement | Revoke consent mid-episode, spoof UI flag, shutter-state disagreement |
| Network loss or compromised cloud identity | Remote control or missing safeguards | Critical stop and bounded safe state local; remote service cannot elevate authority | Drop network during task; verify bounded stop/handoff and no new command acceptance |
| Replay of a prior approval/receipt | Stale permission used in new context | Bind approval to episode, revision, expiry and nonce/monotonic sequence | Replay event with old tool/world/consent revision |
| Malicious plugin/dependency changes geometry or limits | Unsafe tool proposal | Allowlisted dependencies, reproducible build, source/digest attribution, schema/physical envelope checks | Hash change, dependency substitution and unrecognized-plugin fixtures |
| Firmware or model update changes behavior without requalification | Old evidence applied to new system | Signed update, provenance and rollback; invalidate affected qualification receipt | Update digest mutation makes old receipt inapplicable |
| Conflicting safety sensor readings are averaged away | False confidence in hazardous scene | Independent disagreement monitor; conservative stop/recovery rules | Inject occlusion, timestamp skew, stuck and conflicting sensors |
| Emergency stop is inaccessible or software-only | Delayed stop or single-point failure | Independent physical stop path, watchdog and verified stop behavior | Later bench/HIL: stop latency, distance, fault injection and access study |
| Safety supervisor fails or loses power | Uncontrolled or ambiguous actuator state | Hardware-defined safe state and detectable supervisor health | Must be specified and bench-tested per embodiment; manifest tests cannot establish it |
| Public logs leak patient data or become unauditable | Privacy breach or misleading evidence | Data minimization, role controls, retention policy, pseudonymous task IDs, digest-bound receipts | Privacy review, data scans, log tampering/replay tests |

## Safety invariants for v0

- Unknown fields, unsupported tools, unverified identity, malformed traces or missing authorization fail closed.
- Physical actuation, clinical use and patient-contact authorization remain false in every v0 receipt.
- A tool-manifest pass means only “eligible to enter the software simulator.”
- Changes to tool identity/revision/digest, calibration, world/localization, consent or permission invalidate dependent plans and applicable receipts.
- A planner cannot change policy, limits, authorization source or evidence claim ceiling.
- No real patient, medication, invasive tool, autonomous lift or patient-transfer scenario is admitted into this software test suite.
- Stopping and emergency response must not depend on network availability or continued operation of the generative/active-inference model.
- “No event observed” in a limited synthetic test is never translated into zero risk or physical safety.

## Adversarial benchmark plan

### Phase A — contract fuzz/mutation (automatable now)

- Unknown-field injection at every nested schema level.
- Missing, null, wrong-type, extreme and NaN/infinite-like numeric inputs.
- Task/tool/contact mismatch, duplicate IDs and unsupported schema revisions.
- Boundary values for dimensions, mass, payload, power, force and torque.
- Consent revoked, tool revision changed or permission expired after a candidate plan is created.
- Replayed/out-of-order trace, duplicate event, unknown event and changed digest with unchanged display ID.
- Malformed inputs produce deterministic rejection and a receipt containing the exact tested revision and corpus digest.

### Phase B — simulation adversarial episodes

- Door/obstacle appears after perception; lighting, occlusion and reflective surfaces change.
- Localization/calibration drift, lost joint feedback, tool-latch unknown and payload mismatch.
- Human enters reachable workspace or signals stop; conflicting proxemic/perception signals.
- Network outage and cloud-control attempt during an active task.
- Battery/thermal reserve below return/stop threshold; wheel slip or base tilt; biped fall only if that morphology enters scope.
- Report per hazard/fault: detection, false eligibility, safe-stop disposition, recovery result and time to handoff.

### Phase C — physical bench/HIL (future gate)

Only after an independently reviewed risk plan, engineering review and equipment-specific stop design. Use instrumented non-clinical fixtures, physical guards, measured force/torque, repeatable payloads and fault injection. Measure stop latency/distance, peak transient loads, dropped-object frequency, tool-latch failure detection, recovery and sensor-disagreement response. No people in the robot's force path without a separately reviewed study plan.

## Quantitative acceptance targets

These targets are for software/simulation gates; they are not universal medical-robot safety thresholds.

- 100% of registered malformed/unknown-field/out-of-scope fixtures rejected.
- 0 false-eligible outcomes in the frozen adversarial manifest corpus.
- 100% detection of registered revision, authorization, consent, tool-identity and stale-plan mutations.
- 100% of registered authority escalations rejected, with every receipt retaining physical_actuation_authorized=false, patient_contact_authorized=false and clinical_use_authorized=false.
- Deterministic results and digest changes when source or corpus changes.
- No release claim advances beyond the evidence layer actually observed.

Hardware thresholds must be derived per hazard, actuator, tool, user population and intended use in the risk process; this document deliberately does not invent one “safe force” value for all medical tasks.

## Current external anchors

- [FDA — Cybersecurity in Medical Devices, February 2026 final guidance](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/cybersecurity-medical-devices-quality-management-system-considerations-and-content-premarket): device cybersecurity design, labeling and premarket-submission documentation.
- [SAHPRA — MD08-2025/2026: AI/ML-enabled medical devices](https://www.sahpra.org.za/wp-content/uploads/2025/08/MD08-20252026_-SAHPRA-Communication-to-Industry-AI-Medical-devices-pdf.pdf): South African regulatory communication; re-check current requirements before regulated activity.
- [ISO 14971:2019](https://www.iso.org/standard/72704.html): lifecycle risk management, confirmed current by ISO in 2025.
- [NIST Cybersecurity Framework 2.0](https://www.nist.gov/cyberframework): general cybersecurity risk governance reference, not a medical-device certification.

## Open decisions before physical prototype review

- Intended use, target user, clinical setting and jurisdiction.
- Exact embodiment, payload, force/speed/energy envelope and stop architecture.
- Hazard analysis and independent safety-supervision design.
- Tool attachment/identity scheme; sensors, calibration and fault behavior.
- Cleaning/disinfection process, material compatibility, ingress protection and infection-control review.
- Privacy consent mechanics, data retention, cybersecurity threat assessment and recovery.
- Applicable regulatory pathway, quality system, ethics approval and human-factors plan.
