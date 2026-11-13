# Investigation Decision-Value and Measurement-Policy Benchmark v0.1

**Workstream:** INVESTIGATION-DECISION-VALUE-QUAL-001 (#5032)  
**Programme:** SUF #4910  
**Status:** synthetic policy benchmark proposal; no deployed decision agent  
**Date:** 2026-11-13

## 1. Objective

Evaluate the complete loop from an event/coverage/evidence state to a recommended next measurement, survey, human review, field verification or no-action decision.

The policy objective is not maximum entropy reduction, maximum alarm count or maximum sensor utilization. It is lower expected and realized decision loss under a declared resource budget, subject to critical-risk, evidence, owner-authority and safety constraints.

## 2. Candidate policies

- P0 conventional/no additional sensing baseline;
- P1 fixed calendar inspection;
- P2 threshold alarm → investigation;
- P3 fixed risk/severity ranking;
- P4 greedy blind/degraded spatial coverage;
- P5 expected information gain (EIG) ranking;
- P6 expected value of sample information (EVSI) ranking with defensible loss unit;
- P7 bounded hybrid: critical-risk hard rules + EVSI ranking + abstention + human review;
- P8 random feasible action under equal budget.

P5 and P6 are explicitly different. Entropy reduction can be valuable for knowledge but have zero value for the current decision.

## 3. Candidate action menu

Survey a new tie point; tap-test/controlled excitation on non-live assets; request an independent vehicle trajectory; retrieve another waveform window; inspect a splice/endpoint; deploy a temporary point sensor; repeat after clock correction; request human engineering review; defer/no action.

A policy only proposes actions. Owner permission, permits, safe work method and human decision remain external gates.

## 4. Frozen comparison contract

All policies receive the same:
- source observations and data-quality states;
- route/fibre topology, calibration freshness and uncertainty;
- independent truth and event/no-event exposure;
- feasible action menu and action outcome likelihoods;
- initial resource/time/budget envelope;
- loss matrix and unit declaration;
- hard critical-event requirements;
- test scenarios and grouped train/validation/test split.

Policies may not receive future truth, post-action outcome or human adjudication labels before choosing an action.

## 5. Metrics

**Decision quality:** expected loss before action, expected posterior loss, EVSI, realized regret, critical misses and false dispatch.

**Information:** EIG in bits, posterior uncertainty, hypothesis count, ambiguity preservation. Report separately from decision value.

**Operational burden:** survey/technician minutes, human review minutes, queue age, action success/abandonment and budget utilization.

**Efficiency:** cost per independently accepted actionable event, EVSI per action when units permit, decision-loss reduction per resource, repeat visits and avoidable dispatch.

**Safety and rights:** infeasible/unauthorized action rate (target zero), critical safeguard violations (target zero), permission/owner refusal handling, abstention quality and post-action evidence completeness.

## 6. Non-negotiable controls

- The P0 conventional baseline must remain available and can win.
- Critical-risk safeguards are hard constraints and cannot be outweighed by lower cost or higher EIG.
- An action without owner authority, data rights, permit or safe execution envelope is infeasible.
- If decision loss and acquisition cost are in incomparable units, do not compute net value or rank by net value.
- Human review/crew capacity, travel, revisit, retrieval and false dispatch belong in the resource model.
- Unknown/noisy prior probabilities are sensitivity-tested; do not treat model likelihoods as ground truth.
- EIG and EVSI are reported independently.
- An action with positive EIG and zero EVSI should not be selected solely because it increases information.
- Evaluate the policy on held-out routes/sites/time periods/events.
- No live fault induction or unsafe action is part of this benchmark.

## 7. Proposed targets (not achieved results)

- Zero unauthorized/infeasible action recommendations in the finite negative-control corpus.
- Zero critical safeguard violations under all policies.
- P7 improves realized loss or cost per accepted actionable event relative to P1/P2/P3 at matched critical miss and false-dispatch constraints; otherwise the simpler policy wins.
- P6 never pays for a measurement whose declared EVSI is zero unless a separate mandatory safety/rights reason exists.
- P5 is never reported as an economic winner without a compatible utility/loss model.
- Report Pareto fronts when cost, delay, risk and human burden do not share a common unit.
- Abstention reduces risk under OOD/conflicting/insufficient evidence without hiding the amount of uninvestigated route.

## 8. Claim ceiling

This is a benchmark proposal and synthetic test plan. It does not establish a working adaptive policy, actual field efficiency, reduced costs, better detection or autonomous authority.
