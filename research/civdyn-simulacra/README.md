# CIVDYN × SimulacraBench adapter (synthetic-only research)

This directory contains a schema-driven probabilistic categorical survey-completion baseline. It is an evaluation adapter, not a generator of synthetic people and not a production demographic model.

## Official benchmark and pinned public starter kit

- Competition: https://simulacrabench.org/
- Official public starter kit: https://github.com/SituatedEvals/public
- Pinned upstream commit: `aa03186f8f083c7785a1a566c802c2077925e265` (26 October 2026)
- Official licence: MIT (upstream)
- Public starter kit includes schemas, invented sandbox generator, grader and baselines; it does **not** include protected microdata.

The adapter is developed only against the official public synthetic sandbox. It does not access, infer, reconstruct, scrape, or attempt to identify hidden respondents. It must not be used to produce person-level decisions, profiling or high-stakes decisions.

## Task contract

The official `predict(frame, schema)` contract expects one probability vector per blank PREDICT cell, in respondent-major and schema item order. A gated item includes the schema's `NA_GATED` sentinel as its final option. The response vector must be finite, nonnegative, correctly sized and normalized.

The official grader uses a proper categorical log score and normalizes skill against the smoothed crowd-marginal reference. The public baseline is expected to score zero by construction. This local CI checks that invariant against the official grader; the CIVDYN adapter is reported but is not required to beat the baseline on every synthetic seed.

## Adapter

`submission/main.py` implements a deliberately modest conditional model:
- Per-item regularized multinomial logistic regression over schema-defined GIVEN features.
- Optional gate-parent feature for gated PREDICT items.
- If a gate parent is itself held out, the child prediction marginalizes over the parent's predicted option distribution.
- A smoothed per-item marginal backs off for sparse, single-class or unavailable conditional fits.
- The model never uses EXCLUDE fields or identifiers as predictors.
- It learns the skip-sentinel response from visible sandbox rows; it does not blindly impose every `observed_if` relation as a hard rule. This matters because the UNHCR public schema documents inferred gates with imperfect agreement to recorded values.

This is a baseline for research, not a claim of optimality. Next comparisons should include conditional frequency tables, hierarchical/multitask categorical models, calibrated tree ensembles and low-rank latent-factor models, all evaluated by paired log score and calibration against the same public synthetic split.

## Reproduce locally (from a checkout)

Use the official starter kit at the pinned commit and install its requirements plus scikit-learn and pytest:

```bash
python make_sandbox.py --schema data/sample.json --seed 0 --out _sandbox/seed-0
python score.py --submission baseline/marginal_counts --data _sandbox/seed-0 \
  --schema data/sample.json --phase 1 --log /tmp/marginal-seed-0.json
python score.py --submission /path/to/research/civdyn-simulacra/submission \
  --data _sandbox/seed-0 --schema data/sample.json --phase 1 \
  --log /tmp/civdyn-seed-0.json
```

The repository CI checks three fixed synthetic seeds, verifies the crowd baseline skill is zero within numerical tolerance, and reports the adapter scores. It also runs schema smoke tests for the public UNHCR, UNICEF and World Bank instrument definitions. Those tests use invented categorical responses and validate only shape, ordering, sentinel width and probability normalization.

## Scientific claim ceiling

- Sandbox success is code/protocol conformance under the generator's invented distributions and dependencies.
- The sandbox does not establish that its dependencies match the protected survey populations.
- A higher synthetic log score does not establish empirical validity, representativeness, fairness, causal validity or transportability.
- The competition's overall skill is a ranking metric, not a complete CIVDYN model-adequacy certificate.
- No submission is uploaded and no competition registration is performed by this branch. Any external submission requires explicit owner approval and adherence to the official terms.
- No microdata or respondent-level prediction artifact is stored in this repository.
