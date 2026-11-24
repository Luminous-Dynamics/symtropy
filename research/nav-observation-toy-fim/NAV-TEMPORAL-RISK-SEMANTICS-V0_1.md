# Temporal navigation risk semantics: belief, trajectory, and persistent model v0.1

**Status:** research protocol. This document specifies comparator objectives and counterexamples; it is not an implemented dynamic-risk planner or a safety guarantee.

## Why this distinction matters

A partially observed navigation system has more than one kind of uncertainty:

- The current belief may contain a small amount of probability mass on a very dangerous hidden state.
- The next observation and state transition create stochastic future trajectory outcomes.
- The sensor/dynamics/common-cause model may itself be unknown, but remain coherent and persistent for the entire mission.
- A receding-horizon controller may recompute its plan after every observation, changing which future outcomes are considered the tail.

One scalar "confidence" cannot faithfully represent all of these.

## Four distinct risk formulations

Let \(b_t\) denote the current belief over hidden physical and common-cause state, \(m\) a complete persistent mission model, \(\pi\) an observable policy, and \(\omega\) a stochastic mission outcome path conditional on \(m,\pi\).

### 1. Belief-state immediate-cost CVaR

For immediate state-dependent loss \(c(x,a)\), evaluate:

\[
C_{\mathrm{belief}}(b_t,a;\alpha)
=
\operatorname{CVaR}_{\alpha}^{x\sim b_t}[c(x,a)].
\]

The alpha convention must be explicit. In the belief-state primitive, the caller supplies tail mass (e.g. 5%); the path-risk API supplies confidence alpha (e.g. 95%). This formulation targets dangerous states that carry little belief mass but high immediate consequence. It does not directly bound accumulated mission-path loss.

Pariente & Indelman's NeurIPS 2026 formulation replaces expected immediate cost with this belief-tail cost, while retaining expected cumulative future return. Its finite-time planning / particle-belief guarantees depend on their stated boundedness, sampling and approximation assumptions; these do not automatically transfer to our persistent joint model set.

### 2. Static mission-trajectory CVaR

Evaluate the tail of the complete mission return:

\[
R_{\mathrm{static}}(\pi,m)
=
\operatorname{CVaR}_{\alpha}
\left[
\sum_{t=0}^{T} \gamma^t L_t
\mid m,\pi
\right].
\]

This captures rare bad whole mission paths. The exact horizon-two optimizer in the temporal CVaR branch currently targets this finite static root-policy objective, with an outer maximum over complete persistent models.

Static trajectory CVaR is not generally time-consistent under receding-horizon replanning. A root policy can be optimal for the original unconditional path distribution while, after observing a branch, a freshly optimized conditional tail objective prefers a different continuation.

### 3. Dynamic / iterated conditional CVaR

A nested or iterated risk measure recursively applies a conditional risk operator to future costs / values. It is a comparator when the mission requires a time-consistent risk preference under sequential replanning.

The exact recursion depends on the chosen conditional risk envelope and information filtration. Do not simply write "nested CVaR" without specifying:
- whether the tail mass is fixed or state / stage adapted;
- the conditioning information at each epoch;
- whether the risk envelope is rectangular or coupled across time;
- how hidden persistent model identity is retained;
- whether common-cause regimes may change during the mission.

Dynamic CVaR can be more conservative and can amplify sensitivity as conditional tail operators are repeatedly applied. Its computational and statistical cost must be measured rather than assumed acceptable.

### 4. Persistent-model robustification

The outer epistemic objective is distinct from all three inner path / belief risk measures:

\[
R_{\mathrm{robust}}(\pi)
=
\max_{m\in\mathcal M}
R(\pi,m).
\]

Each \(m\) is a complete joint observation/dynamics/common-cause model held fixed for the entire mission. Do not let the adversary reselect an arbitrary model independently at each observation branch unless that is explicitly the threat model. The latter is a rectangular / branch-switching diagnostic, not the same mission-persistent uncertainty.

## What is implemented today

- The belief-immediate-CVaR primitive evaluates immediate state-dependent cost across weighted current-belief particles and returns exact expected cost plus exact upper-tail CVaR.
- The robust mission-CVaR policy optimizer enumerates the existing finite observable horizon-two policy family, reconstructs full mission-path loss distributions per persistent model, and chooses the policy with the lowest outer worst-case model-conditional path CVaR.
- The expected minimax, minimax regret, and rectangular switching objectives remain separate.

Not yet implemented:
- A belief-CVaR POMDP policy optimizer with expected future return.
- A nested / time-consistent CVaR policy-tree recursion.
- A receding-horizon risk-budget state or theorem guaranteeing that re-optimization preserves the original root risk objective.
- An event-triggered mode-replacement controller integrated with the temporal sensing policy.

## The decisive counterexamples

| ID | Construction | Question |
|---|---|---|
| R01 | Same expected immediate cost, different belief tails | Does belief-CVaR reject the rare high-consequence state? |
| R02 | Same one-step belief-CVaR, different full path-loss distributions | Does static path-CVaR distinguish trajectory dependence? |
| R03 | Root-optimal static CVaR tree, but fresh conditional replanning changes the continuation | Is the deployed receding-horizon controller still consistent with the mission contract? |
| R04 | Fixed mission model versus branchwise model switching | How much conservatism is caused by rectangular adversary semantics? |
| R05 | Evidence captured earlier and used later under uncertain dynamics | Does risk use the actual decision/use epoch? |
| R06 | Persistent common cause affects several sensor outcomes | Is dependence preserved rather than multiplied away? |
| R07 | No risk-feasible continuation exists | Does the controller stop / abstain / choose an approved lower-risk mode? |
| R08 | Horizon-level risk becomes unacceptable but one-step action remains filter-safe | Does the controller replace the plan rather than repeatedly repairing it? |

R03 is especially important. We need an explicit two-stage numeric example with:
1. the exact unconditional loss distribution for each root policy;
2. the CVaR-optimal root continuation;
3. the posterior after each observable branch;
4. the conditionally reoptimized continuation and its resulting complete-mission loss;
5. the resulting mission-level risk under the original root objective.

The result should identify whether deployment should commit to the selected tree, use a time-consistent nested risk operator, or carry an explicit remaining-risk-budget state.

## Event-triggered plan / mode replacement

CERT-Replan (CoRL 2026) is a useful comparator: https://nail-uh.github.io/corl2026.github.io/

It combines horizon-indexed conformal obstacle-prediction calibration, a one-step barrier safety filter, and a horizon-level CVaR monitor that triggers replacement of a risky local planning mode (e.g. speed profile, corridor or homotopy class). Its reported conditional horizon-risk certificate is not a distribution-free closed-loop guarantee.

Navigation transfer hypothesis: when a qualified horizon risk exceeds its allocated budget, choose a different feasible mode—stop, yield, slow down, change corridor, alter viewpoint, or acquire a different modality—instead of repeatedly correcting the same nominal plan. Keep obstacle-barrier risk separate from UWB localization integrity.

## Benchmark contract

Report these values separately for every policy and every persistent model:

- expected immediate belief cost;
- belief-state CVaR at declared tail mass;
- expected complete-mission loss;
- static mission-path CVaR at declared confidence level;
- dynamic / nested CVaR, when implemented;
- outer worst-case persistent-model value;
- minimax regret;
- rectangular-switching diagnostic;
- risk-feasible policy availability, abstention and false-degraded exposure;
- false-safe probability and exposure, evaluated at actual use epoch;
- resource use, runtime, memory and branch count.

Compare on identical observable history, action set, model set, resource envelope, time semantics and evaluator information. Do not add, average or multiply risk metrics into a single score without a risk-composition theorem.

## Qualification ladder

1. Exact enumerations and hand-derived cases on small beliefs / two-stage paths.
2. Independent implementation of the same objective and policy evaluator.
3. Metamorphic checks: alpha monotonicity, translation/scaling, candidate-order invariance, model-set expansion, zero-mass rejection and no hidden-state policy access.
4. Seeded adversarial branches for model persistence, common causes, non-stationary dynamics, stale evidence and risk-budget exhaustion.
5. Episode/independence-group calibration and data admission.
6. Matched real replay, then reviewed physical trials.

## Claim boundary

These formulations are mathematically distinct. The implemented primitives are software references on supplied synthetic integer weights / losses. They do not establish calibrated physical probabilities, transition-model coverage, distribution-free mission guarantees, PHMI, safety certification or operational navigation authority.
