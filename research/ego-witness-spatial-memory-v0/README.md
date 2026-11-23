# EgoWitness Spatial Memory Adapter v0

**Issue:** #6574  
**Existing long-horizon evidence owner:** Symtropy #6357 / PR #6359  
**Existing AV replay and memory owners:** Symtropy #6201 / #6249, Symthaea #10644 / #9075 / #9177 / #12837  
**Related benchmark owner:** Symtropy #4414 (RoboCasa-Mem, a distinct mobile-manipulation benchmark)

## Research motivation

EgoWitness was submitted to arXiv on 19 November 2026. The paper reports 8,453 human-verified questions across about 324 hours of egocentric recordings; seven abilities include instance identity, last-seen attribution and room-visit reasoning. Its source includes more than one million reviewed masks across 34 registered landmarks and 21 movable-object labels. Direct open-weight VLM results top out at 46.97%, while the paper reports 72.65% for its trained EgoClue 4B controller on 1,320 held-out week-long questions and 80.56% on a separate 787-question cross-environment split.

Paper: https://arxiv.org/abs/2611.25849

These are different evaluation slices and methods. Do not combine 72.65% and 80.56% into one score, and do not directly compare the 11-case synthetic event-memory benchmark to EgoWitness.

## Purpose and paradigm

This first tranche tests a specific gap: **category recognition is not instance identity**. A model may recognize that two views depict a red mug yet fail to know whether they are the same physical mug. The same challenge applies to multiple wearers, cross-day changes in viewpoint/lighting, and room anchors observed from different camera positions.

The adapter separates:
- category and local track IDs visible to the candidate;
- hidden stable physical-instance identity;
- visible wearer and environment context;
- visible room observation anchors;
- hidden canonical room identity;
- exact source observation references and query-time eligibility.

The candidate must not receive evaluator-only stable IDs. This is metadata conformance only; there are no actual images, masks, biometric features or calibrated maps.

## Data plane

- fixtures/agent/workload-v1.json: four synthetic candidate-visible scenarios.
- fixtures/evaluator-only/truth-v1.json: stable instance identity, canonical room identity, pairwise truth, last-seen observation/wearer and expected answers.
- fixtures/agent/reference-submission-v1.json: authored synthetic reference policy.
- fixtures/expected/reference-scorecard-v1.json: frozen known-answer scorecard.
- schemas/workload-v1.schema.json, schemas/submission-v1.schema.json and schemas/scorecard-v1.schema.json: versioned contracts.
- tools/ego-witness-spatial-evaluator/evaluate.py: deterministic offline evaluator.
- tools/ego-witness-spatial-evaluator/test_evaluate.py: adversarial test suite.

## Synthetic scenario families

1. Cross-day re-identification: same mug across days and wearers, with a same-category lookalike.
2. Last-seen observer attribution: exact same bottle observed by three wearers; a later same-category lookalike must not win.
3. Room-anchor identity: two camera viewpoints resolve to the same physical room; a kitchen anchor remains distinct.
4. Same-category distinct instances: similar red toolboxes in the same workshop remain distinct, while a later view of the first toolbox must re-identify it.

## Run

    python3 tools/ego-witness-spatial-evaluator/evaluate.py \
      --workload research/ego-witness-spatial-memory-v0/fixtures/agent/workload-v1.json \
      --truth research/ego-witness-spatial-memory-v0/fixtures/evaluator-only/truth-v1.json \
      --submission research/ego-witness-spatial-memory-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/ego-witness-spatial-scorecard.json

The reference policy is expected to achieve 4/4 synthetic answers, 12/12 correct instance pair decisions, 3/3 room pair decisions, 1/1 last-seen observer attribution, complete citations and zero hard violations. This validates evaluator semantics only; it is not a model score or EgoWitness reproduction.

## Scorecard

Keep independent:
- exact answer accuracy and evidence-qualified answer accuracy;
- pairwise instance precision/recall, false merges and false splits;
- room identity/topology relation accuracy;
- last-seen event and wearer attribution;
- source citation precision/recall and provenance completeness;
- evidence cutoff and response deadline violations.

Do not turn a false merge into a harmless average: a same-category lookalike incorrectly identified as the same physical instance is a first-class false merge. Likewise, fragmenting one object across views is a false split.

## Rights and external reproduction

The official EgoWitness release, code revision, annotation licence, underlying recording terms and derivative mask rights have not yet been resolved in this adapter. The paper is public but that does not grant redistribution rights for recordings or annotations. Do not download, mirror, commit or train on the dataset until the official immutable release manifest and rights are audited per artefact.

When permitted, reproduce the official EgoWitness test protocol and compare direct VLM, recent-window, summary, retrieval, EgoClue-style agent and Symthaea memory paths under matched frame, token, tool, model and environment budgets. Preserve the upstream week-long and cross-environment splits separately. Add our provenance/time/correction/revocation diagnostics as complementary metrics rather than claiming they replace the upstream score.

## Claim ceiling

This is an adapter and evaluator, not a production spatial memory, SLAM system, identity recognizer or room mapper. Synthetic passing tests do not demonstrate physical instance recognition, multi-wearer attribution, licensed data reproduction or SOTA superiority.