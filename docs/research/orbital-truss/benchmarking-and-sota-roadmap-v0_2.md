# Orbital Truss Inspectability — Benchmark and SOTA Roadmap v0.2

**Status:** research and benchmark protocol; candidate-comparison work not yet reproduced.
**Current implementation:** synthetic 2D geometric observability and exact small-instance viewpoint selection.
**Latest v0.2 change:** separate per-feature k-view count from declared failure-domain diversity; requested k is not silently capped by candidate availability.
**Claim ceiling:** no 3D inspection, physical measurement, NDE/POD, structural acceptance or flight qualification.

## 1. Paradigm shift: design for independently verifiable state

Do not optimize visibility or coverage as an isolated score. The design objective is to make critical physical properties inspectable, measurable, independently challengeable, repairable and re-verifiable throughout the lifecycle.

The assurance chain is:

design identity -> inspection requirement -> candidate viewpoint -> reachable acquisition -> measured observation -> calibrated interpretation -> independent adjudication -> engineering decision -> authorized disposition -> reinspection

The semantic boundaries are non-negotiable:

- geometric visibility is not acquisition;
- multiple views are not necessarily independent evidence;
- acquired data are not automatically valid measurements;
- measurement coverage is not defect detection;
- a detected indication is not a diagnosis;
- a diagnosis is not structural acceptance;
- a synthetic receipt is not physical or flight qualification.

## 2. Current v0.2 implementation

Public draft: [Symtropy PR #6925](https://github.com/Luminous-Dynamics/symtropy/pull/6925).

The planner now has two separate coverage constraints for each feature:

- `required_view_count`: minimum number of distinct candidate view IDs that must robustly observe the feature;
- `required_failure_domains`: minimum number of distinct declared failure-domain IDs represented among the observing views.

The receipt exposes nominal and robust counts for both dimensions, per selected solution and for the entire candidate pool. A requested k is not silently reduced when the candidate pool is too small. For example, a requested k=3 with only two candidate views is reported infeasible with an explicit 2/3 candidate-availability record.

Planning corpus v0.2 has eight synthetic cases: greedy trap, budget infeasibility, domain collapse, robustness abstention, cost/count Pareto, k-view coverage within one declared domain, domain diversity with a one-view demand, and an under-provisioned k-coverage candidate pool.

The exact solver is bounded to 20 candidate views and exhaustively enumerates 1,048,576 subsets at that limit. A strict independent Python oracle reconstructs the same coverage records, optimum, Pareto frontier and greedy baselines. This is an exact small-instance reference, not a scalable solver or a 3D candidate generator.

## 3. Direct external comparator: GSD-constrained k-coverage

Bin Khalid and Yeum, *Ground Sample Distance Constrained k-Coverage Viewpoint Planning via Set Multi-Cover Formulation*, arXiv:2610.36450, submitted 27 October 2026:

https://arxiv.org/abs/2610.36450

The paper defines per-surface-voxel k-viewpoint coverage under a geometric image-quality model. Candidate viewpoints are generated along outward surface directions; candidates are screened by distance, viewing angle, field of view and occlusion; under-covered voxels receive additional candidates. The selection method is a set multi-cover problem.

Reported experiment settings include two public 3D models, voxel sizes 0.04/0.06/0.08/0.10 m, maximum GSD 0.5 mm/pixel, a 0.25 mm/pixel setpoint, k values 1/3/5, overlap thresholds 50%/70%, augmentation tilt 1–90 degrees and azimuth 0–360 degrees. The ILP time limit is 3,600 seconds.

The authors compare a dynamically weighted deficit/scarcity greedy method with sequential overlap pruning and ILP on the same candidate pools. The greedy method uses 3.00–16.39% more viewpoints than the reported ILP results across the tested configurations. The paper explicitly states that ILP viewpoint counts are proven minima only when optimality is certified. It also reports a small number of configurations with 99.93–99.96% requested coverage because the candidate pool could not supply the full k demand.

Important semantic detail: the algorithm sets a per-voxel attainable target k*(v)=min(k, SBW(v)), where SBW is the number of candidate viewpoints that can see voxel v. This is an explicit candidate-pool relaxation, and the paper explains the resulting shortfall rather than claiming unconditional requested-k satisfaction. Our v0.2 receipt reports both the originally requested k and the actual candidate-pool availability, keeping the unmet requirement visible and requiring a separate augmentation step.

Limitations stated by the paper: known static geometry; voxel-center sensing approximation; no minimum viewpoint separation; no UAV reachability/dynamics; no collision-safe trajectories; no downstream reconstruction-quality assessment; two structures. Therefore this paper is a direct selection comparator, not a complete flight/inspection comparator.

Paper code status: the arXiv text describes a Python/Open3D implementation and references public model assets; no dedicated author code repository was identified in the manuscript. We should seek author clarification or a permitted implementation artifact before claiming an exact code reproduction.

Primary sources: arXiv abstract and full HTML, including the method, parameter table, results and limitations.

https://arxiv.org/abs/2610.36450
https://borowiecki.dev/pdf/2610.36450

## 4. Direct full-pipeline comparator: GPU multi-robot 3D inspection

Pšenička and Edelkamp, *A GPU-Resident Pipeline for Multi-Robot 3D Inspection with Limited Perception*, arXiv:2610.26106, submitted 19 October 2026:

https://arxiv.org/abs/2610.26106

The system integrates candidate viewpoint generation, GPU visibility, set cover, min-max vehicle routing MIP, continuous-time multi-agent path finding, trajectory smoothing and post-smoothing collision/waypoint audits. It evaluates 14 meshes up to 7.2 million faces and reports lower-bound and LP-relaxation gaps.

Authors' code and per-run results are public: https://github.com/krystofpsenicka/3D-Inspection (paper points to the camera-ready/experiments branch and experiments/results/). This is a substantial, broad reference implementation, not a feature checklist to casually compare against.

The article says the pipeline's decomposition costs global optimality and notes that no shared multi-robot 3D-inspection benchmark exists. It also distinguishes one real underwater shipwreck from thirteen other non-underwater shape/scan meshes used to vary topology and resolution. Comparisons must preserve that distinction.

The repository currently advertises a high-end CUDA/OptiX/RAPIDS/Isaac Sim setup (RTX 3090, 24 GB, CUDA 12.8, Python 3.11, RAPIDS 26.2 and OptiX 8). Its repository metadata did not expose a licence file at the inspected revision. We should treat it as a reference for reproducibility and architecture, not copy its source into Symtropy or redistribute assets without permission.

Primary sources:

https://arxiv.org/abs/2610.26106
https://github.com/krystofpsenicka/3D-Inspection

## 5. Matched benchmark protocol

### B0 — contract and candidate-pool truth

Freeze one visibility matrix and candidate identities. Compare exact k coverage and independent failure-domain diversity separately. Publish per-feature requested k, nominal candidate count, robust candidate count, selected nominal count, selected robust count, and declared domain counts.

Required adversarial families: insufficient candidate alternatives; k=1/3/5; duplicate views; same-domain multi-view coverage; multiple-domain single-k demand; nominal-only coverage; budget failure; and view-order permutations.

### B1 — selection-only tournament

On identical candidate-view pools, compare:

1. exact set multi-cover optimum (small instances);
2. integer linear programming on larger instances, recording certified optimality, feasible incumbent and lower bound separately;
3. source-faithful dynamically weighted deficit/scarcity greedy (only after its pseudocode is independently transcribed);
4. ordinary marginal-gain/cost greedy;
5. sequential overlap pruning in the paper's vertical and visibility orderings.

Selection-only comparisons must not include route costs in one method but exclude them from another. Use identical target voxels, GSD screens, visibility matrix, view availability and k.

### B2 — candidate generation and sensing quality

Reproduce the paper's outward-normal candidate generation, GSD-derived sensor distance, FOV and oblique-angle limits, occlusion and targeted augmentation. Evaluate voxel-center and denser surface-sample visibility separately. Report candidate pool availability before selection so the selector cannot hide candidate-generation failures.

### B3 — robustness and metrology

Add pose, registration, calibration, resolution and geometry perturbation families. Keep view-count redundancy separate from failure-domain diversity and common-cause cut sets. A finite perturbation suite supports only finite-scenario robustness; probabilistic coverage requires calibrated distributions and a separate validation study.

### B4 — routes and fleets

Convert selected viewpoints into reachable tasks, then use existing Symtropy #5920 for the exact small joint allocation-routing-charging oracle and #5892 for faithful reproduction of the published charging-aware mine inspection baseline. Compare makespan, route distance, energy, charging queue, collisions, return reserve and plan churn independently.

### B5 — acquired measurement and decision outcome

Use #6752/#4258 for observation process, correlated noise and common causes; #6956/#2239/#1892/#4997 for safe active measurement and non-anticipative decision policies. A planned view does not create a measurement, and an observation outcome cannot depend on hidden simulator truth.

### B6 — interoperability and physical qualification

Use Mycelix #6390 for AP242/QIF and persistent identity exchange; use #6846 for covariance-aware 3D datum/metrology decisions. Physical defect-findability/POD requires a method-specific NDE study with independent reference truth. Do not infer it from visibility, GSD or view count.

## 6. Metrics and prospective targets

Report separately: requested-k attainment; capped/attainable coverage if a comparator uses it; candidate-pool shortfall; selected viewpoint count; exact/MIP optimality gap; GSD distribution and exceedance; incidence-angle distribution; occlusion and blind zones; coverage by critical feature; runtime/memory; path length/makespan/energy; collision and return-reserve violations; calibration/uncertainty; false-clearance and missed-indication rates; decision regret and post-repair verification.

Prospective targets, not achieved results:

- 100% exact agreement with an independent oracle for every <=20-view instance.
- Zero silent relaxation of a requested k; any capped target must be reported alongside the original demand and candidate shortfall.
- For larger matched candidate pools, target a median selection gap <=3% and 95th-percentile gap <=10% against certified ILP lower bounds, while reporting timeout and uncertified cases separately.
- 100% explicit distinction between geometric k-view coverage and failure-domain diversity.
- Zero hard route, collision, battery, charging or return-reserve violations in the frozen fleet suite.
- Zero physical NDE claims without method-specific calibrated tests and independent truth.

These thresholds are research goals. They must be preregistered and revised if the matched candidate pool, runtime budgets or distributions make them inappropriate.

## 7. Portfolio integration — retain specialist ownership

- #6845/#6925: orbital design-for-inspection geometry and candidate view selection.
- #1782: reachable next-best-view and energy/return-reserve logic in the forestry/grid domain; comparator only.
- #2239/#1892/#4997: active-view, decision-impact and metrology-action policy semantics.
- #4258/#6752: observation operators, correlated/common-cause sensing and stateful detectors.
- #6956: safe query outcomes and non-anticipative policies.
- #5920/#5892: joint robot routing/charging oracle and published mine-inspection reproduction.
- #6846: 3D datum and uncertainty-aware metrology decisions.
- Mycelix #6390/#6519: AP242/QIF identity, evidence custody and authority-evidence binding.

## 8. Next execution order

1. Finish exact-head qualification of the v0.2 coverage semantics and oracle.
2. Freeze a source-pinned 3D k-coverage candidate-pool fixture with rights and geometry metadata.
3. Transcribe the published greedy algorithm and overlap-pruning baselines independently; keep the exact ILP oracle as ground truth where tractable.
4. Implement GSD/voxel candidate generation as a separate 3D module with an independent visibility oracle.
5. Route candidate tasks through #5920 and preserve #5892's publication-faithful reproduction.
6. Add qualified sensor outcomes, decision policies and AP242/QIF evidence only through the existing owner interfaces.

The success criterion is not fewer selected views on a toy matrix. It is more independently verified, decision-useful inspection outcomes per lifecycle cost, under matched sensing, reachability and safety constraints, with every limitation and unknown preserved.
