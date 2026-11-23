# Decision-aware active evidence planner

**Status:** standalone Pareto planner over synthetic, conditional outcome-set separation reports. It is not connected to a physical executor.

**Composition:** reuses the existing active-evidence acquisition candidate contract and #7098 decision-equivalence projection. It does not replace either owner.

## Why this is a separate planner view

The previous discrimination planner counts every separable pair of fault hypotheses. That can reward an action for separating two nuisance hypotheses that imply the same operating decision.

This wrapper consumes the decision-equivalence projection and counts only pairs whose hypotheses imply different navigation decisions. Same-decision ambiguity remains visible, but it does not earn acquisition value.

## Planner inputs and invariants

Every resource-feasible action must have exactly one decision-projection report bound to:
- the same complete hypothesis-inventory commitment;
- the same frozen decision-partition commitment and exact assignment map;
- an action-specific outcome-model and feature-normalization commitment;
- a report explicitly conditional on successful, valid acquisition.

The wrapper independently revalidates the report:
- complete assignment count and unique hypothesis IDs;
- nonzero hypothesis and decision-class IDs;
- exact same-class and cross-class pair counts;
- exact unordered cross-decision pair set;
- decision-class labels, finite nonnegative separation gaps and boolean consistency;
- distinguishable/ambiguous counts and disposition.

A malformed or incomplete projection is not used to rank an action.

## Pareto objectives

The planner preserves four separate objectives:
1. number of decision-incompatible pairs with guaranteed outcome-set separation;
2. signed conditional protection-level reduction;
3. worst-case action latency;
4. worst-case action energy.

There is no weighted scalar between metres, time, energy and pair separation.

If a target-reaching action exists, the policy selects the one with the greatest decision-relevant pair separation, followed by latency, energy and action ID. If no action reaches the target but a decision-relevant discriminator exists, it selects the best separation/PL/latency/energy trade-off. If no decision-relevant separation exists, a genuine positive conditional PL reduction can still be selected; same-decision parameter separation alone cannot justify a non-improving action.

A projection with zero decision-incompatible pairs is not treated as a successful sensing action.

## Outcome and authority boundary

Every separation is conditional on a successful, valid acquisition. A recommendation is not an acquired observation.

The planner:
- does not remove hypotheses;
- does not update a posterior;
- does not change a protection level;
- does not execute or retry a physical action;
- does not grant NORMAL or any other authority transition.

The #7080/#7138 attempt lifecycle, #7082 reliability admission, #6900 fault inventory and #6470 integrity audit remain separate upstream/downstream contracts.

## Synthetic tests

Tests cover:
- preferring cross-decision separation over same-decision nuisance-pair separation;
- no decision-incompatible pairs plus no PL reduction returning NoFeasibleAction;
- malformed/incomplete cross-class pair lists;
- mismatched inventory or decision partition;
- missing, duplicate and unexpected action reports;
- target-reaching selection prioritizing decision-relevant separation;
- action selection for a decision discriminator without immediate PL reduction.

## Claim ceiling

The planner only ranks caller-supplied conditional outcome-set enclosures and protection bounds under its frozen synthetic policy. It does not prove the physical outcome model, independence, source lineage, acquisition probability, decision partition or fault inventory. It is not a physical navigation, PHMI or safety qualification.
