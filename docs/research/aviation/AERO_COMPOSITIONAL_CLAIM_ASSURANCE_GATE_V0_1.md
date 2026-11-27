# AERO Compositional Claim-Assurance Gate v0.1

**Status:** research-only Rust reference gate, pending exact-head CI and independent semantic-oracle review  
**Owner:** Symtropy / AERO claim-assurance integration  
**Consumes:** observer coverage, vector causal cut, event/acquisition effectivity intervals and owner-supplied required-evidence admissions  
**Claim ceiling:** reviewability for independent decision only; no physical qualification, airworthiness, certification, authorization or inference of universal absence

## 1. Research question

Can AERO combine independently evaluated assurance obligations without collapsing them into a scalar score, majority vote or optimistic Boolean?

The gate accepts the outputs of existing owner-specific evaluators and composes them into one claim-level disposition. It does not duplicate observer discovery, receiver ledgers, source checkpoint ownership, trust roots, cryptographic receipt verification, model validation or the Symthaea action selector.

**A critical composition invariant:** all component reports and every mandatory evidence reference must bind to the same typed, computed decision context. The evaluator derives its SHA-256 from claim identity, physical subject, intended configuration, operating envelope, hard acquisition interval, as-known cutoff, source-scope manifest, owner evidence cut, trust policy, claim specification, required-evidence manifest, risk classification and model-applicability profile. The required-evidence manifest is recomputed from the exact input references. Component report SHA-256 is recomputed over the actual typed assessment plus its claim/subject/context/evaluator envelope; stale or substituted reports reject. See [AERO Typed Decision-Context Envelope v0.1](AERO_DECISION_CONTEXT_ENVELOPE_V0_1.md).

The canonicalization is a versioned Rust/Serde JSON field-order profile, not a general RFC 8785 implementation. External manifest digests and identifiers remain caller/owner-supplied: the module does not resolve or authenticate their bytes, verify an external receipt or issuer, or prove source completeness. A shared independently implemented cross-language profile and owner-native cryptographic verification remain open gates.

## 2. Paradigm shift: from record validation to claim-level admissibility

AERO already has separate primitives for:

- qualified observer coverage within a declared scope;
- source/partition watermark and causal closure of a declared event cut;
- conservative interval-to-interval event effectivity;
- owner-supplied evidence admission states.

The gate composes these facts against one explicit claim identity. A positive result means only that the supplied obligations are satisfied enough for an independent decision-maker to review the claim. It is not a claim that the underlying physical proposition is true.

This is deliberately non-compensatory. A million valid records cannot compensate for one missing required source, a rejected integrity commitment, an unknown stream epoch, an ambiguous effective-time boundary or an absent mandatory receipt.

## 3. Status semantics and precedence

| Status | Meaning | Required consequence |
|---|---|---|
| `REJECTED` | Malformed identity, contradictory integrity, invalid interval, duplicate required evidence reference or rejected required evidence | Reject this evidence package; do not infer the physical proposition |
| `OUT_OF_SCOPE` | Claim is outside the declared qualified observer scope | No inference for this claim |
| `UNVERIFIED` | Observer/source trust is unresolved or a mandatory evidence set is absent/unverified | No positive claim; obtain owner-native evidence |
| `INCOMPLETE` | Required coverage/predecessor/evidence is known missing or event/acquisition intervals overlap | Abstain; preserve the gap |
| `NOT_EFFECTIVE_AT_SUBJECT_TIME` | Event is definitely later than every acquisition time admitted by the hard bounds | Do not apply that event to this subject-time reconstruction |
| `REVIEWABLE_FOR_INDEPENDENT_DECISION` | Every supplied required obligation is satisfied | Send to a competent independent reviewer; no approval or actuation implied |

Precedence is fail-closed: integrity rejection dominates; scope limits remain explicit; unresolved trust precedes known incompleteness; ambiguous chronology abstains; only the fully satisfied conjunction reaches independent review.

The evaluator returns stable sorted reason codes. It does not compute a weighted score, posterior probability, confidence score, safety margin or EVSI. No failed critical obligation is averaged away.

## 4. Exact-context composition

Each component supplies an `AssessmentBinding` containing:

- exact claim ID;
- physical article/subject reference;
- SHA-256 digest of the canonical decision context;
- SHA-256 digest of the component report;
- exact evaluator profile/version.

Coverage, causal-cut and effectivity bindings must agree on claim, subject and decision-context digest. Their component report digests must be well-formed and distinct. Every mandatory evidence reference separately carries the same decision-context digest. Any mismatch or malformed binding rejects the composite package; a valid binding shape is not cryptographic authentication.

The composite output preserves the shared decision-context digest, all three report digests and a sorted set of required evidence references. This enables downstream review to retrieve the exact component reports and inspect the provenance chain rather than relying on an unanchored aggregate PASS.

The gate computes the context digest from the typed input. Owners still have to supply the referenced manifest/policy digests and retained component report bytes. A production integration must independently reproduce the versioned serialization, resolve the source-owned manifests, authenticate each owner receipt and verify the component report digest against its retained bytes.

## 5. Evidence and authority boundaries

`ADMITTED` is an upstream owner/evidence-workflow disposition. This module does not cryptographically authenticate the receipt, verify a signature, establish source authority, qualify an observer, or prove that source discovery is complete. AERO must consume owner-native Mycelix/AeroCommons receipts and BASALT qualification; it must not mint a parallel evidence ledger.

`REVIEWABLE_FOR_INDEPENDENT_DECISION` is the strongest output. The result explicitly sets:

- `authority_granted = false`;
- `physical_qualification_status = OUT_OF_SCOPE_NOT_CLAIMED`;
- `absence_inference_granted = false`.

A production integration must retain the exact evaluator profile, evidence packet digest, component report digests, claim identity, owner receipt references, trust-policy snapshot and verifier build identity. That integration is not implemented by this module.

## 5. Verification and challenge strategy

The Rust tests cover:

- positive conjunction;
- invalid observer, causal-cut and time inputs;
- rejected, missing, unverified, malformed and duplicated mandatory evidence;
- out-of-scope and incomplete coverage;
- causal gaps and unresolved watermarks;
- overlapping effective/acquisition intervals;
- events definitely later than the acquisition;
- permutation invariance;
- single-obligation degradation (no degraded input may preserve reviewability);
- explicit absence of authority, physical qualification and absence inference.

The synthetic unit tests exercise the composition logic; they are not an independently authored oracle. The review gate remains `NOT_INDEPENDENTLY_REVIEWED` until a reviewer outside the implementation authors and adjudicates an independent mutation corpus.

## 6. Benchmarks and target gates

The first release should be compared with a deliberately weak baseline that uses one aggregate PASS/FAIL flag and another baseline that allows weighted scores to compensate for missing inputs.

Measure:

1. **Catastrophic false-review rate:** fraction of adversarially invalid/incomplete/out-of-scope cases incorrectly classified reviewable. Target: 0 on a frozen, independently adjudicated challenge set; always publish numerator, denominator and confidence interval.
2. **Required-obligation sensitivity:** fraction of injected mandatory-obligation failures that prevent reviewability. Target: 100% for the declared finite contract.
3. **Reason-code completeness:** fraction of injected faults with the expected reason family retained. Target: 100% for the frozen, in-scope oracle.
4. **Mutation adequacy:** surviving semantic mutations in the gate implementation. Target: 0 surviving mutations across the named precedence, empty-set, duplicate-identity and non-compensation mutations.
5. **Order invariance:** identical result under all permutations of independent evidence references. Target: exact structural equality.
6. **Review abstention quality:** report reviewability, abstention and rejection separately by fault family; do not optimize reviewability alone.
7. **Cost:** p50/p95/p99 latency and allocations at 1, 10, 100, 1,000 and 10,000 required evidence references, with the full Rust toolchain, machine, build flags and input digest recorded.

Do not claim superiority over an industry system from a synthetic harness. The fair comparator is first a matched semantic baseline, then an independently reproduced end-to-end workflow. Performance is secondary to zero false-review outcomes on required hard gates.

## 7. Research path to a stronger assurance system

1. Obtain independent semantic-oracle review of the coverage, causal-cut, effectivity and claim-gate dispositions.
2. Replace synthetic owner labels with cryptographically verified owner-native receipts, exact byte commitments and pinned trust-policy snapshots at the correct system boundary.
3. Bind the claim gate to the existing temporal replay report, including the three historical cutoffs and uncertain effective/acquisition time intervals.
4. Bind model-specific evidence to the existing AeroWeft applicability envelope and risk-scaled VVUQ dossier; no transfer of validation credit across model/configuration/domain changes.
5. Feed evidence gaps to Symthaea's existing selector only after its independent selector inputs and resource accounting are preserved. The gate itself never ranks or executes actions.
6. Run held-out model, configuration, sensor, source-omission, common-cause, clock, trust-revocation and change-feed adversarial campaigns.
7. Conduct physical SHM/NDE pilots only under a preregistered lab protocol with item-level data rights, independent ground truth and method-specific probability-of-detection treatment.

## 8. Limitations

The module composes supplied evaluator outputs; it cannot detect a flaw shared by all upstream evaluators. It does not independently prove the completeness of the source universe, authenticate owner receipts, validate calibration or measurement truth, quantify model uncertainty, assess structural damage, perform NDE, calculate POD, determine damage tolerance or replace authorized engineering review. A fully satisfied conjunction is a prerequisite for review, not a substitute for the underlying discipline.


## 10. Industry comparison and defensible differentiation (2026-11-26)

This is a capability comparison, not a vendor-product benchmark. Public product pages describe intended capabilities; they do not expose matched independent assurance results.

| Comparator | Demonstrated industry emphasis | AERO should complement or exceed in this specific dimension | What AERO must not claim |
|---|---|---|---|
| NASA-STD-7009B + NASA-HDBK-7009B | Model/simulation lifecycle credibility; program/project acceptance criteria approved by delegated technical authority; handbook implementation guidance | Machine-checkable, claim-specific evidence closure and reproducible cross-owner evidence packets | NASA compliance, technical-authority acceptance, validated physics || IAQG 9100-series / SAE AS9100D (revised 2025-12-08) + IAQG 9104 scheme | Established aerospace QMS, accredited third-party certification, audit oversight and OASIS supplier status | Integrate AERO evidence identities, change impacts and historical replay into an existing QMS; make evidence packages independently reproducible | AS9100 conformity, ICOP certification, audit approval, supplier approval or regulatory acceptance |

| NIST AMS 100-85 (2026-10-05) | Risk-scaled VVUQ based on model influence and decision consequence; harmonization of ASME computational modeling and JCGM metrology practices | Bind risk tier, uncertainty budget, evidence provenance and exact claim context into one reviewable record | A new universal VVUQ method or proof of physical model fidelity |
| Siemens Aerospace Digital Thread / Aircraft Certification | MBSE, requirements, architecture, verification, certification-plan traceability and connected lifecycle data | Portable, owner-independent negative-evidence semantics and compositional assurance across vendor and owner boundaries | PLM/MBSE replacement, stronger lifecycle integration or shorter certification cycle before matched pilots |
| Dassault 3DEXPERIENCE / AI for Systems Engineering | Virtual-twin/MBSE continuity, simulation, requirements traceability, impact analysis and AI-assisted systems engineering | A narrow independent assurance kernel with explicit abstention, exact context binding and auditable non-compensation | A richer virtual twin, broader industrial ontology, or equivalent production integrations |
| ASTM E2862-23 / FAA 14 CFR 25.571 / AC 25.571-1D | Method-specific POD analysis and transport-aircraft damage-tolerance, crack-growth, inspection and limit-of-validity requirements | Preserve inspection-method, subject, configuration, time and evidence lineage as qualified inputs to a review | Replacing NDE, POD demonstration, fatigue testing, damage-tolerance analysis, maintenance programmes or regulatory acceptance |

### Where a genuine paradigm shift is available

1. **Assurance as a claim graph, not a document folder.** Every engineering claim should identify its subject, configuration, operating envelope, time coordinates, dependencies, evidence reports, uncertainty and owner. Changes invalidate only claims whose declared dependencies intersect the change, while incomplete change feeds make the affected frontier indeterminate.
2. **Proof-carrying evidence, not trusted metadata.** Component outputs must reference immutable report bytes, canonical context, owner receipts, trust-policy snapshots and verifier builds. A digest's shape is not authentication. The next production adapter must verify bytes and trust evidence at the owner boundary.
3. **Non-compensatory uncertainty.** Keep model-form discrepancy, parameter uncertainty, measurement uncertainty, sensor/time uncertainty, missingness and domain shift distinguishable. A favourable mean or a narrow confidence interval cannot cancel a failed hard bound or an unsupported operating stratum.
4. **Evidence acquisition as an optimization problem.** Symthaea's existing selector should optimize expected reduction in decision-relevant uncertainty per cost, with robust/distribution-free fallback when probabilities are not calibrated. Benchmark cost per independently reviewed decision—not number of tests proposed or records ingested.
5. **Adversarially qualified digital thread.** Red-team omitted observers, common-cause failures, identity substitution, backdated changes, stream forks, replay, compromised credentials, stale trust policy, model drift and configuration changes. Every test family needs independently authored expected dispositions.
6. **Open interoperability over platform replacement.** Ingest owner-native PLM/MBSE, metrology, NDE, sensor and simulation evidence through versioned adapters; preserve source semantics and raw bytes. Never turn an import into an admission or physical truth assertion.

### Benchmark programme: target scoreboard, not achieved results

| Dimension | Matched baselines | Prospective target |
|---|---|---|
| Catastrophic false-review | Aggregate PASS/FAIL; weighted compensating score; AERO conjunction | 0 false-review outcomes on the frozen, independently adjudicated hard-gate suite; report numerator, denominator, fault-family coverage and finite-sample confidence bound |
| Context integrity | Unbound component reports; claim-ID-only binding; exact claim/subject/context binding | 0 cross-claim, cross-subject or cross-context compositions accepted |
| Required evidence | Vacuous empty-set; Boolean present/missing; four-state admission | 100% of injected mandatory evidence faults prevent reviewability; no empty-set pass |
| Historical replay | Latest-row only; scalar wall clock; bitemporal; three-axis replay plus hard-bound event/acquisition intervals | 100% agreement with independent oracle on frozen deterministic cases; no late-received event affects AS-KNOWN-AT |
| Causal closure | Declared event rows; vector watermarks; vector plus predecessor-DAG closure | 100% correct result on finite oracle cases; no lost, forked or future predecessor accepted |
| Model credibility | Random row split; configuration/flight-group split; risk-scaled VVUQ | 0 leakage across confirmatory groups; explicit NOT_COVERED/INDETERMINATE outside the validated envelope; report per-stratum interval coverage and width |
| Change impact | Global invalidation; latest current flag; claim-dependency intersection with complete/incomplete change feed | Exact affected-claim frontier on complete synthetic feeds; no claim marked unaffected when the frontier is incomplete |
| NDE/SHM | Detection accuracy or ROC-AUC alone; method-specific POD and independent ground truth | Report POD curve, uncertainty, sample/exposure denominator, false calls and performance by defect type, depth, orientation, material, sensor and environment; thresholds are method/authority-specific |
| Performance | Naive scan; hash-indexed; vector/DAG; full owner-receipt path | p50/p95/p99, throughput, peak memory and storage at 1k/10k/100k/1M events and 1/10/100/1k/10k evidence references; asymptotic behaviour and hardware disclosed |
| Engineering economics | Manual requalification; global retest; claim-targeted requalification | Prospective ≥20% lower cost per independently reviewed decision at matched risk and evidence quality; not credited until a controlled pilot |
| Reproducibility | Producer-only report; independent replay | Independent evaluator reproduces the context, component digests, dispositions and all published metrics from retained source bytes |

For zero observed false-review events, publish a finite-sample upper confidence bound rather than writing "zero risk." Under an iid Bernoulli approximation, the rule-of-three gives an approximate 95% upper bound of 3/N after zero failures; this is only a descriptive bound for the sampled challenge population. Fault-family dependence, selection bias and distribution shift require separate treatment. A thousand generated mutations from one generator are not a thousand independent assurance cases.

### Minimum fair head-to-head protocol

- Freeze exact model, configuration, subject, acquisition window, operating domain, trust policy, evaluator version and raw-data manifests before the confirmatory run.
- Use the same input bytes, fault injections, resource budget, reference measurements and expected dispositions for each baseline.
- Separate calibration/development from confirmatory data by physical article, production lot, flight/configuration, campaign and environmental episode, not random rows.
- Blind the implementation team to confirmatory outcomes; publish failed/censored cases and all exclusions.
- Have an independent team write and adjudicate the oracle before running the candidate system.
- Publish code SHA, input and output digests, environment, hardware, confidence bounds and all deviations.
- Do not combine hard-gate safety outcomes with latency, throughput, RMSE or cost in one weighted score.

### Sources and scope notes

- [NASA-STD-7009B](https://standards.nasa.gov/node/263) is active; its stated scope includes lifecycle credibility assessments and acceptance criteria approved by delegated technical authority.
- [NASA-HDBK-7009B](https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009) is active and dated 2026-02-03; it is implementation guidance, not itself a technical acceptance certificate.
- [NIST AMS 100-85](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing) was published 2026-10-05 and frames VVUQ effort around model influence and decision consequence.
- [Siemens aerospace MBSE](https://www.siemens.com/en-us/digital-thread/mbse/aerospace/) and [aircraft certification](https://www.siemens.com/en-gb/digital-thread/integrated-lifecycle-management/aircraft-certification/) describe integrated requirements, architecture, verification and certification-data threads.
- [Dassault Systèmes AI for Systems Engineering](https://www.3ds.com/industrial-ai/generative-virtual-twins/systems-engineering) describes AI-assisted MBSE, simulation and requirements traceability on 3DEXPERIENCE.
- [ASTM E2862-23](https://store.astm.org/e2862-23.html) is specifically a statistical practice for POD analysis of hit/miss NDE data under defined examination parameters; it is not a universal detection or certification threshold.
- [14 CFR 25.571](https://www.govinfo.gov/content/pkg/CFR-2025-title14-vol1/xhtml/CFR-2025-title14-vol1-sec25-571.html) sets damage-tolerance/fatigue evaluation requirements for transport-category aircraft structure, including analysis supported by test evidence and required inspections/limits.
- [FAA AC 25.571-1D](https://www.faa.gov/airports/resources/advisory_circulars/index.cfm/go/document.information/documentNumber/25.571-1D) is active guidance for damage-tolerance and fatigue evaluation.

These sources define comparators and boundaries. None is a direct benchmark result for AERO, and vendor capability pages do not establish a matched superiority result.