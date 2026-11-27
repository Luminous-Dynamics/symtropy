# AV Modality Necessity Evaluator (offline reference)

This directory contains a deterministic, standard-library-only Python evaluator for the synthetic AV modality-necessity benchmark. It is research/benchmark tooling, not a production service, sensor-fusion component, sensor driver, cryptographic verifier, or safety function. Runtime integration remains with Symthaea #12837 and the existing Symtropy AV observation/replay owners.

## Run

From the repository root:

    python3 tools/av-modality-necessity-evaluator/evaluate.py \
      --workload research/av-modality-necessity-v0/fixtures/agent/workload-v1.json \
      --truth research/av-modality-necessity-v0/fixtures/evaluator-only/modality-truth-v1.json \
      --submission research/av-modality-necessity-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/av-modality-scorecard.json

The evaluator-only truth file must remain outside the candidate policy process, filesystem namespace and credentials. Only the public workload is candidate input; predictions are exported separately. CI evaluates the authored reference submission, not a model run.

## Submission and scoring contracts

The policy-submission schema defines per-episode answer, abstention, confidence, candidate-declared modality use, exact cited observations, independence/association claims and evidence/response ticks. Optional measured fields are reserved for latency, audio bytes, video frames, generated tokens and energy.

The evaluator requires one prediction per workload episode and checks episode-scoped citations, observation availability at the declared evidence cutoff, query/response time ordering, modality-to-citation consistency, transcript unavailability, necessary modality declaration, missing-audio abstention, common-cause non-independence and no forced association under conflict or misalignment.

The scorecard reports unweighted per-dimension metrics: decision accuracy, coverage, selective accuracy, answered-only Brier score, per-necessity strata, paired counterfactual discrimination, evidence compliance, hard violations and deadline misses. There is no composite leaderboard score; hard violations cannot be traded against task accuracy.

## Claim limits

The reference policy's 13/13 decisions and zero hard violations validate finite evaluator semantics only. They are not model results, an external benchmark baseline, or an SOTA comparison.

Candidate-declared modality use plus input-reference citations is an auditable declaration, not proof of internal model attention or causal feature use. External causal claims require actual admissible PCM/RGB bytes, matched modality ablations, frozen content hashes, exact model/runtime/hardware revisions and multiple seeds. The current episodes contain synthetic metadata and placeholder commitments, not raw sensor media.

Simulation ticks are not milliseconds. Latency, energy, token, frame and byte budgets require measured run telemetry and are not inferred by this evaluator.