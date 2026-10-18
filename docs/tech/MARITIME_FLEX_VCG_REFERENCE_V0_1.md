# Bounded VCG Mechanism for Consented Maritime Flexibility v0.1

**Status:** research/reference implementation proposal; Rust checks are not yet verified.  
**Code:** `crates/domains/symtropy-transport-optimization/src/maritime_flex_mechanism.rs`  
**Tracker:** [MARITIME-MECHANISM-001 #1917](https://github.com/Luminous-Dynamics/symtropy/issues/1917)  
**Authority boundary:** [Mycelix LOG-FLEX-001 #5185](https://github.com/Luminous-Dynamics/mycelix/issues/5185)

## 1. Why mechanism design is the next paradigm shift

A planner may find an efficient schedule using declared flex offers, yet fail if participants can profit by exaggerating their private delay/compensation cost or if payments are not affordable. Total-system surplus, incentive compatibility, budget balance, and distributional fairness are separate properties; one does not prove the others.

The 2026 research literature addresses vessel demand shifting and capacity use, strategic berth pricing/equity, and coordinated multi-port energy schedules with fair-profit allocation:
- Lei & Jia, *An integrated framework of vessel demand shifting and port capacity utilization for congestion mitigation*, Transportation Research Part E 208 (2026): https://doi.org/10.1016/j.tre.2025.104660
- Guo, Zheng & Du, *Pricing and equity in strategic berth allocation problem considering general and dedicated service modes*, Transportation Research Part E 205 (2026): https://doi.org/10.1016/j.tre.2025.104477
- *Two-stage coordinated transportation-energy scheduling with fair profit allocation in multi-port integrated energy systems*, Energy 347 (2026): https://doi.org/10.1016/j.energy.2026.140058

These publications motivate testing incentives and distributions; their results do not substantiate performance for this implementation.

## 2. Restricted finite model

Let each eligible offer (i) have:
- (g_i \ge 0): externally declared gross operational benefit;
- (c_i \ge 0): participant-reported private disutility/compensation cost;
- a demand, baseline time slot, target slot, quantity, maximum shift, hard deadline, expiry and acceptance timestamp.

Choose a binary selection (x_i \in \{0,1\}) that maximizes declared surplus:

\[
\max_x W(x)=\sum_i (g_i-c_i)x_i
\]

subject to:
- time-slot capacity after every selected move;
- at most one selected move per demand;
- one candidate offer per participant in v0.1;
- a currently valid conditional acceptance known at the decision cutoff;
- shift bounds, hard deadline and expiry checks.

Benefit (g_i) is an external model input, not a participant-reported field. Its evidence reference is preserved, but this module does not authenticate that evidence. The implementation requires a capacity-feasible baseline and discrete non-overlapping time buckets per physical resource; overlapping same-resource bucket definitions fail closed. It does not simulate continuous berthing dynamics, multi-resource service duration, or physical navigation.

## 3. Pivot payment rule

For the selected allocation (x^*), let (W_{-i}^*) be the highest declared surplus obtainable when offer (i) is removed. For selected (i), the recommended pivot compensation is:

\[
p_i = \sum_j g_jx_j^* - \sum_{j\ne i}c_jx_j^* - W_{-i}^*
\]

Unselected offers receive zero in this reference model. The independent test oracle recomputes the small allocation and counterfactual welfare without calling the production subset search/payment helpers.

Example: two participants each offer a shift with gross benefit 10; their declared costs are 2 and 4; one target-slot capacity unit is available. The optimizer selects the offer with cost 2 (surplus 8). Without that participant, the alternative surplus is 6, so the selected participant's pivot payment is 4. This preserves the modeled dominant-strategy incentive in this simple competition. If the buyer budget is only 3, the report flags a shortfall of 1—it does not pretend the plan is affordable or silently change the allocation.

## 4. Assumptions and claim ceiling

The intended truthfulness property applies only under the declared assumptions:
- single-parameter private cost per participant (one candidate offer per participant);
- quasi-linear participant utility: payment received minus true private shift cost when selected, zero otherwise;
- fixed, additive operational benefits independent of reports;
- a complete finite offer set and exact welfare maximization;
- no unmodeled participant externalities beyond the shared-capacity feasibility constraints.

This is not a universal claim about actual shipping markets. Multiple offers per participant, private/strategic estimates of gross benefits, budget-constrained allocation, combinatorial contracts, participation dynamics and distributional policy changes require separate analysis. VCG does not automatically guarantee strong budget balance or a fair distribution. Buyer budget shortfall is reported as a diagnostic; the budget is not imposed as a post-selection veto because that may invalidate the incentive property.

The solver only selects a **proposed** allocation. It cannot accept/activate offers, reserve a berth, change arrival times, amend contracts, authorize a payment or settle compensation. Mycelix remains the owner of those lifecycle/effect transitions.

## 5. Frozen correctness campaign

The Rust module includes a test-only bit-mask oracle and cases for:
- allocation and pivot-payment agreement;
- offer not accepted, revoked or expired;
- missing/future acceptance receipt;
- shift limit and deadline violations;
- overlapping time buckets aliasing one physical resource;
- baseline capacity overrun;
- buyer-budget shortfall without a hidden allocation change;
- exhaustive unilateral bid reports over a finite two-participant cost grid;
- canonical input permutation;
- search-budget exhaustion without a partial report;
- duplicate candidate offers from one participant.

The exhaustive truthfulness grid uses true costs and unilateral reports from 0 through 12 for each of two participants in a one-unit target-slot competition. A separate JavaScript check of the same simple single-slot payment algebra covered 4,394 unilateral-report comparisons with zero profitable deviations. That independent arithmetic check is not execution of the Rust source.

## 6. Proposed qualification gates

- 100% agreement with an independently written exhaustive oracle on all frozen tiny feasible offer spaces.
- 100% dominant-strategy truthfulness in the restricted declared grid and in all frozen finite cases; any counterexample is a defect, not a benchmark exception.
- Zero hard-feasibility, consent, expiry, deadline or capacity violations.
- Byte-identical allocation, counterfactual welfare, payments and receipts across canonical input permutations.
- Search exhaustion returns an explicit error, never a partial final allocation/payment.
- Report social surplus, buyer spend, compensation total, budget shortfall, participant utility, participant-level displacement, concentration and runtime separately.
- Keep budget balance, individual rationality, truthfulness and distributional fairness as separate test dimensions.
- Exact-head Rust format/tests/Clippy evidence before qualification. Missing/queued/skipped checks are not passes.

