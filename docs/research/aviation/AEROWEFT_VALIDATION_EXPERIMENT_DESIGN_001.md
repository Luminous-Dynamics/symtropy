# AeroWeft — Decision-Relevant Validation Experiment Design

**Tracking issue:** #5394  
**Composes:** Track T #5362, Track U #5371, Symtropy #4487/#460/#4366/#3749/#4446  
**Status:** synthetic-first candidate admission and Pareto selection prototype  
**Claim ceiling:** deterministic ranking of supplied candidate metadata only; no physical test authorization or realized information-gain claim.

## Research question

When several candidate aircraft, sensor, control or environment models remain plausible, which next simulation, analytic, bench, SIL, HIL, shadow or controlled-flight experiment best distinguishes the competing hypotheses and reduces uncertainty that matters to the engineering decision?

The answer must account for safety/admissibility, independent references, semantic equivalence, held-out data integrity, model-family correlation, robust utility, decision relevance and cost. Maximum information gain in the abstract is not the objective.

## Paradigm shift

Replace a fixed huge Monte Carlo campaign with sequential, decision-aware experimentation:

1. Declare the engineering decision and the uncertainty that can change it.
2. Identify the competing model hypotheses and their correlation/lineage.
3. Generate candidate experiments across the qualified modality portfolio.
4. Reject candidates lacking approval, safety-envelope fit, independent references or semantic equivalence.
5. Pareto-rank the admissible candidates by decision-relevant uncertainty reduction, model discrimination, worst-case robustness, relevance, uninformative probability and cost.
6. Run only an independently authorized test; preserve full observations and uncertainty.
7. Re-evaluate model claims under Track T, then use Track U for later change impact.
8. Stop when the decision is robust, or abstain when evidence cannot resolve it.

No opaque weighted score. A test may be information-rich but decision-irrelevant; a cheap test may be useful but not sufficient; a safe experiment may still be non-discriminating. Keep these dimensions explicit.

## Ownership

Track V owns the aviation-specific experiment design profile, frozen candidate/utility schema, Pareto selector, sealed-holdout controls and model-discrimination benchmark. It does not own:
- simulator/solver backends or multi-fidelity physics model selection (#4487);
- the single canonical physics qualification corpus (#460);
- semantic-equivalence policy (#4366);
- analytic/manufactured numerical oracles (#3749);
- differential fuzzing/minimization (#4446);
- model validation or evidence applicability (Track T/U);
- provenance trust custody (Track Q/Mycelix);
- test authorization or operational decisions.

The prototype consumes owner-supplied admissibility facts. It does not independently establish them.

## Selection contract

Candidate estimates are normalized [0,1] conservative bounds produced by a pinned estimator and method at a declared confidence level. Benefits are lower bounds; uninformative probability is an upper bound. The selector does not calculate or authenticate those estimates. Model-discrimination candidates must also provide explicit hypothesis-pair contrast estimates; listing model families alone is not sufficient. The exact campaign, model/sensor/control scope, intended-use profile, scenario, design and measurement-plan digests are bound to every candidate.

The following dimensions remain separate:
- expected decision-relevant predictive uncertainty reduction;
- expected model-hypothesis discrimination;
- worst-case robust uncertainty reduction;
- decision relevance;
- probability of an uninformative outcome;
- wall-clock minutes;
- setup, facility/operator, CPU, accelerator, energy and adjudication costs as non-substitutable resource dimensions.

Hard eligibility gates include:
- decision question and objective identity match;
- explicit external authorization where required; ControlledFlight is always authorization-gated, and the frozen policy can require authorization for HIL/other modalities;
- owner receipts for affirmative safety, envelope, reference, semantic-equivalence and authorization gates; ControlledFlight is always authorization-gated, and the frozen policy can additionally require authorization for HIL/other modalities;
- positive owner receipts for safety, envelope, independent-reference, semantic-equivalence and authorization gates;
- safety constraints clear;
- within declared test envelope;
- independent reference measurement/oracle available;
- semantic equivalence established for cross-implementation comparisons;
- no use of sealed confirmatory or challenge outcomes to select the experiment;
- two or more exact model hypotheses with distinct artifact digests and correlation/lineage groups for model-discrimination claims; explicit pairwise contrasts name both hypothesis IDs, provide a conservative lower bound on predicted outcome separation, and bind a contrast-estimate digest; multiple variants from one correlated family count as one group-pair, not multiple independent distinctions;
- probability of an uninformative result below a frozen policy maximum;
- frozen policy thresholds and cost budget satisfied.

Only after eligibility is established does Pareto ranking happen. Benefits are maximized; uninformative probability and each resource-cost dimension are minimized. Costs are never summed or exchanged: a low compute cost cannot compensate for exceeding a facility, energy or adjudication budget. The output is a set of nondominated options, not a single "winner" forced by arbitrary scalarization. The Pareto record exposes the number of distinct independent model-family pairs with qualifying contrast bounds.

## Frozen benchmark suite

The dependency-free Rust suite currently contains 55 adversarial tests:
- positive control, dominated-option removal and genuine cost/utility tradeoffs;
- high-information but decision-irrelevant experiment rejection;
- missing/denied/unknown authority;
- uncleared safety constraint and out-of-envelope experiment rejection;
- missing independent reference rejection;
- cross-implementation semantic mismatch and matched positive control;
- sealed challenge/confirmatory data leakage rejection;
- correlated model-family pseudo-diversity rejection;
- explicit pairwise hypothesis contrasts, lower-bound separation thresholds, duplicate hypothesis IDs, same-artifact cross-family relabelling and distinct group-pair coverage;
- explicit model-discrimination and robust-reduction thresholds;
- cost-budget and malformed/non-finite utility rejection;
- parameter-identification case that does not falsely require independent model families;
- question/objective binding;
- deterministic input permutation;
- duplicate candidate identity and invalid policy rejection;
- reject-all control returns abstention, never success;
- exact campaign/model/use-scope binding;
- affirmative owner-receipt requirements and authorization hard-gate against candidate-flag bypass;
- frozen estimator build/method/confidence binding;
- non-substitutable resource budgets and multidimensional Pareto tradeoffs;
- valid zero ceilings for resource dimensions that a modality is not allowed to consume.

## SOTA comparison protocol

Use equal candidate sets, priors, available information, admissibility rules and *each separate resource budget* to compare:
- fixed expert-designed test matrices;
- factorial/covering-array plans;
- Monte Carlo, Latin-hypercube and Sobol sensitivity campaigns;
- parameter-only expected information gain;
- Bayesian optimal experimental design;
- robust/minimax experimental design under prior/model uncertainty;
- active learning and sequential model discrimination;
- batch active learning for reliability-based model validation (Lämmle et al., Reliability Engineering & System Safety 272, August 2026; DOI 10.1016/j.ress.2026.112631), including mean-/quantile-based misclassification acquisition and input constraints;
- model-discrepancy active learning with Bayesian experimental design (Yang et al., Computer Methods in Applied Mechanics and Engineering, 1 November 2025; DOI 10.1016/j.cma.2025.118198);
- the multi-fidelity admission/escalation lane in #4487.

Report:
- decision-relevant uncertainty reduction per cost;
- model discrimination on independent model-family groups;
- worst-case utility under prior/model misspecification;
- decision-boundary regret against an independently constructed oracle Pareto set;
- probability of uninformative result and false-utility calibration;
- unsafe/ineligible candidate selection rate (target zero);
- adaptive development versus untouched confirmatory data leakage (target zero);
- selector runtime, experiment execution cost and evidence-adjudication cost separately.

A reduction in test count is not success if the resulting model is less reliable or the decision changes under plausible model discrepancy.

## Hard prospective targets

- 100% exact agreement with a separate finite known-truth selector oracle.
- Zero ineligible or unsafe candidates selected.
- Zero sealed confirmatory/challenge outcomes used to pick the experiment.
- Zero correlated hypothesis family groups counted as independent models.
- Zero decision-irrelevant high-information experiments admitted below the frozen relevance threshold.
- Every known-truth challenge returns an oracle-Pareto candidate or an explicit reproducible abstention.
- Calibrated utility estimates on a held-out campaign, with regret versus oracle Pareto selection.
- Improvement over fixed/random/parameter-only baselines at equal constraints and budget, measured on held-out decision outcomes.
- Report by modality/model family/risk tier; no pooled scalar obscures failures.

## Industry/research anchors

- NASA-STD-7009B: modeling/simulation lifecycle credibility and intended-use assessment; acceptance criteria are set by the programme/project and approved by delegated technical authority.
- NIST AMS 100-85 (5 October 2026): harmonizes ASME VVUQ with JCGM metrology and argues that credibility effort should scale with model influence and decision consequence.
- NASA-HDBK-7009B (2026-02-03): current implementation guidance accompanying NASA-STD-7009B.
- AIAA R-154-2021: model-based reduction of flight testing and six engineering tasks.
- ASTM F3705-25 / F3442-26: DAA test and performance comparison only for matched operating assumptions.
- NASA NESC-RP-23-01853: cross-simulation comparison under common inputs can contribute to validation, while remaining distinct from empirical flight validation.
- NASA 2025 strategic path management flight evaluation: two helicopters in a live-virtual-constructive environment with up to 330 virtual aircraft; a useful scale comparator, not universal safety evidence.
- NASA NESC 6DOF check-case programme: standardized common inputs and cross-simulation comparisons can identify implementation defects and contribute to model credibility, but remain distinct from physical empirical validation.

## Claim ceiling

A passing result qualifies the selector only for the supplied synthetic candidate space, frozen policy, thresholds, contrast/utility estimates and separate resource budgets. It does not verify those estimates, authenticate owner gate receipts, prove a candidate model true, authorize a physical test, establish aircraft safety, or demonstrate certification/regulatory compliance.
