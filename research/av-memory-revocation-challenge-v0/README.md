# AV Memory Revocation Noninterference Challenge v0

**Issue:** #6488  
**Parent long-horizon track:** #6357 / PR #6359  
**Source/replay contract:** #6201  
**Existing memory owners:** Symthaea #10644, #9075/#9177 and #12837

## Objective

Test whether source information remains retrievable after revocation through direct retrieval, summaries, embeddings, indexes, working context, response caches, cross-modal derivatives, replicas and backups.

This first tranche adds a workload schema, eight synthetic challenge episodes, evaluator-only canary truth and exact-head integrity workflow. It does not implement a memory/deletion service, nor prove physical erasure.

## A key assurance distinction

The challenge distinguishes:

1. **Policy noninterference:** serving/retrieval no longer uses, cites or returns revoked evidence.
2. **Residual-probe resistance:** independent fresh probes cannot elicit or rank the revoked information through tested memory paths.
3. **Storage erasure:** every copy in the declared storage scope has independently verifiable deletion evidence.

A candidate's own "purged" receipt is a report, not proof. Black-box probes can support the statement "no residual was found under these tested probes"; they cannot prove every copy has been physically erased. Storage erasure requires an explicit storage threat model and independently verifiable storage-layer evidence.

The workload includes synthetic canary text because these are mechanics fixtures. The evaluator truth file includes alias strings for deterministic exact-match checks. This repository is public, so neither file is a blind holdout. A true challenge must generate canaries after candidate freeze and keep truth, alias expansion and hidden probe variants in evaluator-controlled storage.

## Eight challenge paths

1. Direct source/session recall after revocation.
2. A paraphrase that can recover a canary from a retained summary.
3. Nearest-neighbour recall via embedding and retrieval-index derivatives.
4. Replayed answer-cache retrieval.
5. Working-context and replica retrieval after restart.
6. Cross-modal semantic leakage from an audio source into a visual event index.
7. Backup and secondary-region replica scope.
8. Selective revocation with an unrelated non-revoked control fact that must remain available.

Each episode defines a pre-revocation positive probe, post-revocation probes, source observation, derived-memory dependency chain and revocation scope profile. The pre-probe is important: a canary the system never possessed cannot be used as evidence that erasure succeeded.

## Data planes

- fixtures/agent/workload-v1.json: visible source content, derived item references, revocation event and probe prompts.
- fixtures/evaluator-only/truth-v1.json: canary values, alias forms and expected noninterference labels.
- fixtures/expected/known-answer-v1.json: finite expected probe outcomes.

The candidate should receive the source before revocation, then receive the revoke event, then receive fresh post-revocation prompts. It must not receive evaluator labels or alias lists. Candidate-visible workload is static for mechanics testing; real evaluation must stream events causally and keep the canary-label truth external.

## Future evaluator requirements

A candidate run should return, per probe:
- exact response and confidence;
- source references and derived-memory references used;
- whether the query was answered or abstained;
- a run/session/storage-scope identity;
- candidate purge receipts and object references as supporting evidence only.

The evaluator should measure:
- pre-revocation positive-canary retrieval recall;
- direct, paraphrase, semantic, nearest-neighbour, cache, restart, replica and cross-modal residual leakage;
- policy ineligibility time and storage erasure time separately;
- purge witness coverage over the dependency DAG;
- false-positive deletion on non-revoked controls;
- source-reference or derived-memory reuse after revocation.

Exact string/alias matching is appropriate for the synthetic v0 harness. Semantic leakage against real models requires a preregistered independent adjudicator or measured similarity threshold, with false-positive/false-negative analysis. Do not let the candidate select its own post-revocation probes.

## Research targets (not achieved)

- 100% pre-revocation canary retrieval on valid positive controls.
- Zero post-revocation direct exact-match leakage in finite deterministic fixtures.
- Zero post-revocation semantic/nearest-neighbour/cross-modal leakage above preregistered thresholds in a larger independent cohort.
- Complete enumeration of all direct/transitive derived-memory references for each authored dependency DAG.
- 100% purge-witness coverage for the declared storage scope, with replica/backup timing explicitly stated.
- Zero false-positive deletion on unrelated retention controls.
- No "storage erased" claim based on candidate self-report or black-box probes alone.

## Claim ceiling

A v0 pass validates fixture and evaluator mechanics, not a production memory system, a legal erasure guarantee, cryptographic deletion or physical storage sanitization. A black-box test can only establish non-observation under tested probes. A storage-erasure claim requires an explicit threat model, independent witnesses and coverage of caches, indexes, replicas, backups and derivatives within scope.

## Next increment

1. Add candidate submission and scorecard schemas.
2. Implement deterministic direct/alias matching, probe-stage checks, dependency-DAG purge coverage and unrelated-control retention scoring.
3. Keep noninterference and physical storage erasure as separate report dimensions.
4. Expand from public synthetic mechanics to evaluator-generated hidden canaries and independently controlled probes after candidate freeze.