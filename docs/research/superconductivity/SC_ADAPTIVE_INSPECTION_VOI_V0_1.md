# REBCO adaptive inspection: one-step value-of-information auditor v0.1

**Parent:** [SC-ADAPTIVE-INSPECTION-001 #6583](https://github.com/Luminous-Dynamics/symtropy/issues/6583)  
**Implementation branch:** `research/sc-adaptive-inspection-voi-v0.1`  
**Base:** condition-profile qualification branch `research/sc-ic-condition-profiles-v0.1`

## Why this is a useful next increment

The spatial auditor now distinguishes direct transport, calibrated non-contact proxy, inference and unmeasured regions, and partitions (I_c) coverage by exact test-condition profile. This child adds the decision layer: when inspection time or cost is limited, which single measurement should be requested next to reduce application-specific decision risk?

It composes the condition-profile work in #6602 and the calibration / VVUQ owners. It does not operate equipment or release conductor.

## Model

The fixture represents a prior ensemble of complete reel weak-cell maps. Scenario weights sum to one. Correlation across locations is represented by joint scenario patterns rather than assuming independent cells.

The application profile declares that a reel is at risk when a contiguous weak run reaches `minimum_contiguous_weak_cells`. A few isolated weak cells do not automatically count as the same failure as a long contiguous defect.

For posterior failure probability (p_f), false-accept loss (L_{FA}) and false-reject loss (L_{FR}), the minimum expected decision loss is:

`L(p_f) = min(L_FA * p_f, L_FR * (1 - p_f))`

For candidate action (a) with positive / negative outcomes:

`VOI_gross(a) = L_before - [P(+) L_after(+) + P(-) L_after(-)]`

`VOI_net(a) = VOI_gross(a) - measurement_cost(a)`

The policy selects at most one unused action per call, only when its net VOI exceeds the preregistered stop threshold and it fits within the remaining budget. After a physical inspection, the observed positive/negative result is appended to `observed_results` and the policy is rerun. This is a one-step greedy policy, not a claim of global optimality for the multi-step problem.

## Evidence and condition gates

- Every candidate action must match the exact application `condition_profile_id`. Unlike temperatures, field magnitudes/orientations or current criteria cannot be pooled.
- Direct transport requires `calibration_status=valid`.
- Non-contact magnetization proxy requires `calibration_status=qualified` and a validation artifact digest.
- Proxy and direct methods retain separate method identity; sensitivity/specificity are method- and profile-specific inputs that must be experimentally justified before using real data.
- Already-observed actions are excluded from the next recommendation.
- Prior observations update the scenario ensemble by Bayes' rule.
- If an observed outcome has zero likelihood under the declared prior, the audit fails closed rather than silently altering the model.
- Candidate costs and prior inspection costs are charged against a fixed budget.
- No physical hidden-truth reel map is included in this policy input; the policy sees only its declared prior ensemble and measurement model.

## Run

```bash
cargo test --locked -p rebco-long-length-auditor
cargo run --locked --quiet -p rebco-long-length-auditor --bin adaptive_inspection_voi -- \
  docs/research/fixtures/rebco_adaptive_inspection_voi_synthetic_v0_1.json
```

## Baselines and benchmark progression

At matched conditions, scenario prior, measurement characteristics and total budget, compare:
1. fixed/uniform spacing;
2. random feasible action selection;
3. lowest-cost action first;
4. maximum local posterior variance;
5. highest posterior weak probability;
6. one-step decision-risk VOI (this implementation);
7. robust VOI under sensitivity/specificity uncertainty intervals;
8. a full-horizon dynamic-programming oracle on small synthetic grids.

Metrics should include false-accept risk, false-reject rate, expected decision loss, net VOI, inspection cost, weak-run miss probability, coverage by evidence channel, regret to the oracle and actions per unit of accepted kA·m. Use complete held-out reel scenarios / weak-run morphologies; do not random-split adjacent cells.

## Claim ceiling

The committed fixture is synthetic. Its sensitivity/specificity, losses, priors, cell geometry and measurement costs are illustrative test sentinels, not manufacturer specifications or validated product limits. The receipt is a decision-support recommendation only. It is not a product pass/fail, a physical inspection, an application guarantee, a manufacturing recipe or equipment authorization. A real deployment requires rights-cleared paired data, calibrated detection characteristics, a documented condition profile and application-owned loss/risk limits.
