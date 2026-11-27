# AERO Model-Evidence Artifact Assurance Contract v0.1

**Status:** research-only owner-interface proposal  
**Base:** AERO model-credibility claim integration PR #7742  
**Claim ceiling:** evidence-admission contract only; no VVUQ calculation, physical qualification, airworthiness, certification or authorization  
**Purpose:** close the gap between a typed model-credibility cut and verifiable evidence for the exact model-report bytes and their producer.

## 1. Why this increment exists

PR #7742 binds the typed model-credibility cut to the claim context and to the matching owner-evidence record digest. That is necessary, but it does not by itself establish that the exact AeroWeft report bytes were verified, that the producer was authorized to issue them, or that the report was received by the historical evidence cutoff.

The following identities must remain distinct:

1. **Raw report bytes:** exact content-addressed model report, with byte length and SHA-256.
2. **Typed model cut:** AERO's versioned, normalized interpretation of the owner report. Its digest is not interchangeable with the raw report digest.
3. **Owner receipt:** the owner-native record that says who supplied which artifact, when it was first received and which verifier evaluated it.
4. **Semantic result:** AeroWeft's claim about model validation, applicability, uncertainty and strata.
5. **AERO disposition:** the claim-level composition of the above with all other obligations.

A digest establishes byte identity only after the bytes have been obtained and checked. A valid signature establishes a statement's origin under a trust policy; it does not prove that the report is scientifically correct. A schema-valid report does not establish model applicability. None of these can substitute for another.

## 2. Ownership is a feature, not a problem to erase

The contract deliberately assigns the work to existing owners:

| Concern | Owner | AERO's role |
|---|---|---|
| Model identity, validation semantics, applicability profile, modality requirements, uncertainty decomposition and per-stratum metrics | AeroWeft / model owner | Consume exact claim-scoped disposition; never calculate or reinterpret VVUQ |
| Artifact bytes, packet membership, physical-unit/configuration identity and lifecycle | Mycelix / AeroCommons | Consume the owner artifact and receipt references |
| Producer signatures, key status, issuer authority, trust-policy history and receiver-first-seen | Owner-native receipt/trust verifier | Require owner-verified dispositions and bind their exact receipt digests |
| Resource generations, dependency completeness, staleness and re-verification frontier | BASALT | Consume freshness/coverage dispositions; do not implement another dependency ledger |
| Evidence-action choice and value of information | Symthaea | Expose exact blockers and resolvable evidence classes; do not select or authorize actions |
| Claim composition | AERO | Enforce the conjunction and preserve all blockers |

This proposal does not assert that the current Mycelix/AeroCommons or BASALT branches already implement every receipt described here. Mycelix PR #6347 demonstrates exact artifact byte-length/SHA-256 checking but explicitly does not authenticate the publisher. PR #6355 adds run receipts but does not authenticate a signer or prove a runner executed. BASALT PR #9203 keeps confirmatory eligibility false until trusted observation and signed runner receipts are established. This contract consumes those owners' future qualified outputs; it must not manufacture those outputs inside AERO.

References:
- Mycelix [#6347](https://github.com/Luminous-Dynamics/mycelix/pull/6347), [#6355](https://github.com/Luminous-Dynamics/mycelix/pull/6355), [#6360](https://github.com/Luminous-Dynamics/mycelix/pull/6360).
- Symthaea BASALT [#9203](https://github.com/Luminous-Dynamics/symthaea/pull/9203), trusted-runner follow-on [#9210](https://github.com/Luminous-Dynamics/symthaea/issues/9210), observer challenge corpus [#9269](https://github.com/Luminous-Dynamics/symthaea/pull/9269).

## 3. Five independent assurance axes

A model-report attestation is not one Boolean. It is a tuple of independently qualified dimensions:

### A. Payload integrity

The owner verifier resolves the exact artifact bytes and recomputes both byte length and digest. The raw report digest is kept separate from the typed model-cut digest.

- `verified`: exact bytes match the declared length and digest, with an owner verification receipt.
- `unverified`: bytes or an adequate receipt are unavailable.
- `rejected`: the bytes contradict the declared identity or the owner verifier rejects them.

### B. Producer authority

The owner trust verifier establishes the producer identity, signing-key state, role/authority scope and trust-policy snapshot for the exact artifact or signed statement.

- `verified`: authorized producer under the pinned trust-policy snapshot.
- `unverified`: identity, authority, key state or historical trust evidence is unresolved.
- `rejected`: signature, identity, authorization or policy check fails.

A signature that is valid today must not silently rewrite what was trusted at an earlier cutoff. The owner receipt must preserve the trust-policy digest used for the decision and the verifier's receipt. A signed assertion of completeness is not proof that the observer saw every input.

### C. Semantic validation

The AeroWeft owner (or a separately qualified independent verifier) supplies the result and exact validation profile/report receipt.

- `passed`: all required model-report semantics were evaluated and passed under the declared profile.
- `incomplete`: a mandatory semantic result, metric, stratum or modality is missing or indeterminate.
- `rejected`: the report is malformed, contradictory, leaked, or explicitly fails a required obligation.
- `not_assessed`: no semantic verification is available.

AERO never interprets a hash match as semantic validation.

### D. Claim applicability

The attestation binds the exact claim, physical unit, configuration, operating envelope, model, solver, applicability profile and risk profile. The applicability disposition remains a separate result:

- `in_scope`
- `out_of_scope`
- `indeterminate`

A report for another aircraft, article, configuration, solver or risk class cannot be substituted, even when its bytes and signature are valid.

### E. Knowledge time and freshness

Receiver-first-seen is an owner observation, not the source's claimed recorded time. The exact receiver receipt and its bounded timestamp evidence must be preserved. BASALT remains responsible for resource-generation pins, evidence dependencies, incomplete coverage and freshness invalidation. The AERO adapter must not infer that a receipt is current merely because its digest is unchanged.

These axes must remain visible in the final claim report. Do not collapse them to `evidence_verified=true`, a weighted score, a majority vote or a single confidence number.

## 4. Proposed receipt shape

The companion Draft 2020-12 schema defines an adapter-neutral receipt with:

- exact raw artifact reference, role, media type, byte length and SHA-256;
- typed claim/context identity and model/solver/configuration/profile digests;
- payload-integrity disposition plus owner verifier identity, build/profile digests and verification receipt;
- producer-authority disposition plus producer, role, authority reference, signing key, pinned trust policy and receipt;
- semantic-validation disposition plus verifier, build, validation profile, report digest and receipt;
- bounded receiver-first-seen interval and owner timestamp/receipt reference;
- owner system, owner packet and owner receipt identity.

Receipt references and digests are pointers to owner-native evidence, not cryptographic verification performed by AERO. AERO must not accept a caller-supplied status without the corresponding owner receipt being admitted by that owner's trust boundary.

## 5. Non-compensatory AERO mapping

| Condition | AERO consequence |
|---|---|
| Raw artifact digest/length contradiction | `REJECTED` |
| Producer signature or authority rejected | `REJECTED` |
| Producer identity, key state, trust snapshot or receipt unresolved | `UNVERIFIED` |
| Byte-integrity verification absent | `UNVERIFIED` |
| Semantic validation rejected | `REJECTED` |
| Semantic validation incomplete or indeterminate | `INCOMPLETE` |
| Semantic validation not assessed | `UNVERIFIED` |
| Model applicability out of scope | `OUT_OF_SCOPE` |
| Model applicability indeterminate | `UNVERIFIED` or `INCOMPLETE`, according to the owner disposition; never positive |
| Receipt first-seen after the historical evidence cutoff | `NOT_KNOWN_AT_SNAPSHOT`; exclude from that historical view |
| BASALT freshness/dependency coverage is stale or incomplete | Preserve BASALT's non-positive disposition |
| All axes qualified and all other mandatory claim obligations satisfied | At most `REVIEWABLE_FOR_INDEPENDENT_DECISION` |

An owner-rejected result must never be downgraded to a warning. An unverified result must never be converted to incomplete merely to permit a retry path. The original owner status and reason codes remain in the report.

## 6. Independent challenge programme

The contract should be evaluated against independently authored, blinded cases. The expected result must be authored by a reviewer/oracle that does not reuse the Rust gate implementation, its helper functions or its generated expected-output table.

Minimum challenge families:

1. Exact bytes match; positive control.
2. Single-byte report mutation with unchanged digest.
3. Correct digest but wrong byte length.
4. Correct bytes from an unauthorized producer.
5. Valid signature from a key revoked under the historical trust snapshot.
6. Valid signature under current trust but not under the original snapshot.
7. Producer role valid but authority scope does not cover the report type.
8. Correct report and producer, wrong physical unit.
9. Correct report and unit, wrong configuration or operating envelope.
10. Correct model artifact but wrong solver configuration.
11. Typed cut digest substituted for raw report digest.
12. Semantic report incomplete while byte and signature checks pass.
13. Unsupported model domain with strong in-domain metrics.
14. One failed or missing required stratum hidden by otherwise positive strata.
15. Missing independent reference or split leakage.
16. Receiver-first-seen after the historical evidence cutoff.
17. Backdated issuer timestamp with later receiver-first-seen.
18. Stale BASALT dependency after a resource-generation change.
19. Incomplete dependency-observation coverage with no known resource change.
20. Irrelevant dependency change that should preserve unrelated evidence.
21. Conflicting owner receipts for the same source-scoped identity.
22. Trust-policy update between historical and present-day replay.
23. Duplicate artifact identity with different payload bytes.
24. Report correction that must preserve the predecessor and original historical view.

Each case should record the injected fault(s), proposed disposition, proposed reason-code set, oracle rationale, evidence scope, independent adjudicator, and whether the case belongs to a public or blinded holdout partition. The public seed corpus and schema are checked in under the companion example/schema files. Its proposed outcomes are explicitly not independently adjudicated and none of the cases has been executed. Coupled-fault cases are mandatory: tests must include byte mutation plus a valid unrelated receipt, stale evidence plus a risk/profile substitution, and late receipt plus backdated issuer time.

The oracle author must not derive expected results from the implementation under test. At least one independent implementation or hand-derived adjudication must consume the raw challenge facts. A second implementation that copies the same fixture-generation assumptions is not an independent oracle.

## 7. Benchmark targets and statistical interpretation

These are proposed gates, not achieved results or industry standards.

| Measure | Initial gate | Qualification target |
|---|---:|---:|
| Hard-gate false review | 0 observed on 300 independent negative cases | 0 observed on 2,995 independent negatives for an approximate 0.1% one-sided 95% upper bound |
| Context/subject/configuration/solver/policy substitution | 0 accepted | 0 accepted across single- and coupled-fault campaigns |
| Byte integrity and producer-authority controls | 100% expected outcomes | 100% on frozen and blinded suites |
| Missing/unverified owner receipts | 100% non-positive | 100% preserved as non-positive |
| Temporal knowledge-cut behavior | 100% independent-oracle agreement | 100% across all three temporal replay modes |
| Model-domain and per-stratum blockers | 100% preserved | 100%; no cross-stratum or cross-modality compensation |
| Critical mutation score | 100% hard-gate mutants killed | 100% hard-gate mutants killed, with all surviving noncritical mutants published |
| Requalification selectivity | 100% affected-claim recall | Minimize unnecessary re-verification only after zero missed invalidations |
| Scale | 10k and 100k events | 1M and 10M events, 1–10k evidence references |
| Decision economics | Measure reviewer time/cost | Prospective ≥20% lower cost per independently reviewed decision at matched risk |

For zero failures in n independent Bernoulli trials, the one-sided 95% upper bound is approximately 3/n. Thus zero misses in 300 cases is only an initial screen (about 1% upper bound); it is not evidence of zero real-world risk. Approximately 2,995 independent cases are needed for a 0.1% upper bound under that simple model. Cases sharing a root cause, generated from the same template, or differing only by repeated event rows are not independent trials. Report counts by challenge family and use cluster-aware analysis where appropriate.

Publish false-review, false-reject, unresolved/abstention, and risk-coverage results separately. Do not optimize a single score or allow the false-review target to be traded against higher approval throughput.

Performance comparisons must use the same claim set, semantic obligations, hardware, resource budgets, cache conditions and verifier versions. Report p50/p95/p99, peak RSS, CPU time, bytes read, evidence references, cold/warm cache, and the cost of producing owner receipts separately from replay. Synthetic throughput is not a safety or SOTA claim.

## 8. Relationship to current standards and infrastructure

AERO should interoperate with, not replace:

- ISO 23247-5:2026, which specifies a manufacturing digital thread across the product lifecycle, and ISO 23247-6:2026, which specifies composition and interoperation of digital twins developed by different parties.
- NIST IR 8536 (September 2026), an industry-neutral manufacturing traceability meta-framework for securely linking and verifying supply-chain traceability information while allowing organizations to continue using existing standards.
- NASA-STD-7009B and the active 2026 NASA-HDBK-7009B implementation guide for model-and-simulation credibility practices.
- IAQG 9100-series quality-management and 9104 certification scheme, including third-party audit and oversight.
- FAA 14 CFR 25.571 / AC 25.571-1D damage-tolerance and fatigue obligations, and method-specific standards such as ASTM E2862-23 for hit/miss probability-of-detection analysis.

The differentiator is not a broader digital thread, a new VVUQ authority, a QMS certificate or an NDE method. It is a portable, non-compensatory claim disposition that preserves owner-authenticated evidence, exact context, historical knowledge and model applicability across those existing systems.

## 9. Promotion gates

Before this contract can influence any physical pilot:

1. AeroWeft independently reviews the required modalities, split rules, metric/stratum definitions and applicability semantics for each claim class.
2. Mycelix/AeroCommons exposes an owner-native receipt for exact model-report bytes, packet membership and physical subject/configuration.
3. The owner trust boundary authenticates the producer/authority and preserves the exact trust-policy and receiver-first-seen evidence.
4. BASALT independently qualifies dependency observation and freshness receipts; `confirmatory_eligible=false` remains a hard blocker until its owner changes that status on evidence.
5. An independent challenge oracle and blinded holdout agree with the AERO result on frozen cases.
6. Exact-head formatting, tests, Clippy and schema checks pass on the exact branch head.
7. A competent engineering authority independently evaluates the underlying physical claim.

Until then this remains a research contract. It creates no operational authority and makes no physical qualification claim.

## 10. Existing change-impact and requalification work to reuse

AERO must not create a second bitemporal invalidation engine or a generic obligation ledger. The repository already contains the aviation-specific Track U model-evidence requalification work:

- [Symtropy PR #5571](https://github.com/Luminous-Dynamics/symtropy/pull/5571) — current documented exact head `936dc49f684ced9acb5dabf2c9c6d35778616c29`, with workflow run [#43850752898](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43850752898) completed successfully. Its reported suite contains 46 passing tests. It computes per-claim, per-dependency affected, requalification, contradicted, historical-only, unsupported and indeterminate dimensions under separate effective-time and knowledge-time cutoffs, source stream sequence and bounded change-feed coverage.
- [Symtropy PR #5426](https://github.com/Luminous-Dynamics/symtropy/pull/5426) is an earlier Track U predecessor with a 43-test exact-head run.
- [Mycelix PR #6351](https://github.com/Luminous-Dynamics/mycelix/pull/6351) supplies an AeroCommons-side gate for mandatory/conditional/unknown requalification obligations. It blocks optional candidate tests while mandatory or unknown obligations remain open. Its closure receipt remains a typed reference, not proof of physical-test adequacy or authenticated issuer.
- [Mycelix PR #3641](https://github.com/Luminous-Dynamics/mycelix/pull/3641) provides AeroEvidence and conservative configuration-impact primitives. Mycelix #6219/#1087/#2631/#1207/#181 remain the general evidence-invalidation, applicability, temporal and lease owners referenced by Track U.

The integration target is therefore a narrow typed composition seam:

1. Track U computes the aviation claim's exact dependency-impact dimensions and the evidence/coverage frontier under its bitemporal semantics.
2. Mycelix/AeroCommons supplies owner-native evidence lifecycle, change-set, obligation and closure receipts.
3. BASALT supplies resource-generation freshness and independently qualified dependency-observer coverage.
4. AERO binds these exact reports to the same claim, article, configuration, operating envelope, model and decision-context digest, then composes their non-positive states without collapsing their reason sets.
5. Symthaea may rank evidence-acquisition options against the resulting blockers and component-wise resource constraints, but cannot close the obligations or authorize work.

Before reuse, verify that the Track U output schema can bind the current #7742 decision-context envelope without translation loss. In particular, preserve both time-scale identifiers, source/stream identity, sequence frontiers, effective and knowledge cutoffs, policy digest, claim package digest and each per-dimension result set. Any unmappable dimension becomes `UNKNOWN/INDETERMINATE`, not silently dropped.

The #5571 workflow pass is exact-head evidence for that separate Track U branch only. It does not qualify #7742, #7765 or a cross-branch integration. The current owner contracts and adapters still require exact-head integration tests and independent semantic review.

## 11. Benchmark-method reuse without cross-domain overclaim

The benchmark design should reuse the method—not the labels, cases or claimed outcomes—of Mycelix [#6776](https://github.com/Luminous-Dynamics/mycelix/pull/6776). That separate synthetic water-service assurance benchmark has a recorded exact head `6ee626d5f7f88b3226226ea13e15d1a72a297650` and two successful exact-head workflow runs (#14 and #16242). Its useful method includes 18 metamorphic relations, 34 adversarial mutation operators, 20 negative controls, 17 machine-readable metrics with numerators/denominators/targets/field-data requirements, and comparator profiles for manual experts, checklists, single/RAG/multi-agent LLMs and established PLM/QMS platforms.

AERO must author a domain-specific aerospace challenge suite and independently adjudicate every expected disposition. The water-service corpus is not evidence of aerospace model credibility, temporal correctness, airworthiness or physical qualification. The benchmark transfer is limited to these practices:

- Register positive controls, hard negative controls, boundary cases, coupled faults and metamorphic relations before implementation tuning.
- Preserve negative controls where a valid but irrelevant change should not invalidate an unrelated claim, so a reject-everything gate cannot score well.
- Publish executable metric definitions with numerators, denominators, inclusion/exclusion rules and field-data requirements.
- Include human domain experts, structured checklists, LLM baselines and PLM/QMS comparators under the same claim set, evidence rights, risk class, time cutoffs and review task.
- Count abstentions, incomplete claims, failed runs and incomparable contexts explicitly; never drop them from denominators or call them successful.
- Keep public development seeds separate from independently adjudicated blinded holdouts.

The AERO comparison should report the risk-coverage curve and false-review rate alongside reviewer time, evidence-acquisition cost, unnecessary requalification, replay reproducibility and resource consumption. Any prospective economic benefit (including the ≥20% cost reduction target) must be measured at matched risk and independently reviewed decision quality.

## 12. Machine-readable benchmark metric registry

The companion registry defines 17 prospective metrics with fixed numerators, denominators, populations, inclusions, exclusions and required stratification. It is a preregistration artefact, not a result file: its status is `PROSPECTIVE_NOT_EXECUTED`, and its case-adjudication status is `NOT_INDEPENDENTLY_ADJUDICATED`.

Safety metrics remain separate from coverage and economics. False-review, context substitution, owner-proof bypass, missing mandatory obligations, unsupported model domains, missing required strata, temporal disagreement, missed invalidations and hard-gate mutation survival have prospective zero-observed targets. False rejection and abstention are report-only, so a reject-everything gate cannot win. The at-most-0.80 cost ratio is a prospective target only at matched risk and independently adjudicated decision quality. The unnecessary-requalification target is subordinate to zero missed invalidations. Replay p95 has no fixed target until workload and hardware baselines are matched.

The schema and registry are `schemas/aero_assurance_benchmark_metrics_v0_1.schema.json` and `examples/aero_assurance_benchmark_metrics_v0_1.json`. CI checks shape and metric census; it does not execute the benchmark.

