# Orbital Truss Inspectability — Benchmark and SOTA Roadmap v0.3

**Research date:** 23 November 2026  
**Current verified implementation:** synthetic 2D centre-feature geometry and exact <=20-view planning; nominal k-coverage selection baseline.  
**New increment:** 3D enclosing-ball support sensing-envelope certificates, with occlusion explicitly NOT_ASSESSED.  
**No claim:** physical sensor calibration, NDE/POD, structural acceptance or flight readiness.

## 1. Paradigm shift: from centre-visible to support-certified

The 2026 set-multicover paper explicitly models visibility at voxel centres and calls this a discrete approximation, not a guarantee over the whole voxel surface. That is a clearly stated limitation in the paper's model. Run two models side by side:

1. Paper-faithful centre-sampled model (#7002), preserving its exact candidate generation, visibility and k-coverage semantics.
2. Support-envelope extension (#7136), bounding the entire declared support in distance, FOV, incidence and GSD, and refusing to call it full visibility until support-wide occlusion is also assessed.

The goal is to quantify when centre sampling is adequate, when it misses boundary failures, and how much conservative abstention costs.

## 2. Current state and evidence

The preceding #6925 exact head was c2d94e06da4665200e0ca1fd802aad5d44e09ec9. Its four checks passed: dedicated inspectability workflow and repository CI on Ubuntu, Windows and macOS. It remains a draft PR. The exact planner and independent Python oracle comparisons cover synthetic 2D candidate matrices.

The new support-envelope increment is separate and remains synthetic. Its upper/lower bounds use fixed-point output units to make Rust and Python receipts exactly comparable. CERTIFIED_WITHIN_SENSING_ENVELOPE never implies unoccluded visibility; every receipt says occlusion_status=NOT_ASSESSED and may_count_toward_full_support_coverage=false.

## 3. Research comparison

### Centre-sampled GSD k-coverage

Bin Khalid and Yeum, arXiv:2610.36450, use voxel-centre observation geometry, outward face normals, candidate viewpoint generation and augmentation, and explicit set multi-cover. The paper's full text says the centre-based test is a discrete approximation rather than a guarantee over the whole voxel surface. It reports a 3.00–16.39% viewpoint-count difference against its ILP comparisons; the gap is meaningful only on the same generated candidate pools and objective.

Source: https://arxiv.org/abs/2610.36450

### Full-stack multi-robot 3D inspection

Pšenička and Edelkamp, arXiv:2610.26106, combine candidate generation, GPU visibility, set cover, min-max route MIP, continuous-time MAPF, trajectory smoothing and post-smoothing collision/waypoint audits over 14 meshes up to 7.2M faces. Their paper explicitly notes the decomposition sacrifices global optimality and there is no shared multi-robot 3D-inspection benchmark. One of the 14 is the real underwater shipwreck; the other 13 are topology/resolution stress shapes rather than underwater field validation.

Source: https://arxiv.org/abs/2610.26106  
Code/results: https://github.com/krystofpsenicka/3D-Inspection

## 4. Benchmark ladder

| Stage | Benchmark | Primary comparator | Evidence |
|---|---|---|---|
| P0 | Contract semantics | frozen controls, independent oracle | exact receipts |
| P1 | Candidate-pool quality | source-faithful GSD candidate generation | pinned models, SBW and visibility hashes |
| P2 | Centre k-selection | paper greedy, overlap pruning, certified ILP | same candidate matrix |
| P3 | Support envelope | centre-only vs enclosing-ball support bounds | false-envelope/abstention/bound-tightness |
| P4 | Support occlusion | ray cone/visibility volume vs independent triangle ray tests | false-clear, false-block, partial/unknown |
| P5 | 3D structure | mesh/datum/support identity and multi-normal tests | independent geometry oracle |
| P6 | Reachability/fleet | #5920/#5892 | route, energy, charging, collision and return reserve |
| P7 | Actual observation | #6752/#4258 | instrument-specific observation/noise validation |
| P8 | Decision and evidence | #6956/#2239/#4997 plus Mycelix | non-anticipative decisions and identity/evidence custody |
| P9 | Physical NDE | qualified procedure and independent reference truth | method-specific POD/uncertainty study |

Metrics remain separate: requested-k attainment; attainable capped-target attainment; support-certified envelope fraction; occlusion proof/unknown/partial fractions; GSD upper bound vs dense sample maximum; FOV/incidence margin; false-clear/false-block; candidate generation/selection/runtime/memory; route feasibility; instrument calibration; and decision-level false clearances.

## 5. Prospective targets

- Zero false CERTIFIED_WITHIN_SENSING_ENVELOPE cases in frozen adversarial controls.
- Zero false-clear occlusion cases before any full-support visibility claim.
- Exact independent receipt agreement for every frozen support fixture.
- Rigid-frame invariance and monotonicity under support-radius/normal-cone enlargement.
- Publish the indeterminate and false-block costs; do not weaken bounds to improve apparent coverage.
- Exact/ILP selection gap zero for certified optima on tractable candidate pools; larger heuristic targets remain prospective and require matched baselines.
- No physical detection/POD claim without instrument- and procedure-specific validation.

## 6. Ownership and interfaces

- #7002: source-faithful centre-sampled GSD/k-coverage reproduction.
- #6925: candidate-view optimization, exact nominal k oracle and robust/domain-aware planning.
- #7136: support-wide range/FOV/incidence/GSD and occlusion certification.
- #6846: datum registration and covariance-aware metrology.
- #5920/#5892: route/fleet feasibility.
- #6752/#4258: observation operators and common-cause measurement models.
- #6956/#2239/#1892/#4997: active measurement and non-anticipative decision semantics.
- Mycelix #6390/#6519: AP242/QIF identity, evidence custody and authority binding.

The intended end-to-end chain is candidate generation -> support certificate -> multi-view selection -> reachable inspection plan -> acquired observation -> calibrated interpretation -> independent disposition. A downstream stage never inherits a result that an upstream stage did not prove.
