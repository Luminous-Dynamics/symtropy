# WPT health-aware weighted proportional-fair allocator v0

**Status:** single-interval synthetic service-allocation oracle, stacked on the health eligibility contract  
**Parent:** [#3559 robust dispatch benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/3559)  
**Health gate:** [#3794](https://github.com/Luminous-Dynamics/symtropy/pull/3794)  
**Research task:** [#3793](https://github.com/Luminous-Dynamics/symtropy/issues/3793)

## Purpose

This crate implements a transparent baseline to compare against equal split, round-robin, greedy efficiency, proportional fairness and eventually robust MPC. It allocates one supervisory power interval across multiple receivers using a weighted proportional-fair objective:

`maximize sum_i w_i log(p_i / requested_i)`, with `w_i = priority_i * urgency_i * requested_i`.

subject to the aggregate source budget, per-receiver health-derated capacity and per-receiver minimum service floor.

The optimizer uses KKT water-filling:
`p_i(lambda) = clamp(w_i / lambda, minimum_i, capacity_i)`.
A bounded deterministic bisection finds the multiplier. For this one-interval convex problem, the returned allocation is optimal to the stated numerical tolerance; this is not a claim of optimality for multi-period dynamics, network AC power flow, thermal ageing or robust uncertainty.

## Health receipt integration

Every receiver embeds a complete `wpt-health-dispatch-eligibility-v0` receipt and is passed through `wpt-health-dispatch-contract`.

- `DecisionSupportEligible`: eligible up to requested power.
- `PolicyRiskExceeded` + soft derate: eligible up to requested power times the operator-declared risk derating factor.
- `PolicyRiskExceeded` + hard risk cap: receiver excluded from this planned horizon.
- `SyntheticOnlyNotEligible`, `NotEvaluable`, `OutOfDomain`, `HardLimitViolation`, or asset/segment/route identity mismatch: receiver receives zero.

A soft risk derating that falls below the receiver's declared minimum service floor makes the episode `InfeasibleServiceFloors`; the allocator does not silently discard a minimum-service obligation. A global PCC or stability failure blocks every receiver.

## Frozen synthetic example

Three receivers request 8, 10 and 6 kW. Their health receipts are synthetic; the unit tests promote in-memory copies to measured-style solely to exercise the allocation mathematics. The third receiver is outside its validated domain and is excluded. The second exceeds the 5% target-event CIF budget and is capped at 50% of requested power. The first two have minimum service floors of 2 kW each.

With an 8 kW source budget, base priority×urgency weights are 1 and 4; effective log-utility weights are 8 and 40 after multiplying by requested kW. The KKT solution is:
- vehicle-01: 3 kW;
- vehicle-02: 5 kW;
- vehicle-03: 0 kW (out of domain).

This saturates the 8 kW source envelope and respects the 5 kW health-derating cap and both 2 kW service floors. It is a synthetic arithmetic control, not a measured dispatch result.

## Reported scoreboards

- allocated/requested/unmet kW per receiver;
- eligibility state, cap, reason and service ratio;
- total source capacity and total allocated power;
- eligible and all-receiver Jain service-ratio indices;
- weighted log-service objective;
- explicit status: `Allocated`, `NoEligibleReceivers`, `InfeasibleServiceFloors`, `GlobalHardLimitFailure`.

No scalar objective can compensate for a hard-limit failure, unsupported risk input or missed minimum-service floor. Equal priority and urgency produce equal delivered/requested ratios when neither receiver is capped.

## Validation

13 Rust unit tests cover synthetic-only rejection, health/domain exclusion, soft risk derating, hard risk caps, KKT allocation, equal normalized service for equal priority/urgency, minimum-floor infeasibility, global PCC/stability hard gates, identity mismatch and fail-closed input validation.

## Limitations and next step

- Single interval only; no inter-temporal storage, rolling forecast, trajectory, ramp or handover state.
- No AC power-flow or feeder constraints beyond a global source envelope and pass flag.
- No robust uncertainty sets or recourse. Next version should compare deterministic allocation against rolling conformal robust dispatch with recourse and individual departure-energy constraints.
- No hardware energization, live PCC operation or standards certification.

## Run

`cargo test --locked -p wpt-proportional-fair-dispatch`
