# Temporal mission-global minimax policy oracle v0.1

**Status:** finite, offline Rust reference implementation. All model probabilities and transition kernels are supplied synthetic inputs. This is not a calibrated physical motion model, dataset result, safety monitor, or navigation authority.

## Purpose

This module composes three previously isolated ideas:

- Persistent observation-model and common-cause state: PR #7275.
- Mission-global minimax risk and regret under one shared observable policy: PRs #7277 and #7278.
- Belief propagation from measurement capture to evidence use: PR #7284.

The new object is one non-anticipative sensing policy that remains valid as a candidate model and common-cause regime persist, while the navigation service state may evolve during each evidence-age interval.

## Hidden-state and action contract

For every fixed model (m \in \{0,1\}), the initial distribution is over four states:

1. nominal / inside;
2. nominal / outside;
3. shared-fault / inside;
4. shared-fault / outside.

The two common-cause regimes are mission-persistent. Observation likelihoods are conditioned on model, common-cause regime, and navigation state. The transition kernel is conditioned on model and common-cause regime and moves only inside/outside mass. It never changes model or common-cause identity.

Each action supplies:
- a three-outcome likelihood row (no observation, observed zero, observed one) for each joint model/cause/navigation state;
- one inside/outside Markov transition for every model/cause pair;
- an evidence-age duration, resource vector, observation-model commitments and transition-model commitments.

For each action, the computation is:

1. condition the historical capture-epoch mass on the observed outcome;
2. propagate that conditional mass through the corresponding model/cause transition;
3. treat the result as the posterior mass at the action's use epoch.

No-observation follows the same path. For a second action, its capture epoch is the previous action's use epoch. The second action is chosen only from the first observable outcome.

## Three distinct objectives

### Mission-global minimax risk

The solver enumerates one shared observable policy (pi) and evaluates the same policy under each fixed mission model:

[
\min_{\pi \in \Pi_{obs}}\max_{m\in\{0,1\}} E_m[L\mid\pi].
]

The selected action and all branch decisions are model-blind. Model-specific losses are returned so the worst-case model is inspectable.

### Mission-global minimax regret

For each model (m), a separate model-aware oracle (L_m^*) is computed by exact minimization over the same finite policy class when that model is known. The robust controller still uses one shared observable policy:

[
\min_{\pi \in \Pi_{obs}}\max_m\left(E_m[L\mid\pi]-L_m^*\right).
]

Regret is not the same as absolute risk. The module returns separate selected policies for the two objectives.

### Rectangular branch-switching diagnostic

The diagnostic holds the first-stage generating model (m_0) fixed while the first action produces an observable branch. After that branch, the adversary may select a second-stage model (m_1) independently for the branch's second action. The controller's action and terminal-decision map remain the same across all model choices.

The resulting objective maximizes over (m_0) after summing its root branches, with the worst (m_1) chosen separately for each branch. This is a distinct rectangularized diagnostic. It must not be presented as the mission-global persistent-model objective or as a unique physically correct adversary.

## Exact arithmetic bound

Let (S=10^6) be the PPM scale.

- Initial prior mass sums to (S).
- First-stage observation multiplies by (S): mass scale (S^2).
- First-stage transition multiplies by (S): mass scale (S^3).
- Second-stage observation multiplies by (S): mass scale (S^4).
- Second-stage transition multiplies by (S): mass scale (S^5).

Therefore the common risk denominator is:

[
S^5 = 10^{30}.
]

For a terminal loss no larger than (10^6), the maximum risk numerator is bounded by (10^{36}), below the (u128) maximum (approximately (3.4\times10^{38})). Every addition/multiplication uses checked arithmetic; overflow is an error, not saturation. A stopped first-stage branch is padded by (S^2); a no-sensing terminal decision is padded by (S^4) so all policies share the same denominator.

This bound is specific to the v0.1 state count, horizon, PPM scale and loss cap. Extending horizon or state dimension requires a new bound before changing constants.

## Policy enumeration bound

For at most four actions:
- choose one first action;
- for each of its three observable outcomes, choose either one of two stop decisions, or one of at most three remaining actions and one of (2^3=8) terminal-decision maps;
- therefore each first-outcome branch has at most (2+3\times8=26) choices;
- at most (26^3=17,576) policies per root action;
- at most four root actions plus two no-action terminal policies: (4\times17,576+2=70,306).

The policy count is returned. Actions are sorted by stable ID and branch/outcome/decision enumeration order is fixed for deterministic tie behavior.

## Resource and fail-closed rules

- Every model prior must sum exactly to one PPM scale.
- Every likelihood row must sum exactly to one PPM scale.
- Transition probabilities must be within [0, S].
- Evidence age may not exceed action latency.
- Zero evidence age requires identity transition for every model/cause pair.
- Every model/cause pair requires nonzero observation and transition commitment bytes. This checks commitment presence only; it does not authenticate them.
- Duplicate/zero action IDs reject.
- At most four candidate actions are admitted.
- Every selected first/second-stage path must fit latency, energy and bandwidth budgets.
- The second action cannot reuse the first action.
- Missing transition evidence never defaults to identity.
- The policy output never authorizes a navigation mode transition.

## Tests authored

The module includes tests for:
- opposing persistent transition models changing the robust sensor choice;
- global persistent risk versus rectangular switching;
- common-cause mass remaining in its original cause slice under navigation-state transitions;
- missing transition commitments and zero-age identity;
- model-label permutation invariance;
- model-set extension not beating the model-specific oracle lower bound;
- action permutation invariance, branch resource ceilings and no action reuse;
- evidence age, malformed likelihood and duplicate-ID rejection;
- no-action rectangular objective consistency.

Exact-head CI remains the qualification gate. These are synthetic finite-model tests only.

## Not yet modeled

- Continuous pose, velocity and orientation.
- Arbitrary duration transition semigroups and nonstationary motion.
- Separate capture, sample-completion, transport/queue, receipt, decision and actuation epochs.
- Uncertain transition-kernel sets within one nominal model.
- More than two candidate mission models or common-cause regimes.
- More than two sensing stages.
- Non-Gaussian continuous observation distributions.
- Physical kernel calibration, source authenticity, statistical coverage or field availability.

These should be introduced only with their own explicit evidence/time contracts and checked arithmetic budget.
