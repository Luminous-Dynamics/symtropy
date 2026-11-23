# Randomized maximin policies over complete scientific campaign trees

**Status:** finite campaign-tree primal/dual game reference; exact-head CI pending  
**Tracking:** [#7094](https://github.com/Luminous-Dynamics/symtropy/issues/7094), [deterministic campaign PR #7090](https://github.com/Luminous-Dynamics/symtropy/pull/7090)  
**Branch:** `research/cmni-randomized-campaign-policy-v0`

## 1. Why complete-tree randomization is different

The deterministic campaign oracle chooses one complete outcome-contingent tree before it knows the hidden dependence scenario. The one-shot randomized decision game mixes outcome-to-action maps. The next correct capability is to mix **complete campaign trees**: each mixture component contains its own acquisition choices, observed-history branches, stop/abstain decisions and terminal actions.

The randomization distribution is selected ex ante. At execution time a tree is sampled, then the tree responds only to outcomes observed along its branches. The hidden dependence scenario ID is never an input to the policy.

## 2. Finite primal/dual game

Let \\(\\mathcal T_f\\) be the finite set of pathwise-feasible complete trees, and let \\(\\mathcal S\\) be the admitted hidden source/dependence scenarios. For tree \\(\\tau\\) and scenario \\(s\\), let \\(V(\\tau,s)\\) be expected terminal utility minus acquisition costs in declared utility units.

The experimenter chooses a distribution over whole trees:

\\[
V_{\\mathrm{primal}} =
\\max_{q\\in\\Delta(\\mathcal T_f)}
\\min_{s\\in\\mathcal S}
\\sum_{\\tau\\in\\mathcal T_f}q_\\tau V(\\tau,s).
\\]

The dual chooses a least-favourable mixture of admitted scenarios:

\\[
V_{\\mathrm{dual}} =
\\min_{w\\in\\Delta(\\mathcal S)}
\\max_{\\tau\\in\\mathcal T_f}
\\sum_{s\\in\\mathcal S}w_s V(\\tau,s).
\\]

The primal/dual solver is reused from CMNI's randomized robust decision game (#7057). It checks simplex feasibility, the primal/dual gap and complementary-slackness residuals. The least-favourable scenario mixture is a dual witness over the declared finite ambiguity set, not a claim that physical scenarios are randomly generated from that mixture.

## 3. Implemented reference

The Rust API `plan_randomized_non_anticipative_robust_campaign`:
- enumerates complete feasible trees using the deterministic two-stage campaign semantics;
- includes root stopping/abstention policies in the same policy set;
- records each tree's scenario-wise net expected utility and maximum path cost;
- removes only componentwise-dominated or exact-duplicate payoff vectors before solving the game;
- uses the finite primal/dual solver with an independent caller-supplied basis cap;
- returns the whole-tree primal mixture, least-favourable scenario mixture, scenario values, deterministic baseline, randomized gain, clairvoyant upper bound, non-anticipativity gap, pathwise costs, primal/dual gap, feasibility and complementarity residuals.

Pareto pruning is exact for this finite maximin game: a tree with no better payoff in any admitted scenario (and strictly worse in at least one, or identical payoffs with no lower path cost) cannot improve a mixture. The full pre-pruning tree count and scenario-evaluation count are retained.

## 4. Frozen strict-randomization control

Fixture: `research/cmni/fixtures/randomized-complete-campaign-game-v1.json`.

The control reuses the three-state/two-scenario likelihood model from #7057. It has three actions, three outcomes, one acquisition costing 0.01 utility units, and 68 complete campaign trees (four root stop policies plus 64 outcome-to-action acquisition maps).

The optimal whole-tree mixture is:
- map \\( (0,0,2)\\) with probability \\(2/3\\), scenario net utilities \\( (0.39,0.47)\\);
- map \\( (2,0,2)\\) with probability \\(1/3\\), scenario net utilities \\( (0.55,0.39)\\).

The mixture obtains \\(133/300\\approx0.443333\\) net utility in both scenarios. Its dual least-favourable mixture is \\(w=(1/3,2/3)\\), with the same value. The best deterministic complete campaign tree is a stop policy at 0.40 net utility. Randomization therefore adds \\(13/300\\approx0.043333\\) robust utility over the deterministic campaign baseline and over the prior best action. Both mixture components have maximum path cost 0.01, equal to the hard budget.

The fixture is analytic and synthetic, not physical data.

## 5. Adversarial controls

- **Hidden calibration reversal:** a sensor is perfectly informative within each scenario but its outcome/state mapping reverses. The randomized campaign policy must not condition on scenario ID; with 0.05 acquisition cost, stopping at 0.50 remains better.
- **Policy-tree cap:** exceeding the complete-tree cap returns an error before generating an unbounded search.
- **Basis cap:** exceeding the finite primal/dual support-basis budget returns an error, never a partial optimum.
- **Pathwise budget:** every tree in the randomized mixture reports maximum root-to-leaf acquisition cost, and every component must satisfy the hard budget independently.

## 6. Scope and next steps

This is finite-scenario, finite-tree and f64 numerical game verification; `formal_certificate=false`. The dependence receipt is an external scope identifier and is not authenticated. It is not a continuous PSD/elliptope ambiguity solver, a scalable POMDP, formal proof or physical validation.

Next, connect campaign-tree mixtures to continuous dependence uncertainty only for objective classes with a proved convex/affine structure, or derive sound outer relaxations and branch-and-bound bounds. The current covariance SDP cannot be reused as a certificate for arbitrary nonlinear campaign expected utility.
