# AV Evidence Certificate v0

**Issue:** Symtropy #6856  
**Programme:** #6806  
**Composes:** #6183 audiovisual truth/observation planes; #6357/#6359 long-horizon source-linked memory; #6753/#6555 EVSI and multi-round acquisition; #6497 response/silence; #6754 custody; Symthaea #13179 candidate trace export.

## Objective

Score whether each candidate answer or abstention is supported by the exact observations it actually received before its decision cutoff. Keep answer accuracy separate from evidence-grounded answer accuracy.

## Files

- `tools/av-evidence-certificate/evaluate_evidence_certificate.py`: standard-library evaluator and CLI.
- `tools/av-evidence-certificate/test_evidence_certificate.py`: deterministic and adversarial contract tests.
- `tools/av-evidence-certificate/counterfactual_evidence_suite.py`: controlled evidence-removal / distractor-addition robustness harness that reuses the same claim scorer.
- `tools/av-evidence-certificate/test_counterfactual_evidence_suite.py`: counterfactual invariance, necessary-evidence withdrawal and adversarial tests.
- `schemas/workload-v1.schema.json`: candidate-visible workload and delivered observation catalog.
- `schemas/submission-v1.schema.json`: candidate claims and observation references.
- `schemas/truth-v1.schema.json`: evaluator-only acceptable answers, minimal sufficient evidence sets, temporal spans and spatial boxes.
- `schemas/counterfactual-evidence-suite-v1.schema.json`: baseline plus independently generated evidence-intervention runs.
- `fixtures/agent/`: candidate-visible workload and submission.
- `fixtures/evaluator-only/`: hidden truth for the synthetic known-answer fixture.
- `fixtures/expected/`: frozen expected scorecard.

## Evidence and custody boundary

The workload contains only delivered observations, artifact hashes, modality, capture interval, clock domain/generation and delivery/capture-completion ticks. The submission must bind to the exact workload transcript head.

The truth file is evaluator-only. It must never be mounted or passed to the candidate. It contains acceptable answer values, alternative minimal sufficient evidence sets, supporting temporal intervals, and spatial boxes only where the benchmark annotation supports them.

The evaluator rejects:
- unknown/unobserved or post-cutoff references;
- observations delivered or captured after the decision cutoff;
- evidence intervals outside the delivered observation;
- transcript-head mismatches;
- missing/extra candidate claims or duplicate IDs;
- spatial boxes on audio observations or invalid normalized geometry.

Temporal IoU is computed per observation reference, so the evaluator does not compare timestamps from different clock domains as though they shared a clock. It does not perform clock transformations or infer physical localization.

## Scorecard

Every episode reports separate:
- answer accuracy;
- grounded answer accuracy (correct answer + sufficient source evidence + temporal/spatial thresholds where annotated);
- unsupported claim rate;
- citation precision, recall and F1;
- temporal IoU;
- spatial box IoU;
- correct abstentions;
- unnecessary citations relative to the chosen minimal sufficient evidence set.

## Counterfactual evidence robustness

The suite runner consumes a baseline and separate candidate submissions generated after each intervention. It never fabricates the model's counterfactual response.

Supported interventions:
- `REMOVE_IRRELEVANT`: remove only observations outside every minimal sufficient evidence set for the target claim. The answer/abstention must remain unchanged and the claim must remain grounded.
- `REMOVE_ALL_SUFFICIENT`: remove at least one source from every alternative minimal sufficient evidence set. The candidate must abstain with no answer or citations rather than keep asserting an unsupported claim.
- `ADD_IRRELEVANT`: add delivered distractor observations outside every sufficient evidence set. The candidate must preserve its grounded answer and not cite the distractors.

The variant transcript head must differ from the baseline. All retained source observations must be byte-for-byte metadata-identical; only the declared observation addition/removal is allowed. Evaluator truth continues to reference the immutable baseline source catalog while candidate evidence is validated against the changed candidate-visible catalog.

Run:

    python3 -m unittest discover -s tools/av-evidence-certificate -p 'test_*.py' -v
    python3 tools/av-evidence-certificate/counterfactual_evidence_suite.py --input counterfactual-suite.json

Input schema: `schemas/counterfactual-evidence-suite-v1.schema.json`.

The counterfactual rates are policy robustness metrics, not causal proofs. Their interpretation depends on the evaluator's minimal-sufficient-evidence annotations and the intervention not changing unrelated task context.

Unannotated temporal/spatial axes are reported as null/NOT_EVALUATED, never silently as zero. The v0 uses exact structured answer values; free-form semantic matching is outside its scope.

## Running

    python3 -m unittest discover -s tools/av-evidence-certificate -p 'test_*.py' -v
    python3 tools/av-evidence-certificate/evaluate_evidence_certificate.py \
      --workload research/av-evidence-certificate-v0/fixtures/agent/workload-v1.json \
      --truth research/av-evidence-certificate-v0/fixtures/evaluator-only/truth-v1.json \
      --submission research/av-evidence-certificate-v0/fixtures/agent/submission-v1.json \
      --output /tmp/av-evidence-scorecard.json

The synthetic fixture is a mechanics test, not a blind benchmark or model result. Real media must be rights-qualified, source/session split and independently custodied under #6754 for confirmatory use.

## Explicit v0 limits

- One episode at a time; no streaming candidate runtime or automatic sensor acquisition.
- Exact structured answer values only; no semantic LLM judge.
- Temporal overlap is measured within observation IDs; no cross-clock alignment service is introduced.
- Spatial support is 2D normalized bounding-box IoU only; no inferred 3D position, physical geometry or acoustic bearing.
- Minimal evidence groups are declared by the evaluator; this does not prove causal sufficiency beyond the annotation.
- No measured minimum-FPS curve yet; v0 provides the certificate scorecard onto which fixed-FPS and adaptive acquisition runs can be joined later.
- No independent blind result or SOTA claim.

## Claim boundary

A passing evaluator contract establishes only schema, cutoff, citation and overlap-scoring mechanics. It does not prove the truth of the annotations, causality, human explanation quality or universal evidence-grounded reasoning.
