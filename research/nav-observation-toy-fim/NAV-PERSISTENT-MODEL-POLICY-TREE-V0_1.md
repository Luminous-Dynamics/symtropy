# Mission-persistent observation-model policy tree v0.1

**Status:** isolated offline Bayesian model-mixture oracle for NAV-MODEL-CONTINGENT-VOI-001 (#7251). Stacked independently on the validated binary horizon-two reference in PR #7250.

## Paradigm shift

An unknown observation model is a latent mission variable, not a fresh random draw at every measurement. A model's likelihood row remains fixed across the policy tree. Each observation—including no-observation—updates the posterior over both navigation state and model identity.

The v0.1 oracle uses a finite two-model Bayesian mixture. The initial four-element PPB prior is ordered:
- model 0 / inside;
- model 0 / outside;
- model 1 / inside;
- model 1 / outside.

This joint prior sums exactly to one billion. It encodes both the model prior and each model's navigation-state prior. Every action supplies a separate normalized outcome law for each of those four states. A single policy tree is selected using the mixture belief; the controller does not see the hidden model and cannot choose a different action for each model.

This is explicitly a **Bayesian model-mixture** result. It is not a minimax or distributionally robust guarantee. The mission-global worst-case objective over a persistent model set is a separate extension under #7251.

## Implemented contract

The Rust module navigation_persistent_model_policy_tree.rs provides:

- joint model/navigation prior with exact PPB normalization;
- persistent model-specific likelihood rows across the horizon;
- exact updates of navigation-state and model-identity posterior beliefs;
- no-observation/dropout as an informative event when model-dependent;
- one shared controller action per observable branch, not one per hidden model;
- asymmetric false-safe / false-degraded terminal loss;
- exact non-anticipative horizon-two enumeration and matched greedy-first comparator;
- hard latency, energy, bandwidth constraints; action non-reuse; deterministic ordering; checked u128 arithmetic; explicit abstention and non-authority marker.

## Exact model-identity counterexample

With equal prior model mass and two sensors that each report outcome 1 with probability 0.1 under model 0 and 0.9 under model 1, the observation outcomes are dependent after marginalizing over the persistent model:

\[
P(O_1=1,O_2=1)=0.5(0.1^2)+0.5(0.9^2)=0.41
\]

but the product-of-marginals approximation yields \(0.5^2=0.25\). A first observed 1 changes the model-1 posterior from 0.5 to 0.9. A no-observation event also changes the model posterior when dropout likelihoods differ between models.

These are constructed exact probabilities, not empirically calibrated model probabilities.

## Tests authored

Ten tests cover model-posterior update, no-observation model update, induced dependence, exact-vs-greedy risk, candidate-order invariance, hard resource budgets and non-reuse, malformed joint priors and likelihoods, asymmetric terminal loss, explicit abstention, and the hidden-model/controller boundary.

Exact-head CI remains the gate. No physical data or sensor acquisition is implied.

## Robust-model boundary

The next minimax version must optimize a *single observable policy tree* against a persistent mission-global model:
\[
\min_{\pi}\max_{m\in\mathcal M} \mathbb E_m[L\mid\pi].
\]

It must first evaluate the complete policy's risk under each model, then take the maximum. Applying \(\max_m\) independently at each branch creates a different rectangular adversary that can switch models between outcomes. That may be a useful pessimistic upper bound but must not be conflated with one persistent mission model.

The exact global minimax version requires common terminal actions across models and joint enumeration of first action, branch continuations and terminal decisions. It remains open work; this Bayesian mixture module does not claim to solve it.

## Research grounding

- Li, Kuhn & Sutter, *Policy Gradient Algorithms for Robust MDPs with Nonrectangular Uncertainty Sets*, SIAM Journal on Optimization, 2026: https://doi.org/10.1137/24M1631250
- Kumar et al., *Non-rectangular Robust MDPs with Normed Uncertainty Sets*, NeurIPS 2025: https://papers.nips.cc/paper_files/paper/2025/hash/372cb7805eaccb2b7eed641271a30eec-Abstract-Conference.html
- Jakeman et al., *Risk-Aware Goal-Oriented Bayesian Optimal Experimental Design*, 7 September 2026: https://arxiv.org/abs/2609.08023
- UWB critical survey (23 November 2026): https://doi.org/10.1186/s13638-026-02703-9
