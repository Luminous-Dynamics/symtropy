# AERO Compositional Claim-Assurance Gate v0.1

**Status:** research-only Rust reference gate, pending exact-head CI and independent semantic-oracle review  
**Owner:** Symtropy / AERO claim-assurance integration  
**Consumes:** observer coverage, vector causal cut, event/acquisition effectivity intervals and owner-supplied required-evidence admissions  
**Claim ceiling:** reviewability for independent decision only; no physical qualification, airworthiness, certification, authorization or inference of universal absence

## 1. Research question

Can AERO combine independently evaluated assurance obligations without collapsing them into a scalar score, majority vote or optimistic Boolean?

The gate accepts the outputs of existing owner-specific evaluators and composes them into one claim-level disposition. It does not duplicate observer discovery, receiver ledgers, source checkpoint ownership, trust roots, cryptographic receipt verification, model validation or the Symthaea action selector.

**A critical composition invariant:** all component reports and every mandatory evidence reference must bind to the same claim identity, exact physical subject reference and canonical decision-context digest. The evaluator checks report-digest shape, component-report digest uniqueness, evaluator-profile presence and exact claim/subject/context agreement. It carries these bindings forward into its output. It does not recompute the report/context digests, authenticate the reports, or prove that the caller bound the correct underlying bytes; that remains an upstream receipt/verifier responsibility.

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

The caller still has to compute the canonical decision-context digest over a frozen, versioned serialization of the claim and its configuration, operating envelope, acquisition/time coordinates, policy and evidence requirements. The exact canonicalization profile must be included in the owner packet. A future production integration should make this a shared, independently implemented canonicalization contract and verify every report digest against the retained report bytes.

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
