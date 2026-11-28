# AERO eVTOL Simulation Assurance Transfer Study v0.1

**Status:** prospective study design; no flight/simulator data, oracle adjudication or performance results  
**Authority ceiling:** research-only evidence-chain reconstruction; no means-of-compliance approval, airworthiness or certification  
**Prerequisite:** do not begin a comparative performance claim until the structural-material AERO IAB benchmark has an independently adjudicated oracle and an exact preregistered protocol.

## 1. Why this transfer domain

EASA's fifth publication of the Means of Compliance with the Special Condition VTOL, MOC-5 SC-VTOL Issue 2, dated 5 November 2026, includes guidance on certification credit for simulation benches and test rigs. It expects representativeness and limitations to be established relative to test objectives, calls for configuration management of the bench and models, and requires changes and problem reports to be assessed for their effects on previously performed tests. It discusses system, multi-system and aircraft-level views, failure cases and validation evidence.

EUROCAE ED-342, published September 2026, provides a structured approach to using simulation as a means of compliance for VTOL handling qualities, including pilot-in-the-loop simulation in cases where flight test may be partially or fully substituted for selected scenarios.

These are demanding, real engineering evidence chains. They are not evidence that AERO itself can establish simulator adequacy. The intended experiment is narrower: can AERO reconstruct the claim context and determine whether the required owner-authored evidence has been supplied, is correctly bound, was known at the chosen time, remains applicable to the current configuration and can be independently replayed?

References:
- [EASA MOC-5 SC-VTOL Issue 2, 5 November 2026](https://www.easa.europa.eu/en/downloads/144423/en)
- [EUROCAE ED-342, September 2026](https://www.eurocae.net/product/ed-342-guidance-on-using-simulation-in-handling-qualities-certification-of-vtol/)

## 2. Claim episode and owner boundaries

The study unit is a certification-demonstration claim episode, not a simulator run, scenario row or generated mutation.

Each episode binds:

1. **Aircraft and configuration:** aircraft model/type, serial or prototype identity, hardware/software configuration, propulsion/rotor arrangement, control-law revision, mass properties and relevant aircraft-level integration state.
2. **Requirement and intended use:** exact requirement identifier/revision, means-of-compliance reference/revision, test objective, requested credit and declared operating/flight envelope.
3. **Simulation bench:** simulator identity, host and I/O hardware, real-time target, software image, model package, parameter set, HMI, pilot interface, sensor/actuator models and environment configuration.
4. **Representativeness declaration:** owner-authored statement defining scope, comparison basis, known limitations, deviations, failure cases and the exact test objectives for which representativeness is claimed.
5. **Validation and correlation artefacts:** flight-test, rig, hardware-in-the-loop, system-integration, pilot-in-the-loop or other relevant results; dataset provenance; coverage of normal and failure conditions; model discrepancy and uncertainty records.
6. **Configuration/change history:** bench or aircraft changes, model updates, issue/problem reports, approved deviations, regression tests, post-test modifications and their assessed impact.
7. **Authority and chronology:** source owner, producer role, signed/receipted artifact identity, issuer event time, physical/acquisition time where applicable, receiver-first-seen time and policy snapshot.
8. **Independent replay package:** disclosed source bytes or explicitly typed disclosure class, dependency manifest, canonicalization/version data and a standalone verifier input.

AERO must not author the representativeness declaration, invent a validation result, infer flight-test equivalence, supply missing uncertainty budgets or grant certification credit. Those belong to the applicant, model/bench owners, engineering authorities and competent regulators.

## 3. Adversarial challenge families

The initial transfer corpus should contain at least the following independently adjudicated root-cause families. The numbers below are planning coverage targets, not achieved sample sizes.

| Family | Representative fault | Required AERO behavior |
|---|---|---|
| Aircraft identity | Correct model name, wrong prototype/serial | Reject or block the cross-subject evidence binding |
| Configuration drift | Control-law or rotor configuration changed after test | Preserve original snapshot and identify affected claims |
| Bench representativeness | Bench declared representative for a different test objective | Mark applicability unsupported; do not infer equivalence |
| Model identity | Report refers to different solver/model/parameter package | Detect exact artifact/configuration mismatch |
| Failure-case coverage | Required failure case omitted from owner coverage declaration | Incomplete; do not let nominal-envelope success compensate |
| Flight-test correlation | Correlation data from different aircraft/software configuration | Reject the cross-configuration binding or leave applicability unverified |
| Model limitation | Limitation discovered after historical demonstration | Current-policy/current-knowledge replay differs from historical-as-known; retain both |
| Bench problem report | Real-time delay, I/O defect or HMI issue affects test representativeness | Propagate owner-assessed impact to affected claims |
| Post-test modification | Design changed after the bench test | Require a qualified change-impact assessment before preservation |
| Validation-data lineage | Correlation dataset is missing source provenance or calibration | Mark evidence unverified/incomplete |
| Pilot / HMI configuration | Different pilot interface or cueing configuration | Treat as a distinct configuration; no silent substitution |
| Observer coverage | Issue tracker or configuration observer missed a change class | Unknown impact, never an empty impact set |
| Temporal replay | Evidence received after the review cutoff but issuer assertion is backdated | NOT_KNOWN_AT_SNAPSHOT for the historical query |
| Revocation / correction | Model report, key or bench artifact superseded or revoked | Preserve historic state and create a current-policy counterfactual |
| Coupled fault | Wrong simulator version plus missing failure case | Preserve both blockers and reason codes |
| Metamorphic replay | Manifest reorder, filesystem relocation or package transport | Same disposition and reason-code vector for the same frozen inputs |

Each generated mutation from one root cause remains in the same statistical cluster. The benchmark's independent n is the number of independent engineering episodes/root-cause families, not the number of generated variations.

## 4. Required baseline comparisons

Use a matched input corpus and completion criteria. Compare categories separately:

- Expert certification / systems / simulation assurance review.
- Existing PLM/QMS and configuration-management workflow.
- Structured assurance-case authoring and analysis tools.
- Owner-native simulation V&V / VVUQ and qualification workflow.
- Rules-only requirement/configuration/evidence validator.
- Graph-only change-impact workflow.
- Single-LLM evidence extraction and multi-agent investigation workflow.
- AERO without temporal replay, without owner authority binding, without observer qualification, and full AERO.

CM4QC and AM Bench are important evidence and measurement programme comparators for computational materials, not software-agent baselines. EASA's VIRTUA work is an adjacent lifecycle-certificate / approved-part provenance comparator; it explored distributed-ledger approaches to certificate authenticity and revocation. AERO should compare its owner-scoped evidence receipts and historical replay with such lifecycle systems without assuming that blockchain is necessary or sufficient.

No baseline receives reduced evidence, fewer configuration records or a less demanding time cutoff. Human labor, integration setup, escalation, correction, abstention and compute costs are included.

## 5. Metrics and hard gates

Reuse the AERO IAB metric definitions wherever the concepts match:

- critical configuration/context substitution acceptance: target 0;
- unauthorized-producer acceptance: target 0;
- failure-case / mandatory-stratum omission acceptance: target 0;
- late-evidence historical-knowledge error: target 0;
- critical invalidation miss rate: target 0;
- false preservation under unknown observer coverage: target 0;
- critical disposition agreement: target 100%;
- exact independent replay and reason-code agreement;
- false rejection and positive-control retention;
- unnecessary requalification, only after critical invalidation gates pass;
- matched reviewer cost per completed claim decision.

Additional domain-specific diagnostic metrics should include:
- required failure-case coverage completeness against the owner-declared requirement profile;
- simulator/aircraft configuration binding correctness;
- bench change-to-affected-claim recall;
- limitation-to-test-objective trace completeness;
- correlation-data provenance and calibration binding;
- post-test change impact assessment completeness.

These metrics evaluate the integrity of the evidence chain, not the physical quality of the simulation model. The model owner's validation methods and acceptance thresholds remain authoritative inputs.

## 6. Independent oracle and evidence handling

Two reviewers independently adjudicate each episode:
- one certification / systems assurance reviewer;
- one simulation / flight-sciences reviewer familiar with the test objective and declared context.

They label exact subject/configuration identity, evidence authority, applicability, historical knowledge, failure-case coverage, observer completeness, disposition vector, blockers, reason codes and affected-claim set. Disagreements go to a third adjudicator. Reviewers must not use the AERO output to construct the oracle.

Use synthetic fixtures first. Real aircraft, pilot, operational, security-sensitive and proprietary records require explicit rights, data minimization, access controls and owner authorization. Never scrape or publish restricted flight-test data, proprietary model packages or security-sensitive system details.

The benchmark custodian freezes blind-test episode digests and root-cause partitions before evaluation. All sibling mutations remain together. Report independent episodes, root-cause families, variants, stratum counts, exact confidence intervals and the worst critical stratum.

## 7. Release sequence

- **E0 — schema/contract:** validate claim, configuration, evidence and replay schemas.
- **E1 — synthetic semantics:** independently adjudicate synthetic scenarios and run metamorphic/property tests.
- **E2 — clean-room replay:** verify the capsule in an independent environment with no undeclared dependency.
- **E3 — matched workflow study:** blinded human reviewers, pre-registered time/error/cost endpoints.
- **E4 — authorized non-operational data pilot:** only owner-approved simulator/bench records; no certification credit claim.
- **E5 — authority-engaged assessment:** only after a competent applicant/authority defines a bounded intended use and acceptance criteria.

Passing E0-E3 does not authorize E4 or E5. No software benchmark establishes airworthiness or grants certification credit.

## 8. Research hypothesis

AERO's hypothesis is not that its graph, signature or LLM is superior to existing aerospace engineering tools. It is that a portable claim episode can bind owner-controlled configuration, simulation, V&V, failure-case, limitation, chronology and lifecycle evidence into a deterministic, independently replayable disposition—while preserving owner and regulatory authority.

The hypothesis is falsified if matched expert/PLM/assurance workflows perform the same reconstruction with equal or better critical error, reviewer effort, temporal fidelity and portability, or if AERO cannot reproduce decisions without importing undocumented context from the original runtime.
