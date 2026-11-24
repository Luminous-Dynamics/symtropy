# Mission-global minimax regret policy tree v0.1

**Status:** draft exact finite extension to the mission-global minimax risk oracle in PR #7277 and issue #7276. It is offline synthetic research only.

## Why a second robust objective

Mission-global minimax risk asks the policy to minimize its worst absolute terminal loss:
\[
\min_{\pi\in\Pi_{\mathrm{observable}}}\max_{m\in\mathcal M}
\mathbb E_m[L\mid\pi].
\]

Minimax regret asks how much performance is lost by not knowing the true model:
\[
\min_{\pi\in\Pi_{\mathrm{observable}}}\max_{m\in\mathcal M}
\left(\mathbb E_m[L\mid\pi]-L_m^\star\right),
\]
where \(L_m^\star\) is the best achievable risk when model identity is known to the evaluator/controller comparator.

These are different objectives. A model may intrinsically be difficult even for a model-aware planner, so worst absolute loss can overweight that model; regret instead measures the avoidable penalty caused by using one policy across multiple possible models. Both should be reported.

## Implemented

The existing module navigation_mission_minimax_policy_tree.rs now adds:

- plan_mission_global_minimax_regret;
- one exact model-aware oracle risk per persistent model;
- a model-aware oracle built by duplicating the selected model into both model slots, then invoking the same exhaustive policy-tree solver;
- exact per-model regret subtraction using checked integer arithmetic;
- a single shared observable policy-tree search minimizing maximum model regret;
- the chosen policy tree, model-wise oracle baselines, model-wise regret values, maximum regret, and common risk denominator.

The model-aware comparator may choose a different first action, continuation and terminal decision under each model. The selected robust policy still uses one shared action/decision mapping for all models; no hidden model truth is exposed to it.

## Tests

Two new tests check:
1. shared-policy risk never falls below the exact model-aware oracle for either model, maximum regret equals the maximum of the model-wise regrets, and the plan remains non-authorizing;
2. candidate ordering does not alter the exact minimax-regret result.

The prior six global-minimax tests remain in place. The exact-head GitHub workflow must pass before this PR is qualified.

## Important boundaries

- The model-aware oracle is an evaluator comparator, not a deployable policy. It knows model identity.
- The robust policy does not know the true model and is fixed across models.
- The finite model set must contain the true model for these guarantees to apply; this code does not calibrate or authenticate model-set coverage.
- This is a two-model, four-state, two-stage finite problem with at most four actions. It does not imply tractability for arbitrary non-rectangular robust MDPs.
- The branchwise model-switching adversary remains a separate rectangular diagnostic, not the mission-global physical model.
- No posterior update, physical acquisition, protection-level transition, navigation authority change, or actuation is performed.

## Benchmark outputs

For each frozen benchmark instance publish:
- model-wise shared-policy terminal losses;
- model-aware oracle risk for each model;
- model-wise and maximum regret;
- mission-global worst-case risk;
- branchwise model-switching diagnostic;
- policy tree, branch outcomes, terminal decisions, worst-path resource use, number of enumerated trees and execution time.

Do not collapse worst-case loss and regret into one score. Both are required.

## References

- Li, Kuhn & Sutter, *Policy Gradient Algorithms for Robust MDPs with Nonrectangular Uncertainty Sets*, SIAM J Optim (2026): https://doi.org/10.1137/24M1631250
- Kumar et al., *Non-rectangular Robust MDPs with Normed Uncertainty Sets*, NeurIPS 2025: https://proceedings.neurips.cc/paper_files/paper/2025/hash/372cb7805eaccb2b7eed641271a30eec-Abstract-Conference.html
- Osogami, *Robust partially observable Markov decision process*: https://jmlr.csail.mit.edu/proceedings/papers/v37/osogami15.pdf
