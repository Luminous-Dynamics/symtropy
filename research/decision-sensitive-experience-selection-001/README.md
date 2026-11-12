# DECISION-SENSITIVE-EXPERIENCE-SELECTION-001

**Status:** deterministic Rust selector reference; no simulator or training integration  
**Canonical issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4568  
**Parent benchmark:** https://github.com/Luminous-Dynamics/symtropy/issues/4550

## Research question

Can an embodied driving learner allocate practice and human intervention more efficiently by selecting experiences that are likely to change an important decision before its deadline, rather than simply selecting rare or surprising examples?

This crate implements the ranking mechanics over finite, typed, already-qualified candidate records. It compares simple baselines with two SAM-guided ranking arms.

## Selection strategies

| Strategy | Ordering |
|---|---|
| Uniform cyclic | Stable ID order rotated after a supplied cursor ID |
| Family rarity | Lowest prior family exposure count first |
| Entropy | Highest declared uncertainty first |
| Prediction error | Highest declared prediction error first |
| Decision margin | Smallest normalized action-decision margin first |
| SAM decision value | Highest expected decision-regret reduction per acquisition cost |
| SAM recoverability | Highest option fragility, then lowest recovery depth, then least residual decision time |

Each strategy has a deterministic stable-ID tie-break. There is no learned/randomized sampler in this crate.

## Decision-sensitive value calculation

The selector consumes two independently qualified SAM estimates:
- current expected decision loss;
- expected decision loss after the requested information/intervention.

Only a strictly positive reduction is rankable:

`gain = current_expected_loss - expected_post_query_loss`

The cost-normalized rank is the exact rational quantity `gain / acquisition_cost`. Comparisons use 128-bit cross multiplication, not floating point or rounded scores. The unit of the loss numerator is a caller-declared micro-loss unit; cost is a caller-declared positive micro-cost unit. These are dimensioned values, not a probability or safety score.

This is an ordering primitive, **not an estimator of true value of information**. The upstream SAM evidence must supply calibrated expected losses, identity-bound provenance and the correct cost model. This selector cannot validate those estimates.

## Deadline and evidence gates

A SAM query is eligible only when:
- scenario, observation and SAM evidence are admitted;
- policy-visible observation evidence is available by the declared selection tick and is not older than the configured maximum age;
- SAM checkpoint, context and observation digests match the candidate;
- query latency and actuator/recovery reserve fit before the decision deadline;
- post-query expected loss is strictly lower than current loss;
- acquisition cost is present, positive and within the configured maximum;
- all arithmetic is checked and deterministic.

The deadline condition is:

`selection_tick + expected_query_latency + actuation_reserve <= decision_deadline`

The selector does not return a late query as the top sample. It records a typed rejection reason. A deadline miss must not cause the controller to delay or suspend its independent runtime assurance/fallback path.

## Recoverability ordering

This arm uses a lexicographic tuple instead of an opaque weighted sum:
1. option fragility (descending);
2. qualified recovery depth (ascending);
3. residual decision time (ascending);
4. candidate ID (ascending).

These outputs remain separate. The ordering does not itself decide whether a control action is safe or admissible.

## Inputs and receipt

Every candidate carries exact candidate, scenario, observation, checkpoint and context digests. The selection report carries the strategy, selection tick, considered candidate IDs, accepted/rejected dispositions, source digests, rank evidence and selected prefix.

Findings are deterministic and candidate ordering is invariant to the input vector order. Duplicate candidate IDs and mixed selection ticks reject the entire campaign rather than silently selecting one.

## Run

From this directory:

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

The repository workflow uses Rust 1.96.0.

## Claim ceiling

Passing tests establish only deterministic ranking and fail-closed eligibility over finite supplied records. They do not establish:
- that SAM's expected-loss or recoverability estimates are correct;
- that the selected information is causally useful in the simulator;
- that a policy learns faster or transfers better;
- that a world model is accurate;
- that a vehicle is safe, roadworthy or authorized for public-road operation.

Empirical superiority requires the matched arms, sealed scenario families, sample-efficiency measurements and independent recoverability gates in #4550.
