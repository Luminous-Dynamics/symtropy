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

## Candidate evaluator tranche (v1)

Added:
- schemas/submission-v1.schema.json: probe responses, confidence, cited source references, retrieved derived-memory references, candidate purge receipts and declared policy-ineligibility references.
- schemas/scorecard-v1.schema.json: pre-revocation canary retrieval, post-revocation leakage, source/derived-reference reuse, unrelated-control retention, purge-receipt coverage and policy-ineligibility reporting.
- fixtures/agent/reference-submission-v1.json: finite synthetic reference policy.
- fixtures/expected/reference-scorecard-v1.json: known-answer totals.
- tools/av-modality-necessity-evaluator/evaluate_revocation.py: deterministic normalized canary/alias evaluator.
- tools/av-modality-necessity-evaluator/test_evaluate_revocation.py: adversarial tests for direct leakage, alias leakage, source/derived retrieval, missing purge receipts, missing pre-probe recall, overbroad deletion, false storage-erasure claims and malformed probe sets.

Run:

    python3 tools/av-modality-necessity-evaluator/evaluate_revocation.py \
      --workload research/av-memory-revocation-challenge-v0/fixtures/agent/workload-v1.json \
      --truth research/av-memory-revocation-challenge-v0/fixtures/evaluator-only/truth-v1.json \
      --submission research/av-memory-revocation-challenge-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/av-memory-revocation-scorecard.json

The eight-episode reference submission yields eight pre-revocation canary hits, zero post-revocation canary leaks across nine probes, zero source/derived-reference reuses, one retained unrelated control fact, ten in-scope purge receipts across ten in-scope derived objects and complete candidate-reported policy-ineligibility coverage. These numbers qualify the finite evaluator semantics only. The purge receipts are candidate self-reports, not independent storage witnesses.

### Two separate scope axes

A critical refinement in v1 distinguishes:
- **Storage purge scope:** which derived objects the declared revocation profile requires to be physically purged or cryptographically erased.
- **Policy-ineligibility scope:** every derived object that depends on the revoked source, regardless of whether its storage class is included in the purge profile.

The second episode now contains an embedding outside the storage purge scope. It may remain persisted under that profile, but the candidate must still report it as policy-ineligible and must never retrieve it after revocation. The evaluator counts 11 total derived objects and 10 in-scope purge objects. It separately checks receipt coverage for the 10 in-scope objects and policy-ineligibility reporting for the source plus all 11 derivatives.

This avoids treating "not physically purged by this storage profile" as "still allowed to answer from it". A storage receipt for an out-of-scope item is reported as a scope violation rather than inflating required purge coverage.

### Exact interpretation boundary

The evaluator uses normalized exact canary and alias matching. It does not establish semantic non-recall across arbitrary paraphrase, latent weights, unknown replicas or untested queries. It never reports physical storage erasure as verified, even when the candidate claims completion. Attestation labels in the candidate's receipt are statements to evaluate, not independently authenticated witnesses.

The workflow validates the workload/submission/scorecard schemas, causal probe chronology, dependency order, exact candidate probe coverage, runs the adversarial suite, generates the reference scorecard and compares it with frozen expected totals. Exact-head GitHub Actions remains the CI authority.
