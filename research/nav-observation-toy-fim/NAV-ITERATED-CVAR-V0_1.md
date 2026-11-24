# Exact two-level Iterated CVaR for temporal navigation v0.1

**Status:** executable finite risk-evaluation primitive. Synthetic distributions only. This is not a policy optimizer, physical risk estimate, calibrated belief, or navigation authority mechanism.

## Research basis

Pariente & Indelman, **Online Risk-Averse Planning in POMDPs Using Iterated CVaR Value Function** (28 January 2026): https://arxiv.org/abs/2601.20554

The paper defines an iterated conditional risk recursion in which the current value is the immediate cost plus the CVaR of the next-belief value distribution. It develops policy-evaluation and online-planning algorithms for ICVaR. The risk parameter in that work is a tail-mass parameter: alpha=1 recovers expectation, while smaller alpha is more risk-averse.

This is distinct from:
- static CVaR over the complete mission loss distribution;
- immediate belief-cost CVaR over hidden states in the current belief;
- the outer worst-case operation over epistemically uncertain persistent mission models.

## Implemented two-level operator

For one fixed complete mission model, each observable first-stage branch y has:
- a branch probability mass p(y);
- a conditional continuation-loss distribution L_y.

Given tail mass alpha, the exact two-level operator is:

\[
\rho^{\mathrm{ICVaR}}_{\alpha}
=
\operatorname{CVaR}^{\mathrm{root}}_{\alpha}
\left(
\left\{
\operatorname{CVaR}^{\mathrm{continuation}}_{\alpha}(L_y)
\right\}_{y}
\right).
\]

The implementation:
1. computes exact conditional CVaR for each observable branch;
2. treats those rational values as outcomes weighted by branch probability;
3. computes exact root CVaR over the branch values;
4. retains exact rational arithmetic, including fractional atoms at both quantile boundaries.

The API takes tail mass in PPM:
- 50,000 means the worst 5% at each recursion level;
- 100,000 means the worst 10%;
- 1,000,000 recovers expectation at each level.

This differs from the mission-path cvar API, which takes confidence alpha (e.g. 950,000 for CVaR95). Conversion is explicit.

## Persistent-model semantics

The API accepts a separate branch tree for every complete mission model. It requires the same root acquisition action, branch-to-continuation-action mapping, and terminal decision map across every model, while allowing each model to assign its own branch probabilities and conditional loss distributions. It computes the two-level ICVaR independently per model, then takes:

\[
\max_{m\in\mathcal M} \rho^{\mathrm{ICVaR}}_{\alpha}(\pi,m).
\]

No probabilities or branch values are pooled across epistemic models. Model identity is fixed throughout each model's tree. The outer maximum is applied only after each model has been evaluated. The evaluator also checks that every model uses the same observable branch-to-continuation-action mapping; model-specific action choices for the same observable branch are rejected.

This is an evaluation primitive for a fixed contingent policy / conditional continuation family. It does not yet solve the full non-rectangular robust Bellman problem or determine how the plausible-model set should be updated after an observation.

## Exact relationship to the static-CVaR replanning counterexample

The R03 fixture from NAV-STATIC-CVAR-TIME-CONSISTENCY-V0_1.md is used again.

- Root branch L has 99% probability; its conditional continuation loss is 300 with mass 4/99 and zero otherwise.
- Root branch H has 1% probability.
- In H, continuation A has 5% loss 200 and 95% loss 150.
- In H, continuation B has constant loss 180.

Conditional values:

| Branch | Continuation | Conditional CVaR, worst 5% |
|---|---|---:|
| L | Shared continuation | 8000/33 ≈ 242.424 |
| H | A | 200 |
| H | B | 180 |

Static full-mission CVaR95:
- root A = 541/2 = 270.5;
- root B = 276.

The root two-level ICVaR value is 8000/33 under either continuation because the 99%-probability L branch alone occupies the root's worst 5% of branch values. The H branch is outside that root tail. Nevertheless, its conditional value is lower under B (180 vs 200).

This is an important distinction: evaluating the root nested scalar does not by itself select an action in every off-tail branch. A full ICVaR Bellman planner evaluates and optimizes each reachable continuation locally, then propagates those values upward. The present code verifies the risk recursion; it does not yet implement that Bellman policy optimizer.

## Exact implementation and tests

The Rust module provides:
- iterated_cvar for one fixed mission model;
- worst_case_iterated_cvar for independent per-model evaluation followed by an outer maximum;
- exact CVaR over rational-valued conditional outcomes;
- deterministic model / branch ordering and identity validation;
- shared root-action, branch-to-continuation-action, and terminal-decision signature checks across persistent models;
- checked arithmetic and explicit tail-mass validation.

Tests cover:
- exact conditional and root CVaR values;
- the R03 static-versus-conditional counterexample;
- risk-neutral alpha=1 expectation recovery;
- independent persistent-model values and outer maximum;
- malformed branch/model identities and invalid tail mass.

## What is not implemented

- A full ICVaR Bellman policy optimizer over the navigation sensing tree.
- Stage-specific acquisition, movement, or operational costs and discounting.
- A robust set-valued belief update for hidden persistent model identity.
- An exact policy-tree bridge from mission_path_loss_distributions to conditional continuation branches.
- Time-uniform, episode-level calibration of the branch distributions.
- Physical transition-model estimation, dataset admission or field trials.

The next major increment should connect the current observable policy tree to conditional branch distributions, then compare static path-CVaR, ICVaR and immediate belief-CVaR on the same finite model set. The robust optimizer must retain one observable policy for all candidate persistent models and must not silently introduce model switching between branches.

## Claim boundary

This is an exact two-level risk evaluator over supplied finite integer distributions. It does not establish a physical navigation risk guarantee, transition-set coverage, PHMI, safety certification or operational authority.
