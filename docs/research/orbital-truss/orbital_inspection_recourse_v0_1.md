# Orbital inspection recourse v0.1

## Research proposition

A fixed route can finish with unresolved engineering ambiguity. A better inspection campaign is contingent: after each admitted observation, it selects another admissible view, qualified reference, recalibration or abstention based only on the public history so far.

The stopping condition is decision-level identifiability, not physical-world identification. If multiple surviving worlds imply the same declared engineering decision, further observation has no decision value under that contract. If opposite-decision worlds remain, the campaign must continue with an admissible action or abstain.

## Scope and composition

## Relationship to the existing sequential decision engine

This finite enumerator is a **test oracle and contract challenger only**. It is not a second production policy engine, not an alternative sequential evidence process, and not an operational planner.

- #6835 already provides the exact finite-world decision-identifiability kernel and a decision-focused one-probe partition/ranking foundation.
- #6887 owns sequential finite-sample surviving-world sets, adaptive observation, optional-stopping validity and calibrated abstention.
- #7292 bridges the source-bound #6887 anytime-valid world-set receipt to downstream incumbent decision evaluation. Its v0.1 adapter checks source contract/inventory/cutoff structure; it does not generate or prove the upstream statistical guarantee.
- This v0.1 evaluator challenges deterministic support-level recourse semantics against exhaustive toy cases: non-anticipativity, outcome-support unions, hard gates, bounded resource use, and decision-equivalent stopping. It may serve as an independent tiny-domain reference oracle for those properties; it must not replace the #6887/#7292 execution path.

The eventual integration must consume the exact versioned #7292 source-bound survivor inventory and #6835 decision-class mapping (or approved successor interfaces), plus admissible actions from acquisition/calibration/measurement owners. Until those interfaces are explicitly pinned and reviewed, this crate's world inventory is synthetic evaluator input only. No hidden identity or synthetic action tree is to be promoted as a real robot policy.

The v0.1 executable work is a finite support-level policy evaluator. It does not duplicate statistical sequential inference. #6887 remains the owner of finite-sample world survival, adaptive observation validity and anytime-valid e-process semantics. The recourse adapter should later consume its admissible surviving-world / decision-set outputs.

Other inputs remain owned by their established workstreams:

- Acquisition and timing admissibility: #7350 / #7384.
- Stateful calibration admissibility: #7339 / #7382.
- Geometry, support, sampling, photometric and feature measurement: #7136, #7002, #7164, #7222, #7043 and #7049.
- Dependency-qualified observation-path diversity: #7341 with #4258 and Mycelix #5762.
- Motion/exposure/illumination finite co-design: #7385 / #7387 / #7388.
- Mission resource and return-reserve semantics: #5920.
- Event capture and physical HIL: #6659 / #6715.

Missing, stale, unsupported or contradictory upstream evidence must leave an action inadmissible. The evaluator must not invent an independent path merely because sensor or robot IDs differ.

## Exact v0.1 semantics

A case declares a finite evaluator-only world set, decision classes, a finite action set, per-world possible outcome support, gate receipts, cost and resource budgets. The policy tree is searched exhaustively until the state cap.

For an admissible action, every possible observed outcome yields a branch containing **all** worlds that could have produced it. This union-support rule deliberately handles overlapping outcome support without assigning a likely world. A branch can terminate only if every surviving world has the same decision class.

The objective is lexicographic: minimize worst-case elapsed time, then worst-case energy, then worst-case action count. No weighted aggregation is used. Action selection is non-anticipative because each node depends only on the currently surviving public support; hidden world identities are never policy inputs. Search-cap exhaustion is explicitly non-optimality-proven, not a best-effort optimum.

## Benchmark protocol

The frozen corpus contains:
1. one admissible observation resolves opposite decision classes;
2. overlapping outcome support requires a second, contingent observation;
3. different worlds with the same decision stop without spending resources;
4. UNKNOWN gate causes abstention;
5. a common-cause observation with identical support cannot resolve opposite decisions;
6. finite-search cap produces SEARCH_LIMIT_NOT_PROVEN;
7. resource-infeasible action causes abstention;
8. unknown outcome model cannot be treated as positive evidence.

Rust tests and a separately written standard-library Python enumeration oracle must agree on every complete receipt. The frozen schema and workflow enforce this at the exact source commit.

## Metrics

Report separately: false decision commitment, correct resolved decisions, abstention, probes per resolved decision, worst-case elapsed time/energy, unused but eligible actions, search states, policy exactness, missing/unknown mapping rejection, and replay parity. Keep physical path length, coverage, localization error, feature error, measurement uncertainty and POD as separate empirical strata when hardware data becomes admissible.

## Claim ceiling

The v0.1 evaluator proves only exact behavior over its declared finite synthetic possible-outcome support. It does not prove that the real physical world is among the declared worlds, that the physical sensor follows the support model, that an outcome was captured correctly, that measurement references are independent, or that a real action is safe. No physical POD, metrology conformity, flight readiness or acceptance authority is claimed.
