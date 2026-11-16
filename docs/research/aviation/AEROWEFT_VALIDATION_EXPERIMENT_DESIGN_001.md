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

## Ownership and composition boundaries

Track V owns the aviation-specific experiment design profile, frozen candidate/utility schema, scope-bound gate receipt checks, Pareto selector, sealed-holdout controls and model-discrimination benchmark. It does not own:
- sequential finite-horizon policy/tree optimization or an anytime-valid evidence primitive; compose the existing constrained sequential planner [#3588](https://github.com/Luminous-Dynamics/symtropy/pull/3588), which itself composes #3375/#3547;
- counterexample-guided model-family expansion, adversarial falsification and calibrated ambiguity sets; compose [SIM-REL-029 #3171](https://github.com/Luminous-Dynamics/symtropy/issues/3171);
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

Candidate estimates are normalized [0,1] conservative bounds produced by a pinned estimator and method at a declared confidence level. Benefits are lower bounds; uninformative probability is an upper bound. The selector does not calculate or authenticate those estimates. Model-discrimination candidates must also provide explicit hypothesis-pair contrast estimates; listing model families alone is not sufficient. The exact campaign, model/sensor/control scope, intended-use profile, scenario, design and measurement-plan digests are bound to every candidate and repeated in each affirmative gate receipt. A receipt's payload digest must match the legacy gate reference; duplicate IDs/roles or a receipt from another design/policy fail closed.

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
- two or more exact model hypotheses with distinct artifact digests and correlation/lineage groups for model-discrimination claims; explicit pairwise contrasts name both hypothesis IDs, provide a conservative lower bound on predicted outcome separation, and bind a contrast-estimate digest; multiple variants from one correlated family count as one group-pair, not multiple independent distinctions; the reported Pareto pair count and dominance relation use the same frozen minimum contrast-separation threshold as admission;
- probability of an uninformative result below a frozen policy maximum;
- frozen policy thresholds and cost budget satisfied.

Only after eligibility is established does Pareto ranking happen. Benefits are maximized; uninformative probability and each resource-cost dimension are minimized. Costs are never summed or exchanged: a low compute cost cannot compensate for exceeding a facility, energy or adjudication budget. The output is a set of nondominated options, not a single "winner" forced by arbitrary scalarization. The Pareto record exposes the number of distinct independent model-family pairs with qualifying contrast bounds.

## Utility estimate evidence and objective semantics

The utility estimate is now itself a first-class, candidate-bound evidence object. Each estimate receipt binds the exact candidate ID, campaign, model/sensor/control scope, intended-use profile, decision question, design, scenario manifest, measurement plan, decision boundary, hypothesis-set manifest, robust ambiguity-set manifest and adaptive-development dataset manifest. It also pins the estimator build and method, confidence level, sampling unit and independent sample count, exact hypothesis groups/contrasts, and every conservative utility value. An estimate receipt cannot be replayed across candidate alternatives, and receipt IDs must be unique across the candidate batch. Receipt authenticity and estimator correctness remain Track Q/independent evaluator responsibilities; this crate checks structure and exact value equality only.

For a `ModelFamilyPair` sample unit, the declared sample count must equal the number of distinct qualifying model-family pairs under the frozen contrast-separation floor. Hypothesis/contrast input order is not semantic: reordering the same set must leave admissibility and selection unchanged. The hypothesis manifest digest remains the exact content identity.

The Pareto comparator is objective-aware. `expected_model_discrimination` contributes only to `ModelDiscrimination` campaigns; parameter-identification and decision-boundary campaigns cannot improve their rank by inflating a metric outside their declared objective. The returned model-discrimination field is zero for those other objectives.

## Frozen benchmark suite

The dependency-free Rust suite currently contains 82 adversarial tests:
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
- structured utility-estimate receipt bound to candidate ID, exact decision boundary, hypothesis and robust-set manifests, adaptive-development data, design, measurement plan, estimator/method, utility vector, sample unit and independent sample count;
- candidate-batch uniqueness for utility-estimate receipt IDs;
- set-order invariance for exact hypothesis groups and pairwise contrast records;
- model-family-pair sampling denominators equal the exact number of distinct qualified pairs;
- objective-aware Pareto comparison: model-discrimination values cannot improve a parameter-identification result;
- duplicate utility-estimate receipt IDs rejected across candidate batches;
- estimate receipt replay across candidate alternatives and mismatched utility/hypothesis/sample metadata rejected;
- hypothesis and contrast set permutation invariance;
- model-family-pair sample count checked against the number of distinct qualified pairs;
- objective-aware Pareto comparison, so model-discrimination scores do not affect parameter-identification campaigns;
- non-substitutable resource budgets and multidimensional Pareto tradeoffs;
- valid zero ceilings for resource dimensions that a modality is not allowed to consume.

## SOTA comparison protocol

Use identical candidate designs, prior/ambiguity assumptions, available data, authority/safety admissibility, holdout policy and each separate resource budget. Compare at least:

- fixed expert-designed, factorial and covering-array test matrices;
- Monte Carlo, Latin-hypercube and Sobol sensitivity campaigns;
- parameter-only expected information gain / mutual information;
- Bayesian optimal experimental design for nonlinear models, including published goal-oriented Bayesian OED using MCMC;
- risk-aware goal-oriented BOED that treats posterior predictive error, risk across the prediction domain and risk across datasets as separate design dimensions;
- goal-driven BOED under model uncertainty, directly optimizing the downstream decision instead of all parameter directions equally;
- generalized Bayesian OED / Gibbs EIG under likelihood misspecification, outliers and incorrect observation-noise assumptions;
- robust/minimax experimental design and distributionally robust design;
- sequential OED / active learning with finite-horizon lookahead and model discrimination;
- multi-fidelity EIG and budgeted multi-fidelity escalation;
- batch active learning for reliability-based model validation and model-discrepancy active learning, including the August 2026 mean-/quantile-misclassification acquisition functions and constrained qMC-Prob batch acquisition of Lämmle et al.;
- the multi-fidelity admission/escalation lane in #4487.

Recent specific comparators:
- Jakeman et al., *Risk-Aware Goal-Oriented Bayesian Optimal Experimental Design* (7 September 2026), which composes three risk levels: predictive uncertainty, risk over prediction space and risk over datasets. Its sensor-placement example reports substantial improvement over expected-information-gain allocation in that problem; do not assume this transfers automatically to aviation.
- Go, Qian and Yoon, *Goal-driven Bayesian Optimal Experimental Design for Robust Decision-Making Under Model Uncertainty* (25 May 2026), which targets the downstream decision and model uncertainty rather than maximizing parameter information alone.
- Zhong et al., *Goal-Oriented Bayesian Optimal Experimental Design for Nonlinear Models Using Markov Chain Monte Carlo*, SIAM/ASA Journal on Uncertainty Quantification 14(1), March 2026.
- Barlas, Sloman and Kaski, *Robust Experimental Design via Generalised Bayesian Inference* (10 November 2025), introducing Gibbs expected information gain for resilience to outliers and incorrect noise likelihoods.
- Lämmle et al., *Batch active learning for reliability-based model validation*, Reliability Engineering & System Safety 112631 (August 2026; DOI 10.1016/j.ress.2026.112631): two mean-/quantile-based misclassification acquisition functions, constrained batch qMC-Prob, and a stopping criterion based on misclassification probability. It is a direct Track W calibration/stopping comparator; Track V is a separate admissibility and Pareto-selection layer.

Report:
- decision-relevant uncertainty reduction per each separate cost dimension;
- held-out decision regret against an independently constructed oracle Pareto set;
- risk-tail / high-consequence boundary discovery rate and worst-case utility under prior/model misspecification;
- model discrimination across independently adjudicated model-family groups;
- calibration of utility bounds and their empirical coverage on untouched campaigns;
- probability of uninformative results and false-utility calibration;
- unsafe/ineligible candidate selection rate (target zero);
- adaptive development versus untouched confirmatory/challenge leakage (target zero);
- candidate admission, selector runtime, experiment execution and evidence-adjudication costs separately;
- performance by modality, model family, operating stratum and risk tier.

The selector implemented in this crate is not yet an OED optimizer and does not calculate EIG, Gibbs EIG, a posterior, acquisition functions or realized utility. It also does not verify the empirical independence of estimator samples; it only checks the declared sampling-unit type and count, with an additional exact denominator check for model-family-pair units. It is a scope-/receipt-qualified candidate-admission and Pareto-ranking layer. To outperform SOTA, the next research increment must implement a separate estimator/selector adapter and compare it head-to-head on the same frozen candidate space. A green selector test is not evidence that the chosen design is statistically optimal.

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

- NASA-STD-7009B: requires validation-domain records and assessment of empirical data acquisition/processing with the same rigor as the model validation; intended-use acceptance criteria are set by the programme/project and approved by delegated technical authority.
- NASA-HDBK-7009B (3 February 2026) is the companion implementation handbook, not a separate technical standard or a replacement for discipline-specific acceptance criteria.
- NIST AMS 100-85 (5 October 2026) harmonizes ASME VVUQ with JCGM metrology and proposes risk-scaled credibility assessment based on model influence and decision consequence.
- AIAA R-154-2021: model-based reduction of flight testing and six engineering tasks.
- ASTM F3705-25 / F3442-26: DAA test and performance comparison only for matched operating assumptions.
- NASA NESC-RP-23-01853: cross-simulation comparison under common inputs can contribute to validation, while remaining distinct from empirical flight validation.
- NASA 2025 strategic path management flight evaluation: two helicopters in a live-virtual-constructive environment with up to 330 virtual aircraft; a useful scale comparator, not universal safety evidence.
- NASA NESC 6DOF check-case programme: standardized common inputs and cross-simulation comparisons can identify implementation defects and contribute to model credibility, but remain distinct from physical empirical validation.

## Claim ceiling

A passing result qualifies the selector only for the supplied synthetic candidate space, frozen policy, thresholds, contrast/utility estimates and separate resource budgets. It does not verify those estimates, authenticate owner gate receipts, prove a candidate model true, authorize a physical test, establish aircraft safety, or demonstrate certification/regulatory compliance.


### Semantic boundary clarification

Controlled flight always requires external authorization; the frozen campaign policy has an explicit `authorization_required_modalities` set and can require it for HIL or other modalities too. For model-form discrimination, the common scenario/measurement contract and explicit pairwise prediction contrasts must match, but competing model outputs are expected to differ; implementation-equivalence receipts are required specifically for cross-implementation comparisons, not as a blanket requirement for physical model hypotheses. The Pareto result's model-family-pair count uses the exact same policy separation floor used for admission.
