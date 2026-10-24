# External validation plan: cooperation and public-goods dynamics

**Status:** research plan; no external dataset has been downloaded, ingested, or analyzed in this PR.  
**Purpose:** make the path from synthetic prototypes to empirically grounded social-dynamics claims explicit.

## Why this is the next scientific gate

The current water-service-shock model is useful for deterministic mechanics, intervention bookkeeping, sensitivity probes, and uncertainty reporting across synthetic seeds. It cannot establish that its behavioral rules correspond to real human cooperation. The next model-validity tier should be a separate benchmark where target behaviors are observed and competing models must predict held-out outcomes.

Public-goods experiments provide a manageable start because contribution trajectories, repeated interactions, information conditions, inequality, and payoff feedback are observed. They also expose an important failure mode: a model can match average contributions but miss the distribution of contributions.

## Candidate benchmark datasets

| Candidate | What it provides | Intended use | Gate before data use |
|---|---|---|---|
| Burton-Chellew & West (2021), *Payoff-based learning best explains the rate of decline in cooperation across 237 public-goods games* — Dryad DOI 10.5061/dryad.fn2z34tsv | Three companion packages: simulation analyses, a human black-box experiment, and comparative metadata for 237 published public-goods games. Record describes round-level experiment outputs and meta-analysis of contribution decline. | Primary candidate for studying whether payoff-based learning, persistence/trend baselines, and social-cooperation mechanisms predict contribution trajectories and the rate of decline across different games. | Inspect the record-specific licence and each package's readme/consent limitations; record the exact version, file digests, and permitted use. Do not copy source data into this repository until reuse terms are verified. |
| Pereda et al. (2019), *Large scale and information effects on cooperation in public good games* — Zenodo DOI 10.5281/zenodo.2590686 | Public-goods participant decisions with participant contribution, group-average contribution, round number, alive/status, and treatment fields in a 612 KB CSV record. | A separate external treatment/information generalization test, not a substitute for the primary holdout. | The Zenodo page visibly leaves its record-level license field blank. Confirm permission and terms before downloading into the project, preprocessing, or redistribution. |
| *Future of Society Public Goods Game Dataset* (2026) — Zenodo DOI 10.5281/zenodo.20174364 | 29,590 retained decisions from five profile-generation contexts, three multiplier branches, ten rounds, and five LLM decision models. Its description says five stochastic runs were conducted and no fixed random seed was used. | A separate LLM-agent population benchmark: compare decision distributions and the effect of generated profiles/model choice. This dataset is **not** human-behavior evidence and cannot validate a simulator of actual people. | Confirm the dataset's rights/licence and research status; document the non-fixed-seed design and quality exclusions. Do not treat agents as independent human participants or merge its scores into the human benchmark. |

Links:
- Dryad record and files: https://doi.org/10.5061/dryad.fn2z34tsv
- Pereda et al. data record: https://doi.org/10.5281/zenodo.2590686
- Future of Society LLM-agent data: https://doi.org/10.5281/zenodo.20174364

The Zenodo records above are discovery leads; their visible metadata does not clearly state a record-specific reuse licence. A dataset being downloadable or labelled “open” must not be treated as permission to rehost, redistribute, or train on it without checking the record terms.

## Proposed first empirical task

**Task:** predict held-out public-goods contribution trajectories and the rate of contribution decline from game-level conditions.

### Evaluation protocol

1. Before accessing held-out labels, preregister the primary prediction target, scoring rule, exclusions, and model candidates.
2. Build a manifest containing record DOI and version, source filename and SHA-256, schema version, variable definitions, missing-data treatment, consent/licence constraints, and transformation code commit.
3. Split by whole study/experiment, not by individual round rows. Repeated decisions from the same participant, group, study, or treatment session must not leak across train and test.
4. Fit behavioral parameters only on the training partition. Use validation groups for model selection and leave the final study-held-out partition untouched until choices are frozen.
5. Compare out-of-sample predictions for the full round trajectory, the rate of decline, and distributions whenever individual-level data are permitted.
6. Publish the baseline scores, model scores, calibration, per-study errors, uncertainty interval, exclusions, and all failed cases.

### Required baselines

- Persistence: the last observed mean contribution continues.
- Historical mean: training-set mean by round, without agent-level cognition.
- Linear trend: continuation of the estimated trend as a simple forecasting baseline.
- A formal payoff-based learning model, since the 237-game dataset is explicitly associated with testing this hypothesis.
- A conditional-cooperation/simple reciprocity model.
- Optional LLM-agent policies, evaluated as another model family under the same data boundary and scoring rule, not as a ground-truth source.

Do not compare a sophisticated model only with a straw-man baseline. Select the strongest eligible baseline using training/validation only.

### Metrics

- Primary aggregate trajectory score: MAE of contribution percentage per round, with the exact aggregation and weighting preregistered.
- Distributional score where microdata are available: CRPS or another declared proper scoring rule; also report quantiles and tail error. If a candidate dataset provides only study-level means, label the test aggregate-only.
- Trend target: absolute error in contribution-decline slope and accuracy for the sign of the decline interaction predeclared from the study design.
- Calibration: prediction interval coverage plus interval width.
- Robustness: per-study error distribution, sensitivity to study exclusions, group-size/treatment strata, and performance under held-out information conditions.
- Baseline-relative improvement: target at least 10% relative reduction in the primary held-out loss against the strongest eligible baseline, with a paired study-level 95% interval excluding no improvement. This is an aspirational go/no-go target, not a claimed result.

For clustered data, uncertainty must be resampled at the highest independent design unit available (usually study or experiment), not at the row level. A large number of individual rows from a few studies does not imply a large independent sample.

## Connection to the four projects

- **Symthaea:** candidate behavioral policies (e.g., payoff learning, reciprocity, belief updating) are mechanisms to compare, not axioms that should always be enabled.
- **Symtropy:** executes deterministic and stochastic trajectories, controls shared conditions, exports raw seed-level results, and reports model comparisons.
- **Mycelix:** later experiments can model commitments, evidence, and dispute rules as treatment interventions with auditable provenance; the present prototype does not yet implement commitments or dispute resolution.
- **Sol-Atlas:** later social-ecological tasks can bind cooperative behavior to actual resource capacity and geography. The current water shock is synthetic and has no spatial layer.

## Reproducibility and ethics gates

- No real dataset is bundled in this PR.
- Verify and record licences/terms before data ingress, and preserve original attribution.
- Use de-identified/aggregate targets when possible; do not ingest raw private messages or identifiable social graphs.
- Never use model outputs as an individual citizen reputation score or as sole evidence for consequential decisions.
- Freeze source, transforms, split, seeds, scoring, and versioned parameters before evaluating the test partition.
- Distinguish software correctness, formal-mechanism checks, synthetic intervention results, empirical prediction, and prospective intervention validation.

## Literature grounding

- Burton-Chellew & West's study frames cooperation decline as a differentiating prediction between payoff-based learning and competing explanations: https://doi.org/10.1038/s41562-021-01107-7
- Wunder, Suri & Watts report that a model could predict mean public-goods contributions while failing on the full bimodal contribution distribution, motivating distribution-first evaluation: https://www.microsoft.com/en-us/research/publication/empirical-agent-based-models-of-cooperation-in-public-goods-games/
- ICML 2026 calls for shared benchmarks, open data, and reproducible evaluation to close the validation gap in LLM social simulations: https://proceedings.mlr.press/v306/puelma-touzel26a.html

**Current status:** this plan is not an empirical result. The first data-ingestion task must begin with licence verification and a transparent schema/split audit; only then should a loader be implemented.
