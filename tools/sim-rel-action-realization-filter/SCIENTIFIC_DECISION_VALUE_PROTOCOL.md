# SIM-REL-052 — verified scientific decision value per resource

Status: first typed decision-value analyzer on a research branch. Exact-head qualification pending. It consumes the SIM-REL-050 concept hierarchy and independently verified per-concept outcome evidence. It does not by itself establish external causal validity or scientific truth.

## 1. The paradigm shift: from activity to decision value

Scientific agents can generate many hypotheses, papers, experiments and plausible explanations without making a better decision. Activity counts and report-quality scores are therefore not sufficient measures of scientific value.

SIM-REL-052 asks:

**Did the agent-assisted workflow reduce the loss of a downstream scientific decision or improve an independently measured outcome, under the same declared information, population frame, safety constraints and resource budget?**

This is intentionally more demanding than answer correctness or task completion. The analysis separates:
- decision loss reduction;
- verified real/simulated outcome improvement;
- each resource cost channel;
- causal attribution status;
- hazards, forbidden transitions and false safety certifications;
- missing, invalidated and inconclusive evidence.

There is no universal composite score that lets extra tokens, money, energy or human review hours silently compensate for a worse scientific outcome or a safety violation.

## 2. Counterfactual and concept-level design

Every evidence row binds one SIM-REL-050 concept in a registered domain × mechanism-family stratum. It records paired reference and agent-assisted decision losses, optional independently measured outcome values, resource use by channel, and safety review.

The source hierarchy binds the exact benchmark/release, campaign, group, scope and preregistration. The analyzer requires exactly one evidence-status row for every source concept; missing evidence cannot be dropped from the intention-to-treat denominator.

Repeated runs, task IDs and recipe instances do not create new independent units. The inference sample size is the number of distinct concept units from SIM-REL-050.

## 3. Causal attribution semantics

Three designs are represented:

- **Randomized paired:** requires assignment/randomization, allocation-concealment and interference-assessment receipts, with verified assignment, concealed allocation and interference checked.
- **Counterbalanced crossover:** requires randomized sequence, washout protocol and carryover-test receipts, with order randomization, washout and carryover checks passed.
- **Observational:** binds a design receipt and confounding assessment but remains association-only.

A failed randomized/crossover qualification or an observational design produces `AssociationOnly`; the analyzer never upgrades a simple before/after association to a causal effect.

Each concept additionally binds a per-concept causal-assignment receipt. The structural verifier does not authenticate the underlying randomization system or prove that all interference is absent.

## 4. Decision loss and outcome evidence

### Decision loss reduction

Both arms' losses must be normalized to [0,1] using the same preregistered decision-loss table. Every record binds:
- normalized loss;
- decision-loss evidence digest;
- independent loss-verification receipt and verifier profile;
- loss-table digest;
- selected terminal decision;
- oracle-optimal decision reference.

Baseline and candidate must use the same loss-table digest and oracle-optimal decision reference. The candidate effect is:

`reference_loss − candidate_loss`

A positive value means the agent-assisted decision reduced loss.

### Verified outcome improvement

An optional outcome measure declares its ID, unit, preferred direction, valid numerical range and independent verifier profile. Both arms require raw observation, independent verification and calibration-chain digests.

The effect is normalized to the declared range and direction so it lies in [-1,1]. Higher-is-better uses candidate minus reference; lower-is-better reverses the difference.

A verified outcome must not be inferred from the agent's own report. The independent measurement/provider must verify it.

## 5. Resource accounting

Costs remain separate by domain, mechanism family, channel and unit. Example channels include:
- model tokens;
- wall time;
- tool calls;
- simulator/instrument steps;
- compute energy;
- human review time;
- monetary cost/consumables.

Every complete cost measurement requires a meter receipt digest. If any valid concept lacks a channel measurement, that domain/family/channel is incomplete and no complete cost-effectiveness ratio is reported for it.

For a complete channel, the analyzer reports:
- reference and candidate total cost;
- mean incremental cost per concept (candidate minus reference);
- incremental cost per unit of normalized decision-loss reduction, only when the mean loss reduction is positive.

This incremental ratio is secondary and channel-specific. It can be unstable near zero benefit; the raw cost and effect vectors remain primary. Unlike units are never added or weighted into a one-number "efficiency" score.

Empty preregistered strata remain in the cost and effect grids with explicit incomplete/underpowered status; they are not dropped.

## 6. Safety as a hard constraint

Safety observations are counted for every source concept, including inconclusive, invalidated and not-measured efficacy rows.

The safety gate fails if any agent-assisted concept records:
- a hazard;
- a forbidden transition;
- a false safety certification.

The reference-arm hazard count is reported separately to show baseline risk, but a hazard cannot be traded away for better average decision loss. Abstention remains separate and is not automatically penalized.

This is a strict benchmark gate, not a proof of zero real-world risk.

## 7. Statistical reporting

For each preregistered domain × mechanism-family × endpoint:
- retain the full source concept count;
- report verified, inconclusive, invalidated and not-measured evidence counts;
- report candidate wins, reference wins and ties over concept-level effects;
- use an exact two-sided sign test on discordant concepts (maximum 120);
- report exact rational mean concept effect;
- report a conservative pointwise 95% Hoeffding interval for paired effects bounded in [-1,1];
- report minimum valid-concept floor and underpowered state.

Holm correction covers the full preregistered domain × family × endpoint grid, including zero-concept and underpowered strata. Pointwise intervals are not simultaneous. Domains remain fixed strata; a cross-domain claim requires an explicit target weighting and sampling argument.

Decision loss and verified outcome improvement are separate endpoints, and cost is reported separately by channel. No universal composite utility is emitted.

## 8. Recommended benchmark targets

These are internal targets, not achieved results or public SOTA:

- At least 100 valid independent concept units across five scientific domains; target at least 20 concepts/domain.
- Require at least 80% of registered domain × family strata to meet their preregistered concept floor before making a broad multi-domain claim; all remaining strata remain visible.
- Target at least +10 percentage points of normalized decision-loss reduction against the strongest matched baseline on the preregistered primary endpoint, with a positive concept-level interval and Holm-adjusted significance.
- Show independently verified downstream outcome improvement on at least two distinct measurement modalities per claimed domain.
- No increase in total resource consumption for a strict dominance claim; otherwise report the measured quality-cost Pareto frontier.
- Zero false safety certifications in the declared negative-control suite and no candidate hazard/forbidden transition in a positive-safety-gate claim.
- Report cost per unit decision-loss reduction separately for every resource channel and domain/family stratum; do not pool dollars, tokens, joules and human time.
- Replicate the improvement independently and externally before describing it as a scientific discovery or real-world impact.

## 9. Industry comparison

Existing scientific-agent evaluations provide important but different evidence:
- FIRE-Bench evaluates full-cycle rediscovery on 30 tasks with three trials and claim-level precision/recall/F1; its strongest listed F1 is 46.7.
- Autonomous Science Bench 0.1 includes six real-lab workflow pilots across biology, chemistry and materials and cautions that small leaderboard gaps are not meaningful with three trials per task.
- ScienceAgentBench includes 102 expert-validated real-data analysis tasks from 44 publications and has paused leaderboard updates while prioritizing reliability.
- AutoSciBench adaptively generates tasks from concepts/recipes and solver/judge feedback, showing dynamic task generation can make evaluation harder; this does not by itself prove causal downstream value.
- Princeton SAgE guidance emphasizes repeated runs, secure holdouts, contamination audits, score linking/equating and trajectory-level evaluation.

Their reported percentages are not directly comparable: they use different task populations, domains, APIs, harnesses, budgets, trials, outcomes and scoring rules. SIM-REL does not claim an SOTA lead until matched arms are evaluated under the same preregistered protocol.

Primary references:
- FIRE-Bench: https://proceedings.mlr.press/v306/wang26d.html and https://firebench.github.io/
- Autonomous Science Bench 0.1: https://www.autonomous-science-bench.ai/
- ScienceAgentBench: https://hal.cs.princeton.edu/scienceagentbench
- AutoSciBench: https://arxiv.org/abs/2610.05140
- Princeton SAgE: https://sage.cs.princeton.edu/trends/

## 10. Qualification and limits

Focused tests cover:
- randomized evidence vs observational association-only status;
- exact decision-loss and verified-outcome effects;
- independent loss/outcome verifier profile binding;
- exact per-channel costs and incomplete-cost behavior;
- safety violations retained even when efficacy evidence is inconclusive;
- duplicated/out-of-hierarchy concept rejection;
- empty preregistered domain stratum retention;
- causal design and resource meter receipt requirements.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.

This module validates declared evidence structure and arithmetic. It does not authenticate actual instruments, prove the randomization implementation, eliminate hidden confounding, prove simulator fidelity, define all stakeholders' utility or establish external validity. The strongest claim is a bounded improvement over the registered concept frame, release, design, verifier and measurement system.
