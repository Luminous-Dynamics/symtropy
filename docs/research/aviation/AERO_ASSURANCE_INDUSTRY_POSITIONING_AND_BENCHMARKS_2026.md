# AERO Assurance: Industry Positioning and Benchmark Programme (2026-11-28)

**Status:** research and programme strategy; not a certification finding  
**Scope:** claim/evidence assurance, model credibility, tool assurance, evidence acquisition, simulation evidence and continuous change impact  
**Authority ceiling:** this document does not establish engineering truth, grant design approval, replace an applicant's certification basis, or confer certification credit.

## Executive position

AERO should not try to replace DO-178C/DO-330 processes, ARP4754/ARP4761 safety assessment, NASA model-credibility practice, requirements-management platforms, PLM/digital-thread systems, or simulation tools. Those systems have real capabilities and domain authority that AERO does not possess.

The opportunity is to become the **independent assurance and evidence-decision layer across them**:

1. Compile a precisely scoped engineering claim into immutable, typed obligations.
2. Bind each obligation to source-owned evidence, exact configuration/effectivity, model applicability, receiver-first-seen time and independent observation.
3. Preserve non-compensatory assurance gates and explicit unknown/abstain outcomes.
4. Recommend evidence-acquisition actions only where authority and calibration are independently established.
5. Compute the minimum requalification impact of a change, with proof paths for what was invalidated and what remains valid.
6. Permit confidential evidence to be checked through controlled owner-hosted or cryptographic verification without claiming that a digest or proof establishes physical truth.

This is a stronger competitive proposition than “AI that reads certification documents.” It is a path toward auditable, reproducible assurance decisions while retaining the engineer, DER/DOA, technical authority, applicant and regulator as the authorities.

## 1. What the industry already does well

This assessment is based on the public sources linked below. Public product documentation is not a complete capability audit; absence of a feature from the material reviewed is not evidence that a vendor cannot implement it.

| Industry reference | Publicly documented strength | AERO should interoperate with it by… |
|---|---|---|
| FAA AC 20-115D and DO-178C / DO-330 family | An FAA-accepted means for airborne-software assurance; AC 20-115D recognizes ED-12C/DO-178C, ED-215/DO-330 and related supplements. It is guidance, not a universal mandatory regulation. | Carrying exact software/tool evidence, intended-use scope, configuration and verification results into an independently replayable claim/evidence graph—not assigning a tool qualification level itself. |
| EASA MOC-5 SC-VTOL Issue 2 (5 Nov 2026) and EUROCAE ED-342 (Sep 2026) | Current VTOL simulation-as-means-of-compliance guidance. MOC-5 calls for aircraft-level, multi-system/inter-system representativeness, crew procedures/workload, failure-case representativeness, model limitations and relevant flight-test correlation. ED-342 structures simulation use for handling-qualities certification. | Making model applicability, failure-case coverage, system interactions, data-exchange timing, configuration and validation evidence explicit in the claim state. |
| NASA-STD-7009B and NASA-HDBK-7009B (3 Feb 2026) | Lifecycle-oriented model-and-simulation credibility practice, with acceptance criteria set by projects and delegated technical authority; the 2026 handbook gives implementation guidance. | Preserving model identity, solver, validation package, domain coverage, split integrity, uncertainty and use-specific credibility as first-class obligations. |
| Ansys medini analyze | Publicly documents FHA, FMEA/FMECA, FMEDA, HAZOP, FTA, component fault trees, RBD, dependent failure analysis and aerospace-oriented ARP4754A/ARP4761 model structures. | Ingesting its source reports and typed findings; never duplicating or silently replacing its domain-specific safety analysis. |
| Siemens Polarion ALM | Publicly documents requirements/test traceability, configuration and change management, electronic signatures, historical project state, test execution and change impact views. | Providing a cross-tool, source-bound assurance state with exact temporal semantics and independently replayable impact decisions. |
| Dassault Systèmes Certification by Analysis | Publicly positions simulation and physical testing as a hybrid compliance demonstration, with traceability, repeatability and model-based data integration. | Qualifying the evidence and limitations behind simulation credit; not replacing the simulation, PLM or certification workflow. |
| SLSA provenance v1.2 | Defines verifiable metadata about where, when and how software artifacts were built, including builder, inputs, dependencies and outputs. | Reusing provenance as one software-artifact input while distinguishing build provenance from software correctness, model validity or physical truth. |
| ISO/IEC 27565:2026 | Published guidance on privacy preservation using zero-knowledge proofs. | Using selective-disclosure techniques for bounded predicates over confidential source evidence, with explicit proof-system and source-truth limits. |

### Practical conclusion

Industry is not standing still. Model-based systems engineering, requirements traceability, safety analysis, simulation credibility, software/tool assurance, digital twins and provenance each have mature standards and commercial products. AERO's advantage must be demonstrated at the **cross-boundary decision**: can it resolve one precise claim against heterogeneous evidence, determine exactly what was known and applicable at a historical cutoff, expose why a gate passed or failed, and select the next permitted investigation step without upgrading its own authority?

Do not market AERO as a substitute for any listed system. Market it as a vendor-neutral, authority-bounded evidence fabric that makes their outputs composable, temporally correct and independently replayable.

## 2. Paradigm shifts worth pursuing

### 2.1 Claims become compiled contracts

Treat a claim as an executable, versioned object—not a paragraph. The compiler should derive mandatory evidence classes, population strata, model-applicability obligations, uncertainty treatment, assumptions, exclusions, verification methods and the authority role expected to make the decision.

The submitter may propose a claim but cannot decide which evidence obligations are optional. An unknown requirement profile is unsupported, not silently interpreted.

### 2.2 Evidence becomes an event-sourced temporal object

Do not store only “evidence date.” Preserve at least:
- physical or subject-time interval;
- producer-record time;
- receiver-first-seen time;
- policy/effectivity interval;
- exact configuration and source digest.

This allows the same source to be valid for a later decision but unavailable to a historical decision. Late evidence must never backfill what was known at the cutoff.

### 2.3 Assurance becomes a typed state machine, not a score

Do not average trust, calibration, completeness, uncertainty and authority into one confidence number. Hard gates remain non-compensatory. Unknown, unsupported, conflicting, stale, late and out-of-domain remain distinct dispositions. A positive EVSI cannot compensate for a failed authority or evidence-integrity gate.

### 2.4 Assurance becomes active

Move from “is the current evidence pack sufficient?” to “which permitted evidence action most improves the decision?” Use EVSI, expected cost and delay; compare point estimates with approved ambiguity sets; disclose the assumptions and abstain whenever evidence or calibration is incomplete.

### 2.5 Qualification becomes incremental

Every source, model, configuration, requirement and owner-policy change should generate an impact closure over a typed dependency graph. Requalify the affected closure, not blindly everything and not merely the files that changed. Every retained item needs an unchanged-dependency argument; every invalidated item needs a reason path. Missing graph edges mean incomplete impact, never no impact.

### 2.6 Confidential evidence becomes verifiable without becoming public

Compare full disclosure, authorized replay, owner-hosted execution, TEE attestation and zero-knowledge verification. A cryptographic proof may establish that a specified program was run over a committed witness and that a bounded predicate holds under the scheme's assumptions. It does **not** establish that the source measurement is true, complete, competently produced, representative or physically accurate.

### 2.7 AI becomes a proposal generator, not an authority

LLMs may extract candidate requirements, identify missing evidence, suggest failure modes or propose acquisition actions. The final state must be reconstructed from typed source commitments and deterministic evaluators. Benchmark LLM-only and human+LLM approaches, but keep model-generated assertions outside the trusted boundary until independently checked.

## 3. Proposed competitive architecture

Keep the components separated:

- **Claim compiler:** closed, versioned requirement profiles; deterministic obligation manifest.
- **Source adapters:** raw payload integrity, typed parse, source identity, owner receipts and policy binding.
- **Evidence graph:** temporal/effectivity edges, model and observer scope, dependency provenance.
- **Assurance kernel:** non-compensatory per-obligation state and claim disposition.
- **Impact engine:** change-event invalidation and proof of retained dependencies.
- **Investigation planner:** point and robust EVSI constrained by permissions, calibrated outcomes, cost and schedule.
- **Privacy verifier:** externally verified proof/attestation receipt with exact program, source commitment, public statement, nonce, chronology and revocation scope.
- **Independent benchmark harness:** frozen corpus, independent oracle, negative controls, mutation testing and deterministic replay.

The kernel should remain Rust-first and deterministic. Python may be used as a CI-only independent oracle, never as the production assurance path.

## 4. Benchmark plan: compare against real baselines

Run a paired, blinded evaluation on the same frozen cases. Separate independent engineering/root-cause episodes from mutations or variants derived from one defect.

### Baseline arms

1. Existing document/process baseline assembled by an experienced assurance engineer.
2. Existing requirements/ALM plus safety-analysis toolchain (for example, Polarion plus medini or the customer's equivalent).
3. Static traceability and rule-based completeness checks.
4. Cheapest permitted evidence action.
5. Maximum expected information gain / entropy reduction.
6. Point EVSI.
7. Finite ambiguity-set lower-envelope EVSI.
8. Minimax regret and, only where assumptions justify them, CVaR / distributionally robust candidates.
9. Expert panel.
10. LLM-only and human+LLM workflow.
11. Full AERO pipeline.
12. Oracle with full latent episode information (upper bound only, never a deployable competitor).

### Hard safety/assurance gates

- Zero critical false admissions and zero false “qualified” dispositions in the bounded challenge corpus.
- Zero accepted evidence with wrong owner, subject, configuration, effectivity or historical receiver-first-seen time.
- Zero treating digest-only or owner-hosted receipt as full independent replay.
- Zero seeded critical tool malfunctions escaping the validated tool challenge corpus.
- Zero unauthorized physical-work or engineering-acceptance recommendations.
- 100% replay agreement for identical admitted inputs.
- 100% preservation of the frozen assurance state across investigation planning.
- Any missing source, dependency, authority, applicability or observer evidence must yield an explicit unknown/incomplete/abstain path.

### Performance measures after hard gates

- Claim-to-obligation completeness and precision.
- False-negative and false-positive obligation classification.
- Wrong-configuration, wrong-effectivity and late-knowledge error rates.
- Model-domain coverage, uncertainty calibration and worst-stratum performance.
- Evidence admission yield and source-owner response success.
- Independent replay agreement and provenance completeness.
- Decision regret versus the oracle; value per unit acquisition cost and delay.
- Time-to-decision, review effort, rework and unnecessary requalification.
- Privacy leakage, proof size, proof generation time and independent verification cost.
- Abstention quality, expert override rate and frequency of unsupported recommendations.

### Sample-size discipline

With zero observed failures, the approximate one-sided 95% upper confidence bound is about 3/n. Thus roughly 299 independent episodes are needed to put the bound below 1%, 2,995 below 0.1%, and 29,956 below 0.01%. Use exact binomial bounds in the benchmark report. These counts refer to independent root-cause episodes—not 299 generated mutations of one source defect.

### Initial improvement targets (aspirational, not achieved)

- Zero false negatives on hard-gate impact propagation in the preregistered holdout.
- 100% deterministic independent replay on fixed inputs.
- At most 5% unnecessary requalification and at least 50% less review effort than the matched baseline, only after the safety gates pass.
- At least 20% lower decision regret than the strongest non-AERO acquisition policy, without higher unauthorized-action or false-admission rates.
- No material degradation in the worst population stratum to achieve aggregate efficiency.

These are targets for an experiment, not current results or guarantees.

## 5. Immediate programme sequence

| Priority | Work | Exit evidence |
|---|---|---|
| P0 | Repair and validate the robust-EVSI integration; obtain exact-head Rust, formatting, workflow and Clippy results. | Exact PR-head receipts; no queued/skipped run called a pass. |
| P1 | Finish the source adapter: bind typed claim specification, derive obligation profile, consume all source-owner/model/observer/effectivity reports and map every obligation without bypassing the existing ClaimGate. | Parity test against existing claim gate across the full frozen corpus; adversarial omissions and substitutions rejected. |
| P2 | Build the assurance-delta graph and impact evaluator. | Zero false-negative affected hard-gate obligations in the adjudicated holdout; retained evidence has proof paths. |
| P3 | Add action-permission completeness, cost/delay and delivery uncertainty to a new robust-EVSI version. | Scenario-set commitment binds every decision-affecting input; frozen v0.1 retained. |
| P4 | Prototype one bounded privacy predicate and compare owner-hosted, TEE and ZK routes. | Independent verifier tests plus explicit statement of what is and is not proven. |
| P5 | Benchmark one real material/process claim end-to-end with independent engineering adjudication. | Publicly reproducible protocol, sanitized evidence, exact metrics and limitations. |

## 6. Research references (reviewed 2026-11-28)

1. FAA, [AC 20-115D — Airborne Software Development Assurance Using ED-12/DO-178](https://www.faa.gov/airports/resources/advisory_circulars/index.cfm/go/document.information/documentNumber/20-115D). FAA lists it as active and describes it as an acceptable, non-mandatory means of compliance.
2. EASA, [MOC-5 SC-VTOL Issue 2](https://www.easa.europa.eu/en/downloads/144423/en), 5 November 2026. See its aircraft-level simulation representativeness, inter-system dynamics, crew/workload and model limitation guidance.
3. EUROCAE, [ED-342 — Guidance on Using Simulation in Handling Qualities Certification of VTOL](https://www.eurocae.net/product/ed-342-guidance-on-using-simulation-in-handling-qualities-certification-of-vtol/), September 2026.
4. NASA, [NASA-STD-7009B](https://standards.nasa.gov/node/263), active standard for model and simulation credibility; [NASA-HDBK-7009B](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009), implementation handbook dated 3 February 2026.
5. Ansys, [medini analyze safety analysis](https://ansyshelp.ansys.com/public/Views/Secured/medini/v242/en/medini_analyze_ug/chap_safety_analysis.html) and [ARP4754A/ARP4761 aerospace profile](https://ansyshelp.ansys.com/public/Views/Secured/medini/v252/en/medini_analyze_ug/sec_arp4754A.html).
6. Siemens, [Polarion ALM](https://www.siemens.com/en-us/products/polarion/application-lifecycle-management-alm/), requirements, test, change and configuration traceability.
7. Dassault Systèmes, [Certification by Analysis](https://www.3ds.com/industries/aerospace-defense/certification-analysis), simulation and physical-test integration.
8. SLSA, [Build Provenance v1.2](https://slsa.dev/spec/v1.2/build-provenance) and [Tracks](https://slsa.dev/spec/v1.2/tracks), build and source provenance.
9. ISO/IEC, [ISO/IEC 27565:2026](https://www.iso.org/standard/80398.html), privacy preservation based on zero-knowledge proofs, published February 2026.

---

**Bottom line:** AERO should try to beat industry baselines on *independently verifiable decision quality per unit of time, cost and disclosure*, not on document extraction, aggregate confidence scores or benchmark theatrics. The differentiator is a precise claim, a source-bound temporal evidence state, an authority-bounded investigation plan, and a defensible explanation of exactly what changed and why.
