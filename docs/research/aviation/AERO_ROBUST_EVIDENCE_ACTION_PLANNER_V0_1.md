# AERO Robust Evidence-Action Planner v0.1

**Status:** prospective finite ambiguity-set sensitivity evaluator  
**Authority ceiling:** investigative recommendation only; no claim mutation, engineering acceptance, physical-work authorization, airworthiness or certification  
**Relationship:** extension of the integer point-EVSI kernel in `evidence_action_evsi_v0_1.rs`  
**Verification:** synthetic Rust tests and structural/oracle checks are prospective; exact-head compile, tests and Clippy remain authoritative

## 1. Objective and precise scope

The point-EVSI planner ranks a permitted evidence acquisition under one prior and one owner-calibrated outcome model. That ranking can be fragile when plausible priors, calibration models or likelihood tables disagree.

This v0.1 extension evaluates a finite, explicitly approved ambiguity set:

[
\Theta = \{\theta_1, \theta_2, \ldots, \theta_K\}
]

Each scenario contains a coherent point-EVSI input for the same claim, configuration, frozen assurance state, unresolved-obligation manifest and complete action-catalogue snapshot. Its prior, decision-model commitment, outcome-model identity and action-conditioned likelihood tables may differ, but each must be individually validated by the point-EVSI kernel and owner/calibration receipts.

For action (a), calculate within every scenario:

[
NB(a,\theta)=EVSI(a,\theta)-C(a)-D(a)
]

Then report the exact finite-scenario bounds:

[
NB_{\min}(a)=\min_{\theta\in\Theta} NB(a,\theta),\qquad
NB_{\max}(a)=\max_{\theta\in\Theta} NB(a,\theta)
]

and likewise (EVSI_{\min}) and (EVSI_{\max}). Each scenario-specific EVSI/net-benefit numerator is retained in the output, not hidden behind the bounds.

**Terminology boundary:** this is finite lower-envelope sensitivity analysis under a declared scenario set. It is not a proof of optimality over a continuous distribution family, not automatically distributionally robust Bayesian decision theory, and not a general worst-case policy under unknown model identity.

## 2. Recommendation and abstention rule

An action is eligible for robust ranking only when the point-EVSI kernel admits it in every scenario and its owner permission, action contract, model owner and calibration receipt pass in each scenario.

Recommend an action only where its lower net-benefit bound is strictly positive:

[
NB_{\min}(a)>0
]

Among actions that meet this condition, select the one maximizing the lower net-benefit bound; ties are resolved deterministically by action ID. If no common action is qualified, abstain. If actions are qualified but none has a positive lower bound, report `NO_ROBUST_POSITIVE_NET_BENEFIT`.

A positive point estimate, high entropy reduction, cheap cost or favorable model cannot compensate for failed source/authority/observer/assurance hard gates. This planner cannot modify the frozen claim disposition or authorize a physical activity.

## 3. Scenario and approval commitments

The scenario-set manifest commits to the sorted scenario IDs and, for every scenario:
- decision-model digest (including prior distribution and utility matrix);
- outcome-model digest;
- action-catalogue cut digest;
- unresolved-obligation manifest digest;
- every action ID and outcome-model owner;
- calibration owner, calibration report/profile and exact outcome-likelihood-table digest.

A separate owner/authority approval receipt binds the scenario-set digest, scenario count, claim, decision context, configuration and receiver-time/effectivity interval. The point-EVSI inputs independently require source-state and complete action-catalogue snapshot receipts.

The evaluator rejects duplicate scenario IDs, fewer than two scenarios, inconsistent claim/state/catalogue context, missing/unverified/late/inactive scenario approval, and any changed scenario commitment without a new approval receipt.

The receipt's `VERIFIED` value is an upstream disposition. The pure robust kernel checks scope and chronology, but does not itself authenticate signatures or trust roots. Production integration must supply receipts already authenticated by the appropriate owner/trust verifier.

## 4. Exact output

For each common eligible action, the result preserves:
- lower/upper EVSI numerators;
- lower/upper net-benefit numerators;
- worst- and best-net-benefit scenario IDs;
- per-scenario exact EVSI and net-benefit outputs;
- per-scenario calibration receipt and likelihood-table digests;
- positive-lower-bound status and deterministic robust rank.

Scenario-specific exclusions are retained with their scenario ID and reason. No action excluded or uncalibrated in even one approved scenario is promoted into the robust common set.

## 4A. Uncertainty sources covered and deliberately excluded

This v0.1 finite scenario set varies the declared prior/decision-model commitment and action-conditioned outcome likelihood/calibration commitments. It holds the exact action catalogue fixed across scenarios. Consequently, the exact action-entry commitment—and therefore direct acquisition cost, delay cost, expected artifact contract, owner and target obligation set—remains common across the ambiguity set.

The reported lower/upper net-benefit interval therefore captures **prior and outcome-model sensitivity with fixed declared action costs**. It does not yet capture uncertainty in acquisition price, schedule delay, resource availability, owner response, evidence admissibility probability, correlated multi-action costs or sequential acquisition dependencies. Those require a later versioned catalogue/scenario model that can vary cost and delivery assumptions under separately approved commitments without allowing stale authorization reuse.

A scenario count is not a substitute for calibration quality or independent episode count. Several plausible model variants derived from one calibration defect remain one root-cause family for benchmark statistics.

## 5. Benchmark and hostile cases

The Rust test corpus includes:
- positive lower-bound control (two approved scenarios with the same positive result);
- positive point EVSI but a zero-information scenario, forcing a non-positive lower bound and abstention;
- late scenario-set approval receipt;
- changed scenario commitment without a renewed approval;
- scenario order permutation preserving the complete output.

The v0.4 benchmark extension (#7899) should additionally compare:
- point EVSI;
- finite lower-envelope EVSI;
- maximum expected information gain;
- lowest-cost / fixed-rule policies;
- expert action selection;
- LLM-only ranking;
- full Symthaea robust recommendation.

Report realized decision regret, sensitivity-driven rank reversal, lower/upper net benefit, calibration, cost, delay, admissible-evidence yield, abstention and expert overrides. Scenarios derived from one calibration defect or root-cause family do not become independent statistical episodes merely because many variants are generated.

## 6. Limitations and non-claims

- A finite scenario set can omit important tails; the receipt/manifest does not prove completeness of the uncertainty universe.
- Correlated evidence outcomes can double-count information unless the scenario model encodes their dependence correctly.
- The utility/loss model and risk preferences are owner/authority assumptions, not discovered by EVSI.
- A lower bound is only as defensible as the approved scenario set and calibration evidence.
- Passing software tests does not establish physical truth, model validity, engineering safety, test authorization, airworthiness or certification.
- The planner is a sensitivity and investigation aid. It is not a safety score or assurance disposition engine.

## 7. Dependencies

- #7882 — authority-bounded evidence-action planning.
- #7900 — finite ambiguity-aware EVSI specification.
- #7896 — point-EVSI kernel, action receipts and source-state/catalogue commitments.
- #7881 — source-bound claim and unresolved-obligation adapter.
- #7899 — separately preregistered benchmark v0.4 extension.

