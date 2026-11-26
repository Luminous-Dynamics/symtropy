# AERO → Symthaea Evidence-Gap Adapter v0.1

**Related:** Symtropy AERO temporal assurance #7455 / PR #7456; observer coverage PR #7508; Symthaea ORBITAL-JOINT-EVIDENCE-VOI-001 #13111 / PR #13112; generic decision-impact benchmark #7803; matched-budget sequential tournament #13125.  
**Status:** adapter contract and synthetic schema fixture.  
**Claim ceiling:** evidence-gap exchange only; no experiment selection, execution, physical qualification or release authority.  
**Ownership:** AERO/Symtropy emits engineering-specific assessment gaps; Symthaea consumes them through its existing evidence-action selector; Mycelix/BASALT remain authoritative for their own receipts and dependency/coverage claims.

## 1. Purpose

AERO now has two complementary research mechanisms: temporal replay over acquisition, evidence-receipt and trust knowledge; and observer-coverage/source-partition causal-cut checks.

Those mechanisms can identify why an engineering claim is blocked or unresolved. They do not decide which experiment should be purchased or performed next.

That decision already has an owner: Symthaea's aerospace Evidence VOI adapter (#13112) and the shared decision-impact benchmark (#7803). Its selector enforces exact profile/article/configuration/ICD context, non-compensatory requirements, action admissibility and component-wise resource budgets. It supports calibrated EVSI only when a probability model and calibration evidence are supplied; otherwise it uses distribution-free decision semantics. It separates proposal from execution outcome and does not authorize physical actions.

**Do not implement a second AERO selector, EVSI calculation, action optimizer, resource-cost scalarizer, experiment simulator or evidence invalidation graph.**

This adapter gives Symthaea the exact AERO assessment facts it needs to reason about admissible next evidence actions.

## 2. Evidence-gap packet

The AERO producer emits one immutable AeroEvidenceGapPacketV1 after a specific replay/coverage assessment.

It binds:
- exact AERO assessment reference and report digest;
- exact replay-coordinate digest;
- profile/article/configuration/ICD decision context;
- claim-by-claim dispositions and reason codes;
- owner-authored evidence requirements and their verifier/output contracts;
- source receipts/checkpoints/coverage references already admitted to the assessment;
- optional owner-native change-impact receipt and its completeness state;
- an explicit evidence-gap-only claim ceiling and no-actuation boundary.

Source evidence references carry an explicit AERO replay-view state: `ADMITTED`, `UNVERIFIED`, `REJECTED` or `NOT_RECEIVED`. Only `ADMITTED` references are projected into Symthaea's current-evidence set. Other states remain visible as separate unresolved/rejected/not-received references; they must not be silently represented as available evidence. AERO admission state is not a cryptographic or physical truth claim.

The packet deliberately does not contain:
- recommended action rankings;
- EVSI, expected information gain, entropy or decision regret;
- probability values inferred from intervals;
- estimated action costs;
- permission to retrieve sensitive evidence or launch tests;
- any qualified, released, airworthy or physical-acceptance promotion.

AERO reports what is unresolved. The existing Symthaea owner decides which admissible action is worth proposing.

## 3. Status semantics

Claim dispositions are copied from the qualified AERO assessment, not inferred by the adapter:

| AERO claim disposition | Selector interpretation |
|---|---|
| ACCEPTED_FOR_BINDING_ONLY | Temporal/evidence binding only; not physical qualification |
| BLOCK | Claim is blocked; only a separately admissible evidence action may address the stated blocker |
| ABSTAIN | Evidence is incomplete or ambiguous; do not infer a favorable state |
| TRUST_UNRESOLVED | Authority, observer qualification or trust evidence unresolved |
| INCOMPLETE | Known evidence/source/capture obligation missing |
| OUT_OF_SCOPE | Requested claim is outside the qualified scope |
| REJECT | Integrity or contract invalidity; do not consume as trusted evidence |

The selector may propose a reconciliation or remeasurement action to resolve a gap. It cannot treat that proposal as though the current claim were already supported.

AERO's ACCEPTED_FOR_BINDING_ONLY remains a deliberately limited result. It must never be translated into QUALIFIED, NDE acceptance, material conformity or airworthiness.

## 4. Exact mapping to Symthaea #13112

The adapter uses the existing Symthaea contract; it does not redefine it.

| AERO packet field | Existing Symthaea field / owner |
|---|---|
| decision_context.profile_ref | DecisionContext.profile_ref |
| decision_context.article_subject_ref | DecisionContext.article_subject_ref |
| decision_context.configuration_id | DecisionContext.configuration_id |
| decision_context.icd_revision | DecisionContext.icd_revision |
| unresolved claim_id | Scenario.target_claim_id / action target_claim_id |
| AERO-admitted source evidence references | Scenario.current_evidence_refs (only the packet's ADMITTED references; other states are preserved separately) |
| owner-authored evidence requirements | Scenario.required_obligations and action expected outputs |
| owner-native evidence dependency groups | Scenario.current_evidence_dependency_groups (exact groups over admitted references; it must not infer common-cause relationships) |
| Mycelix #6219 change-impact receipt | Scenario.change_impact / EvidenceActionProposalV1.change_impact |
| exact method, procedure and verifier | action method_ref, admissibility evidence and ExpectedOutput.verifier_ref |
| owner-supplied component costs | Resources / resource_plan and remaining budget |

The projection also preserves `unverified_evidence_refs`, `rejected_evidence_refs` and `not_received_evidence_refs` separately. Dependency groups may only reference AERO-admitted source evidence; the owning system defines those groups, and the adapter checks referential consistency rather than inferring statistical independence.

The packet does not make a candidate action admissible. Symtropy method/facility owners must provide the action's actual applicability, prerequisites, calibration, safety approval, expected output, verifier and resource vector. Symthaea's existing selector performs its own exact-context/admissibility/budget checks.

The action context must exactly equal the AERO decision context. A profile, serialized physical subject, configuration or ICD mismatch makes the action ineligible, regardless of its claimed information value.

## 5. Ownership boundaries

- **Symtropy AERO:** physical/effective-time semantics, acquisition uncertainty, calibration binding, engineering claim scope and AERO-specific evidence-gap dispositions.
- **Symthaea #13112:** evidence-action proposals, decision-value calculation, candidate eligibility, resource Pareto comparison and sequential policy selection.
- **Symthaea #7803 / #13125:** generic experiment-policy benchmark semantics and the matched-budget sequential tournament.
- **Mycelix #6219:** event-frontier completeness, change-impact dependency closure, historical preservation and targeted requalification authority.
- **Mycelix/AeroCommons:** serialized article/build/configuration/artifact identity and owner-native receipt/checkpoint records.
- **Symthaea BASALT/ASSURE:** generic observer coverage, evidence freshness and qualification lifecycle.
- **Symtropy #4165 / #6540 / #6484:** NDE/inspectability semantics, mandatory campaign adequacy and interface lifecycle requirements.
- **Qualified labs and engineering authorities:** physical test execution, ground truth, procedure approval and acceptance.

AERO must not reconstruct Mycelix dependency edges or decide that a claim was unaffected by an owner event. An incomplete owner event frontier remains incomplete and blocks promotion.

## 6. AERO-specific matched-budget tournament extension

Use the existing #13125 tournament and #7803 scorecard. Add AERO evidence-gap cases rather than forking its simulator or reimplementing its policy baselines.

Suggested held-out scenarios:

1. **Late retroactive revocation:** only full evidence+trust reassessment includes the late source record; historical and trust-only views remain unchanged. The selector must not use a trust-only view to claim that missing event bytes were received.
2. **Unqualified observer:** a digest-valid checkpoint exists but observer coverage is unverified. A source/coverage reconciliation may be admissible; a negative inference from absence is not.
3. **Causal predecessor missing:** a downstream event is included but the upstream dependency is absent. Reconciliation is required before using the cut.
4. **Stream epoch mismatch:** source sequence numbers were reset; a new epoch transition receipt is required. Sorting by wall clock is not a substitute.
5. **Acquisition/effectivity overlap:** calibration revocation effective time intersects the acquisition uncertainty interval. A new evaluation cannot simply choose a favorable endpoint.
6. **Wrong timestamp imprint:** a token's declared status conflicts with its digest. Reject the token; do not count it as a usable timestamp.
7. **Common-cause corroboration:** two apparently separate checks rely on one instrument/reference or capture path; they do not provide independent corroboration.
8. **No-information control:** every admissible action leaves the hard claim unresolved or cannot change the decision. Correct output is abstention, not invented EVSI.
9. **Mandatory high-fidelity follow-up:** a cheap analytical or simulation action cannot discharge a physical/NDE obligation; deferred cost remains in the full budget.
10. **Out-of-scope physical model:** no admissible action exists within the declared physics/procedure scope; preserve the unresolved result and escalate through the separately authorized owner workflow.

### 6.1 SOTA baselines for the matched-budget tournament

The AERO extension should use the existing Symthaea #13125 tournament harness and add comparator policies rather than create a second benchmark runner.

| Policy arm | Why it belongs in the comparison |
|---|---|
| Seeded random / space filling | Establishes whether adaptive selection adds value over design-of-experiments coverage |
| Prediction disagreement | Existing Symthaea generic selector baseline; informative about model separation but not automatically decision impact |
| EI / NEI / knowledge gradient | Conventional Bayesian optimization / experimental-design baselines |
| Cost-aware multi-fidelity consequential improvement | 2026 engineering research explicitly accounts for low-fidelity cost, accuracy/validity difference and hidden high-fidelity validation cost |
| R-IDeA-style acquisition | 2026 active-learning research combines representativeness, informativeness and error de-amplification under model misspecification |
| Distribution-free robust/minimax regret | Required when only intervals or uncertainty sets are defensible; no fabricated probability model |
| Symthaea #13112 selector | Existing AERO-specific admissibility, hard-claim and component-wise resource semantics |
| Qualified human engineering/NDE panel | Operational reference for domain expertise, mandatory work and justified abstention |

The experimental-design comparison must distinguish three regimes:
1. **Calibrated-probability EVSI:** use only an explicitly supplied and calibrated state model and observation likelihoods.
2. **Distribution-free robust decision value:** use a declared uncertainty set and worst-case regret/feasibility semantics; do not assign probabilities to intervals.
3. **Multi-fidelity validation value:** charge both the low-fidelity action and the mandatory high-fidelity validation needed before a physical claim can be promoted.

Do not let a low-cost analytical model receive credit for resolving an NDE, material or structural acceptance requirement that it cannot discharge. The cost of the required independent validation is part of the acquisition decision from the beginning.

External research comparators:
- A 2025 production-engineering Bayesian experimental-design study evaluated 15,360 runs across eight engineering-focused test functions, noisy/noiseless variants and 96 algorithm configurations; it reports material differences between configurations and no universal default: https://www.frontiersin.org/journals/manufacturing-technology/articles/10.3389/fmtec.2025.1614335/full
- The 2026 consequential-improvement multi-fidelity acquisition work explicitly includes cost, accuracy and validity differences between low- and high-fidelity samples, including the hidden high-fidelity validation cost: https://link.springer.com/article/10.1007/s11081-026-10078-5
- R-IDeA research (AISTATS 2026) adds representativeness and error de-amplification to informativeness to reduce generalization error under model misspecification: https://proceedings.mlr.press/v300/tang26d.html

These are comparators, not results reproduced by AERO or Symthaea.

### Scorecard

Report separately:
- exact AERO replay/coverage oracle agreement;
- false-complete and false-supported claim rate;
- false-feasible action count;
- action admissibility and context mismatch detection;
- missing predecessor / epoch / observer-gap detection;
- correct abstention and human-review escalation;
- cost to resolve a named engineering claim;
- cost to an independently verified final decision;
- engineering, compute, facility, specimen, calibration and reviewer resources;
- mandatory deferred physical/NDE cost;
- actual failed/aborted/censored actions and consumed resources;
- residual blocked claims and evidence dependencies.

Do not create a composite score that trades away a hard assurance failure for cheaper testing.

A prospective target of 20% lower cost to an independently supported decision may be studied, but only at matched component-wise budgets and zero regression on critical false-support/false-feasible controls. It is not an achieved result or universal performance claim.

## 7. Reassessment loop

1. AERO emits an immutable evidence-gap packet for replay coordinate R0.
2. Symthaea consumes the packet and the owner-authorized candidate action catalogue.
3. Symthaea emits a proposal-only action record. It does not execute the action.
4. The authorized external owner executes or declines the action and records actual outcomes/resources in a separate outcome record.
5. Source-owner receipts, raw artifacts and observer coverage are verified by their owning systems.
6. AERO performs a new replay with a new evidence/trust coordinate and emits a new gap packet.
7. The previous report and packet remain unchanged and link to the successor assessment.

The action's predicted result is not evidence. The proposal cannot pre-fill a future observation, alter a past receipt watermark or update the trust snapshot. Failed/aborted work remains in cost accounting and creates no favorable evidence.

## 8. Promotion gates

- Schema and fixture validation on the exact branch head.
- AERO packet is generated from an immutable replay report, not manually assembled from claims.
- Exact subject/profile/configuration/ICD equality across AERO and Symthaea contexts.
- All AERO reason codes and claim dispositions preserved losslessly.
- Required evidence outputs and verifiers originate from the competent method owners.
- Change-impact receipt and affected-claim set are verified by Mycelix #6219; incomplete event frontiers force abstention.
- Symthaea's existing #13112 tests and independent oracle remain the selector's verification owner.
- AERO-specific scenarios enter #13125 as held-out variants; baselines and resource budgets are frozen before scoring.
- No probability is generated from a hard uncertainty interval.
- No action proposal is interpreted as permission to execute.
- New evidence yields a new AERO replay; no historical report is overwritten.

## 9. Claim boundary

The adapter does not prove AERO's physical evidence is correct, authenticate source receipts, qualify an observer, select an optimal experiment, validate a probability model, execute an inspection, or certify an article. It defines a narrow, testable exchange between AERO's engineering-specific evidence gaps and Symthaea's already-owned evidence-action selection infrastructure.

## References

- Symthaea #13112, Evidence VOI proposal/outcome adapter: https://github.com/Luminous-Dynamics/symthaea/pull/13112
- Symthaea #7803, decision-impact benchmark contract: https://github.com/Luminous-Dynamics/symthaea/issues/7803
- Symthaea #13125, matched-budget sequential tournament: https://github.com/Luminous-Dynamics/symthaea/issues/13125
- Mycelix #6219, event-driven evidence invalidation: https://github.com/Luminous-Dynamics/mycelix/issues/6219
- Symtropy #4165, design for inspection and SHM: https://github.com/Luminous-Dynamics/symtropy/issues/4165
- Symtropy #6540, qualification test-matrix adequacy: https://github.com/Luminous-Dynamics/symtropy/issues/6540
- Symtropy #6484, interface lifecycle qualification: https://github.com/Luminous-Dynamics/symtropy/issues/6484
