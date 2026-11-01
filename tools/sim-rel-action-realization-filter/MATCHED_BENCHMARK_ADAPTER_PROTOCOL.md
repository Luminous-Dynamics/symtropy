# SIM-REL-041 — matched scientific benchmark adapter contract

Status: first matched-pair admission and summary layer implemented on a research branch. No external benchmark is yet fully adapted and no SOTA claim is implied.

## 1. Why a matched adapter is necessary

A comparison is not scientifically interpretable when the reference and candidate systems see different tasks, model configurations, prompts, tools, harnesses, resource budgets or evaluation scopes.

A planner-level result cannot establish end-to-end laboratory superiority. A full scientific campaign cannot be compared fairly if one arm is given more simulator calls, more tool actions or a different foundation model.

SIM-REL-041 creates a common evidence contract before connecting any external baseline implementation.

## 2. Evaluation scopes remain separate

Each campaign manifest has exactly one scope:

- **PlannerOnly:** same public model/task and decision problem; isolates planning/selection logic.
- **ModelInference:** same observations and inference target; isolates model/inference quality.
- **PhysicalExecution:** same apparatus/device interface and action/sensor evidence; isolates action realization and safety.
- **FullScientificCampaign:** complete agent + model + tools + harness + discovery/evaluation workflow.

A campaign cannot pool results from different scopes. Planner-only success must not be presented as autonomous-science success.

Each campaign also binds one evidence split. Development, Calibration and SealedHoldout results must not be pooled. A future sealed-evaluation controller will freeze agent and evaluator identities before opening the holdout.

## 3. Matched campaign manifest

A pre-registered manifest binds:

- campaign/protocol identity;
- scope and evidence split/group;
- fixed agent panel;
- task manifest digest;
- model and prompt configuration digests;
- harness and toolset digests;
- reference and candidate arm IDs;
- shared wall-time, token, tool-call, simulator-step and energy budgets;
- minimum paired samples per mechanism family.

Each paired trial repeats the shared conditions. The verifier rejects a trial if its task/model/prompt/harness/toolset/scope/split/group/panel/budget differs from the campaign manifest.

## 4. Paired outcome evidence

Each valid pair contains:

- unique task/pair ID, mechanism-family ID and paired seed;
- task-validity result and oracle receipt digest;
- one reference result and one candidate result;
- system version, completion, safe success, hazard, abstention and terminal-loss units for each arm;
- per-arm resource usage and evidence artifact digest.

Both arms must stay within the same declared resource caps. Invalid tasks are recorded but excluded from solver outcome statistics and must not contain arm outcomes.

A safe-success receipt is invalid if the arm did not complete the task, observed a hazard, or abstained. Completion and safe success are distinct.

## 5. Reporting without a misleading composite score

The first implementation reports per-mechanism-family counts:

- valid paired trials and invalid tasks excluded;
- candidate lower/equal/higher terminal-loss counts;
- exact sum of candidate-minus-reference terminal loss (divide by pair count for the exact mean);
- completion, safe-success, hazard and abstention counts for both arms;
- whether the pre-registered minimum pair count was met.

This intentionally does not collapse task completion, safety, model adequacy, resource use and terminal loss into one arbitrary leaderboard score. Confidence intervals, paired permutation/bootstrap analysis and statistical significance are handled by the later analysis layer; raw per-pair receipts remain the source of truth.

## 6. Independence boundary

This adapter verifies match conditions, budget compliance, task validity labels and consistency of the supplied per-arm records. It does not verify that an external baseline was implemented faithfully, that a model digest fully identifies a provider's behavior, that an evaluator is unbiased, or that an evidence artifact is truthful.

External baselines require pinned reference code/paper/config versions and independent result reproduction. Where a baseline cannot be run under the same task, model, harness and budget, report it as a contextual literature comparison, not a head-to-head result.

## 7. Initial reference matrix

| Comparator | Published capability | SIM-REL comparison layer |
|---|---|---|
| Constrained Bayesian Experimental Design via Online Planning (ICML 2026) | Multi-step experimental planning under dynamic constraints | PlannerOnly: terminal decision loss, feasibility and lookahead value |
| R-IDeA (AISTATS 2026) | Representativeness, informativeness and error de-amplification under misspecification | PlannerOnly / ModelInference: distribution-shift decision regret |
| EGIG / On the Misinformation in a Statistical Experiment (AISTATS 2026) | Explicit inferential and data-generating model-misspecification penalties | ModelInference: error attribution and decision quality |
| Robust Bayesian Decision Making under Adversarial Uncertainty (UAI 2026) | Sequential decision robustness to adversarial uncertainty | PlannerOnly: worst-world loss and policy stability |
| SafeLab (ICML 2026) | Calibrated apparatus and trajectory-level safety | PhysicalExecution: safe success, hazards, abort and recovery |
| EmbodiedLab (October 2026) | Instrument-grounded physical scientific discovery | PhysicalExecution / FullScientificCampaign: action-to-knowledge chain |
| Autonomous Science Bench / OpenScience | Whole-campaign scientific discovery and held-out learning | FullScientificCampaign: reproducibility and unseen-design prediction |
| AutoSciBench (October 2026) | Automated scientific benchmark recipe generation and refinement | Benchmark validity, novelty, difficulty and leakage controls |

The matrix identifies where to compare, not where superiority has already been demonstrated. The external systems have different scopes; results must remain scope-matched.

## 8. Qualification plan

1. Validate exact manifest and pair matching with exhaustive small registries.
2. Reject every one-field mismatch (task, model, prompt, harness, tools, scope, split, panel, group, budget).
3. Reject over-budget usage, duplicate pair/task/seed IDs, reused evidence artifacts and inconsistent safe-success labels.
4. Preserve invalid-task counts without allowing those tasks into outcome statistics.
5. Require at least 30 paired seeds per mechanism family where practical; target 100+ and report 95% paired confidence intervals.
6. Pre-register candidate/reference identities, statistical metric, exclusions, resource budget and sealed holdout before opening evaluation.
7. Report per-family and per-scope failures, including families where each baseline wins.

Do not claim superiority from different models, different harnesses, different budgets, different task exposures, or planner-level tests presented as full autonomous-science comparisons.
