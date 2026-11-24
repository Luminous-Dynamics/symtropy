# Mission-global minimax policy tree v0.1

**Status:** exact finite robust planning reference for issue #7276. The adversarial model is fixed for the whole mission. This is not the Bayesian mixture in PR #7273 and not a per-branch model-switching controller.

## Problem

There are two plausible persistent observation models. Each has its own initial distribution over a four-state physical latent variable: nominal/inside, nominal/outside, shared-fault/inside, shared-fault/outside. Each candidate action has model- and state-conditional likelihoods for observed-zero, observed-one, and no-observation.

One observable policy tree maps every observation history to the same action and terminal-decision rule under both models. The controller never receives the hidden model identity.

The mission-global objective is:
\[
\min_{\pi\in\Pi_{\rm observable}}\max_{m\in\{0,1\}}\mathbb E_m[L\mid\pi].
\]

For each candidate policy tree, first compute the complete risk under model 0 and the complete risk under model 1. Only then take the maximum. The model is not allowed to switch between branches.

## Rectangular diagnostic

For comparison, the evaluator also computes a distinct adversary that:
1. selects a fixed model for the first acquisition;
2. after each first-stage observation, may choose a different model for that branch's second-stage likelihoods;
3. maximizes the sum of branch losses, then maximizes over the root model.

This is a dynamically switchable model and is not the same uncertainty structure as one persistent mission model. The difference is reported as a diagnostic, not used to choose the mission-global policy.

## Exact finite enumerator

The Rust module navigation_mission_minimax_policy_tree.rs implements:
- two fixed models, each with its own PPB prior over the four physical states;
- up to four candidate actions, three outcomes, horizon two;
- every observable policy choice: root action, branch-specific stop/second action, and terminal decision for each second-stage outcome;
- action non-reuse and hard per-path latency/energy/bandwidth ceilings;
- exact PPB likelihood propagation and checked u128 loss arithmetic;
- one common policy tree under both fixed models;
- model-wise risk vector, worst-case model, global minimax risk, and branchwise-switching diagnostic;
- deterministic policy enumeration and tie-breaking;
- explicit stop policy and non-authority marker.

The output also reports how many policy trees were evaluated. This is a small reference oracle, not a scalable planner.

## Synthetic adversarial example

A first observation strongly indicates which of two fixed sensor models is active. Each branch then selects the follow-up sensor best for the model suggested by that observation. Under either fixed mission model, the resulting common observable policy has the same low expected terminal risk. If the model is allowed to switch after every first observation, it chooses the opposite model separately for each branch and raises the bound.

The regression test asserts a strict gap between the mission-global worst-model risk and this branchwise-switching diagnostic. The probabilities are constructed exactly; they are not empirical calibration results.

## Tests authored

Six tests cover the strict persistence-vs-switching gap, common policy under both models, per-path resource limits, malformed priors/likelihoods/duplicate IDs, no-feasible-action stop policy, and candidate-order invariance.

Exact-head CI remains the qualification gate.

## Limitations and follow-on

This is a finite two-model, binary-navigation, horizon-two Bayesian decision-loss evaluator with a shared-fault regime. It does not calibrate the model set, establish its coverage, handle more than two models, implement minimax regret yet, or provide a physical safety case.

The next robust additions should be:
- model-set expansion and model-set coverage calibration;
- exact minimax regret against model-aware oracles;
- sensitivity to probability/impact intervals;
- larger-horizon approximations benchmarked against this exact oracle;
- truth-swap process isolation and independent evaluator reconstruction.

## Literature

- Li, Kuhn & Sutter (2026), non-rectangular robust MDPs: https://doi.org/10.1137/24M1631250
- Kumar et al. (NeurIPS 2025), a special L1-normed non-rectangular class: https://proceedings.neurips.cc/paper_files/paper/2025/hash/372cb7805eaccb2b7eed641271a30eec-Abstract-Conference.html
- Osogami, robust partially observable MDPs: https://jmlr.csail.mit.edu/proceedings/papers/v37/osogami15.pdf
