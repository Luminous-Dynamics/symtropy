# Long-Horizon Evaluator Notes

The long-horizon evaluator is implemented in the shared benchmark tooling directory:

- \`tools/av-modality-necessity-evaluator/long_horizon_evaluate.py\`
- \`tools/av-modality-necessity-evaluator/test_long_horizon_evaluate.py\`

Run from repository root:

    python3 tools/av-modality-necessity-evaluator/long_horizon_evaluate.py \
      --workload research/av-long-horizon-evidence-v0/fixtures/agent/workload-v1.json \
      --truth research/av-long-horizon-evidence-v0/fixtures/evaluator-only/truth-v1.json \
      --submission research/av-long-horizon-evidence-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/av-long-horizon-reference-scorecard.json

The 12-case reference policy is expected to produce 11/12 answer correctness, 12/12 correct decisions (including correct abstention on the absent-event case and both as-known-before-change checkpoints), 11/12 coverage, 1.0 selective accuracy, 1.0 citation precision/recall, 12/12 complete provenance, zero false memories, zero evidence violations and zero deadline misses.

These are authored synthetic known-answer checks. They do not measure memory retention, token usage, retrieval latency, energy or real model capability. Resource telemetry is intentionally absent, not zero.

The evaluator checks query-time cutoffs, source availability, duplicate observation identity, correction/supersession, revocation, common-cause dependence, cross-clock claims, exact answer and source citation. A citation to a future, superseded or revoked observation is not admissible provenance.

The expected summary is an oracle for this finite evaluator fixture, not a blind test set.

## Evidence-qualified answer and lineage constraints

The scorecard reports `evidence_qualified_answer_correct_count` and `evidence_qualified_answer_accuracy` in addition to raw answer accuracy. A prediction qualifies only when it is answer-correct, source-complete and has no evidence-contract violations. A correct guess without required source lineage is not a qualified evidence retrieval.

Replay identity is fail-closed: repeated delivery may reuse an observation reference only when modalities, content digest, capture tick, source dependency group, clock-domain ID and generation remain identical. Event availability order must be monotonic. Derived memory items are checked for known source references, parent existence, causal creation order and cycles before revocation propagation.

Purge receipt records are constrained to `candidate_self_reported`; they validate a claimed purge workflow only. They do not prove deletion from physical storage, indexes, snapshots, backups, provider-side retention or external processors. A future blind qualification must add independently attested storage/provider receipts and post-revocation residual-query challenges.


## As-known-before-change temporal checks

The fixture now includes two additional pre-event checkpoints:
- lh-010-correction-before-arrival: query tick 300 precedes replacement observation tick 600 and correction tick 601. The old observation remains the valid current state at the query time.
- lh-011-revocation-before-effect: query tick 1200 precedes revocation tick 1800. The private observation and its then-valid derived summary remain eligible at the earlier query. The original post-revocation case separately verifies that the same data becomes ineligible after tick 1800.

Do not filter future control events out of the frozen evaluator workload. Instead, evaluate them as-of the query cutoff. The online candidate must receive only the causal prefix; the hidden evaluator may retain the full later timeline. The workflow checks that all observations required by each query were already available at that query tick, while allowing subsequent correction/revocation events to remain in the stored replay.
