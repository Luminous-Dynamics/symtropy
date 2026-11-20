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

The nine-case reference policy is expected to produce 8/9 answer correctness, 9/9 correct decisions (including correct abstention on the absent-event case), 8/9 coverage, 1.0 selective accuracy, 1.0 citation precision/recall, 9/9 complete provenance, zero false memories, zero evidence violations and zero deadline misses.

These are authored synthetic known-answer checks. They do not measure memory retention, token usage, retrieval latency, energy or real model capability. Resource telemetry is intentionally absent, not zero.

The evaluator checks query-time cutoffs, source availability, duplicate observation identity, correction/supersession, revocation, common-cause dependence, cross-clock claims, exact answer and source citation. A citation to a future, superseded or revoked observation is not admissible provenance.

The expected summary is an oracle for this finite evaluator fixture, not a blind test set.