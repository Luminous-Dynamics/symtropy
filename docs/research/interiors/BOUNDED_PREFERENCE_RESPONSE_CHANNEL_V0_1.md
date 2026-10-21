# Bounded preference response channel v0.1

**Status:** synthetic exact finite-model robustness prototype; it requires formatting, tests, strict Clippy and benchmark execution on the exact branch head.

## Problem and paradigm shift

The exact resident preference probe assumes each answer agrees with one latent utility model. Real pairwise feedback may be uncertain, context-sensitive, noisy, indifferent, or outside the supplied model set. A query that perfectly partitions noiseless models can lose its value under even a small response ambiguity.

This layer models a response as a set of possible observations conditioned on each model-predicted outcome. It is deliberately not Bayesian: response masks contain admissible possibilities, not frequencies or probabilities. When an observation is compatible with several models, every one stays in the support. When no model/channel pair supports an actual observation, resolution returns a typed outside-channel state instead of forcing the nearest model.

## Exact contract

For each latent outcome—left preferred, right preferred, or indifferent—the caller declares a four-bit mask of possible recorded answers: left, right, indifferent, and unresolved/insufficient information. Every mask must be nonempty and use only those four bits. The exact channel is the identity mapping; the fully ambiguous channel permits every observed answer from every latent outcome.

For an observed answer, the compatible model set is all supplied scenarios whose latent outcome permits that answer. Those supports may overlap. The robust post-answer regret is the maximum minimax regret across every reachable observation; robust regret reduction is baseline regret minus that worst post-answer value. All arithmetic stays in the original integer utility-numerator units. No response probabilities, confidence intervals, inferred resident traits or hidden epsilon are added.

An unsupported real answer yields OutsideDeclaredChannel. The caller must preserve that state and expand/review its model/channel explicitly; the function does not map it to a nearest model.

## Negative controls and proof obligations

The module tests:
- exact-channel agreement with the v0.1 exact query report;
- monotonicity under widening the possible-answer sets;
- fully ambiguous responses yielding zero worst-case regret reduction;
- unsupported/unmodelled answers producing an explicit outside-channel state;
- admitted unresolved answers preserving every compatible model;
- held-out balanced weights producing an indifference absent from the trained model set and surfaced outside-channel;
- rejection of empty or out-of-range masks;
- report invariance under input permutations.

Small synthetic cases are designed so results can be hand-checked. The existing exact selector's independent reference tests remain mandatory; this layer is not a replacement for them.

## Evaluation and benchmark matrix

The benchmark compares the exact public probe to the bounded-response robustness report on identical frozen requests:

| Alternatives | Models | Criteria | Pairwise questions |
|---:|---:|---:|---:|
| 8 | 8 | 4 | 28 |
| 16 | 32 | 8 | 120 |
| 32 | 64 | 8 | 496 |
| 48 | 128 | 16 | 1,128 |

Input generation is outside the timed region. Report p50/p95/max, repetitions, and query counts; every timed result is compared against an expected report for deterministic replay. These values are local synthetic kernel-level timings, not service-level latency guarantees. Operational scaling also needs memory and concurrency measurements.

## Research comparisons

- Huber et al. (2025), Bayesian Preference Elicitation for Decision Support in Multi-Objective Optimization, uses pairwise comparisons and a Bayesian utility model; this module intentionally offers an adversarial set-valued baseline rather than a probability model: https://doi.org/10.1002/mcda.70019.
- The 2026 survey on preference-guided multi-objective optimization covers approaches that use explicit and implicit preferences to steer multi-objective search: https://doi.org/10.1007/s10472-026-10012-6.
- Function2Scene (2026) evaluates/refines indoor layouts against functional constraints and reports human pairwise layout preferences; such preference rates are not physical feasibility rates or regret bounds: https://arxiv.org/abs/2605.30819.
- SceneEval (WACV 2026) separately evaluates explicit scene requirements and implicit plausibility including collisions and navigability using SceneEval-500: https://3dlg-hcvc.github.io/SceneEval/.

The intended comparison is complementary: hard-constraint feasibility, functional-layout fidelity, preference-query value, robustness to answer ambiguity and real user outcomes must remain separate metrics. No SOTA superiority is claimed.

## Claim ceiling

This is an exact conditional analysis of a finite candidate/model set and caller-declared response channel. It does not establish model-set adequacy, frequency of real answer noise, human preference accuracy, accessibility, construction-code compliance, physical safety, or improved residential outcomes. A human evaluation must be consent-based and report opt-out/abstention, consistency, time, cognitive load, decision regret under a fixed question budget, and comparison to random/fixed-order and reproducible Bayesian baselines.
