# Recovery Admission Benchmark Protocol v0.1

**Status:** preregistration proposal; not an executed benchmark and not evidence of operational safety.  
**Scope:** synthetic civilian infrastructure repair/recovery tasks, task dependencies, multi-resource assignment, evidence visibility, utility constraints, inspection and safe-stop transitions.  
**Out of scope:** autonomous dispatch, physical control, claims about real-world reliability, cryptographic qualification, or replacing a competent site safety authority.

## 1. Research thesis: optimize only inside the admissible region

Traditional orchestration metrics can reward faster work despite untrustworthy or future information. Separate three capabilities: (1) epistemic correctness—decisions use only observations visible at the decision tick and unknown is not treated as safe; (2) admissibility—resource, capacity, authorization, predecessor, current evidence and inspection constraints pass; and (3) recovery quality—among admissible plans, compare how efficiently accepted work or service is restored. This is a design hypothesis, not an established result.

## 2. Paired benchmark arms

| Arm | Behavior | Purpose |
|---|---|---|
| B0 — fixed schedule | Frozen fixed schedule, as named in the canonical fixture | Baseline outcome and failure rate; simulation only, never dispatch |
| B1 — rule-based recovery | The fixture's rule-based recovery policy, with the same observation-visibility rules | Measure recovery benefit over fixed schedule |
| B2 — evidence-bounded replanning/admission | Typed task state, predecessor receipts, resource/capacity, utility, approval, evidence freshness and inspection gates | Measure unsafe rejection and false blocking |
| B3 — finite-instance exhaustive oracle | Exhaustive reference over a declared small state space; **not deployable** | Bound-check candidate decisions and detect oracle disagreement |
| B2V — verified-receipt overlay (future) | B2 plus schema-bounded, signed, revision-bound, replay-safe envelopes and append-only correction/revocation history | Measure verification security benefit and incremental cost |

Every arm gets the same scenario IDs, paired seeds, dependency graph and exogenous disturbances. Freeze commits and configuration before comparison. B0 is simulation-only, never a dispatch mode.

## 3. Required scenario strata

- **Recoverable:** isolated equipment failure, delayed observation, substitution, interruption, repair, new as-built evidence and successful reinspection.
- **Hard blocks:** missing/failed inspection, declined or unknown approval, insufficient capacity, incompatible capability, stale calibration, poor localization, lost communications, infeasible energy reserve, unsafe or unknown utility.
- **Temporal adversaries:** future observation reference, hidden observation, duplicate/irrelevant observation, reordered delivery, delayed fault reporting, contradiction and corrected/revoked claim.
- **Dependency/concurrency:** accepted-predecessor requirement, cyclic dependencies, shared-resource contention, crews competing for capacity, invalid reassignment and cascading power/water/communications dependencies.
- **Evidence adversaries:** missing, stale, wrong-task, wrong-resource-revision, wrong-kind, duplicate, tampered, expired, revoked, replayed or causally unbound receipts. B2V-only cases remain blocked until B2V exists.
- **Boundary/counterfactual:** exact capacity boundary, visible tick equal to decision tick, work done without acceptance receipt, successful inspection attached to stale artifact, protective stop before fault visibility.

Keep ground truth, injected event time, policy-observation availability, policy-visible projection and evidence-creation time separate. Never expose evaluator-only truth to the policy.

## 4. Non-compensatory qualification targets

| Metric | Target | Measurement |
|---|---:|---|
| Unsafe transition acceptance | **0** on frozen negative corpus | Attempts and denominator by mutation class |
| Future/hidden observation use | **0** | Instrument visible reads and compare against observable tick |
| Acceptance lacking accepted predecessor, current as-built evidence or successful inspection | **0** | Independently inspect the emitted acceptance record |
| Unknown/declined/revoked approval or utility treated as safe | **0** | Unknown and missing are distinct negative states |
| Tiny-model oracle agreement | **100%** | Exhaustively enumerate a declared bounded state/action/gate space against a separately written oracle |
| Identical-input replay | **100% byte-identical normalized decisions** | Pinned build; compare ordered decisions, reason codes and digests |
| Safe-recovery false-block rate | Measure; provisional target **<=5%** | Only count scenarios independently labelled admissible by frozen oracle |
| Paired rollout count before aggregate utility claim | **>=1,000 scenario/seed pairs** | Same exogenous disturbances across arms; publish seeds, failures and exclusions |
| Recovery utility | Provisional goal **>=10% lower paired median time-to-accepted-work vs B0** | Claim improvement only if preregistered uncertainty interval supports it and all hard gates pass; otherwise inconclusive |
| Cost | Report p50/p95/p99 decision latency, max and peak memory | Pin runner, dataset size, toolchain, build mode and concurrency; set SLO after baseline |
| Denominator accounting | **100%** | Include accepted work, blocked attempts, aborts, stops, rework, rejected inspections, missing evidence, timeouts and exclusions |

False-block and utility thresholds are goals, not demonstrated performance. Quarantine cases where the independent oracle disagrees with labels; never relabel simply to make a candidate pass.

## 5. Independent oracle and property tests

Implement the bounded-state oracle independently of production `assess`; exhaustively enumerate small state machines, then use deterministic generated mutations for larger fixtures. Required properties:

- **Fail-closed:** switching a required gate from admissible to unknown/unsafe cannot turn a blocked decision into admission.
- **No temporal privilege:** adding a future/hidden observation cannot improve admissibility.
- **Evidence discipline:** adding relevant, current, verified evidence may resolve a deficit; irrelevant, stale, revoked or unauthenticated evidence cannot.
- **Resource consistency:** turning a resource unavailable/incompatible cannot improve admissibility.
- **Stop dominance:** protective stop does not need planner visibility of the fault, and future/unknown observation references or a bounded-horizon overrun must not veto the stop.
- **Replay stability:** same canonical inputs yield byte-equivalent normalized decisions; unordered fields must canonicalize or be rejected.
- **Correction safety:** correction/revocation appends history and invalidates the affected claim without rewriting prior records.
- **No hidden-state leakage:** decisions depend on public decision inputs, not injection timestamps or evaluator-only truth.

Target >=10,000 deterministic generated mutations before treating property tests as a meaningful stress gate. Preserve failing seeds and minimized counterexamples. This target is not a claim about current test coverage.

## 6. Report/provenance contract

Capture repository, full source commit and tree SHA; fixture, scenario-set, config, toolchain, lockfile and build-profile digests; arm, run ID/attempt, seeds, runner description, wall-clock and CPU time; numerators/denominators for each gate; paired time-to-accepted-work, lost work, stops/aborts, reassignments, rework, inspection rejection and unresolved service constraints; per-scenario input/decision digests and full report digest; and an explicit claim ceiling (synthetic policy check, not physical validation, dispatch authorization, evidence authenticity, or proof a real-world assertion is true).

Store run receipts as downloadable CI artifacts and later sign them independently. Hashes aid reproducibility/tamper detection but are neither signatures nor identity proof.

## 7. Existing evaluation practice to adapt

Build repeatable elemental checks and composed mission scenarios rather than one aggregate score. NIST's [Performance of Emergency Response Robots](https://www.nist.gov/programs-projects/performance-emergency-response-robots) program uses task-oriented measurements for capabilities such as mobility, sensing, dexterity, endurance, communication, autonomy, logistics and safety; adapt the method of publishing tasks/measurements, not its physical-robot standards wholesale. The [W3C PROV Model Primer](https://www.w3.org/TR/prov-primer/) offers a vocabulary for entities, activities and responsible agents. Neither reference validates this proposed benchmark; domain expert review remains required.

## 8. Promotion gates

1. **Contract only:** exact-head format, manifest and adversarial tests plus independent tiny-model oracle pass.
2. **Synthetic comparison:** all hard gates pass; >=1,000 paired seeded scenarios; denominators and uncertainty intervals are public.
3. **Evidence verification:** B2V passes canonical interoperability vectors and all frozen negative mutations; signature validity remains distinct from physical truth.
4. **Shadow deployment:** read-only capture in sandbox/supervised testing, no control path, compare decisions to domain experts.
5. **Operational use:** out of scope until independent engineering/safety and cybersecurity review, site-specific hazard analysis, operator procedures and controlled validation.

Do not skip a gate because a workflow is queued or unavailable. Queued is not pass.
