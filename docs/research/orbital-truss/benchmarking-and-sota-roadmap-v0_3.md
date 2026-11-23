# Orbital Truss Inspectability — Benchmark and SOTA Roadmap v0.3

**Status:** research roadmap and source-aware reproduction protocol.
**Latest exact public implementation:** Symtropy PR #6925, head fe3f436f6f6abbedcf79cf869ef2fcea9b16a49f.
**Verified exact-head checks:** dedicated geometry/planning benchmark workflow and Ubuntu/Windows/macOS repository CI passed.
**Current claim ceiling:** synthetic 2D observability and exact small-instance planning. No 3D, physical measurement, NDE/POD, structural acceptance or flight qualification.

## 1. Paradigm shift: design for independently verifiable state

Optimize neither visibility nor selected-view count in isolation. Design important physical properties to be inspectable, measurable, independently challengeable, repairable and re-verifiable over the asset lifecycle.

Design identity -> inspectability requirement -> candidate view -> reachable acquisition -> measured observation -> calibrated interpretation -> independent adjudication -> engineering decision -> authorized disposition -> reinspection.

Keep separate: geometric view count, failure-domain diversity, physical sensor independence, observation validity, defect findability, diagnosis and engineering acceptance.

## 2. Current exact implementation

Public draft: https://github.com/Luminous-Dynamics/symtropy/pull/6925

The current planner retains two independent requirements per feature: required distinct viewpoint count and required declared failure-domain count. It reports nominal/robust counts on both axes, candidate-pool shortfalls, an exact cost optimum/Pareto frontier for up to 20 candidates, deterministic greedy baselines, and fail-closed dispositions.

Planning corpus v0.2 contains eight frozen synthetic cases, and a separately written Python oracle compares complete receipts. A second exact oracle computes minimum cardinality for nominal k-coverage under the paper's attainable target k*(v)=min(k,SBW(v)); this is separate from the domain-constrained cost optimum.

The published-style greedy baseline now consumes the paper's nominal binary visibility matrix (not our robust matrix), uses deficit and scarcity weighting, a deterministic fixed-point exp(-slack) lookup, lexicographic score ties, and a deterministic selection-order cleanup. Receipts report requested-k attainment, candidate-capped attainment, robust-k diagnostics, nominal/robust failure-domain diagnostics, exact reference cardinality and relative gap. The fixed-point arithmetic and deterministic tie/cleanup conventions are explicitly labelled adaptations; this is not yet a reproduction of the paper's 3D candidate-generation or dataset pipeline.

At exact head fe3f436f6f6abbedcf79cf869ef2fcea9b16a49f, the dedicated benchmark workflow passed geometry and planning replay, both independent oracles, full receipt comparisons, formatting, Rust tests, strict Clippy, manifest creation and artifact retention. Repository CI passed on Ubuntu, Windows and macOS. Exact-head evidence: https://github.com/Luminous-Dynamics/symtropy/actions/runs/45161470588 and https://github.com/Luminous-Dynamics/symtropy/actions/runs/45161470633.

## 3. Direct k-coverage paper comparator

Bin Khalid and Yeum, “Ground Sample Distance Constrained k-Coverage Viewpoint Planning via Set Multi-Cover Formulation,” arXiv:2610.36450, submitted 27 October 2026: https://arxiv.org/abs/2610.36450.

### Published method details to reproduce

- The visibility matrix A is binary and nominal: each candidate viewpoint contributes one count when it observes a voxel.
- Per-voxel SBW is b=A·1 and the paper's attainable target is k*(v)=min(k,SBW(v)).
- At each iteration r=max(0,k*-cov), σ=max(0,b_rem-r), ω=exp(-σ)·(r/k)^2, and the candidate score is Aᵀω. The highest-scoring remaining viewpoint is selected; coverage and remaining candidate counts are updated.
- Cleanup iteratively removes any selected viewpoint whose removal preserves all capped targets. The paper does not specify a deterministic tie/order convention for this set iteration; our fixed order is therefore an explicit implementation adaptation.
- Candidate generation uses a voxelized surface, exterior flood fill, outward boundary-face normals, multiple normal directions for thin walls/edges, a GSD-derived initial distance, blocked-seed ray filtering, distance/FOV/incidence/occlusion checks, and targeted direction augmentation for under-covered voxels.

### Key numerical settings

| Parameter | Paper setting |
|---|---:|
| Voxel size | 0.04, 0.06, 0.08, 0.10 m |
| GSD_max | 0.5 mm/pixel |
| GSD setpoint | 0.25 mm/pixel |
| Maximum obliquity α | 45° |
| Slant range Rs | 0.3394 m |
| Perpendicular distance h | 0.2400 m |
| k | 1, 3, 5 |
| Overlap-pruning thresholds | 50%, 70% |
| Augmentation tilt β | 1° to 90°, 1° step |
| Augmentation azimuth φ | 0° to 360°, 30° step |
| ILP time limit | 3600 s |

The GSD relation is GSD = Rs·2·tan(θ_FOV/2)/(w·cos α), with Rs=sqrt(d²+h²) and h=Rs·cos α. The nadir setpoint is GSD_max·cos² α. Camera width and FOV, frame/origin details, and exact augmentation endpoint conventions must be recovered from source materials or remain explicitly unknown; do not invent them.

### Published results and limitations

The paper reports two structures, four voxel sizes and k values 1/3/5. Its greedy solution uses 3.00–16.39% more viewpoints than the corresponding reported ILP counts. Several ILP cases are not optimality-certified, so these differences are not certified optimality gaps in every configuration. Residual 99.93–99.96% k coverage occurs where the candidate pool has fewer than k admissible views; the method explicitly caps the target and reports the limitation. It claims neither a theoretical approximation guarantee nor a computational speedup.

The overlap-pruning baseline was only evaluated at 0.10 m, with vertical and visibility-descending orderings; its outcomes are order-sensitive and did not reach 100% full coverage in the reported cases. Do not generalize that result to other voxel sizes.

The paper evaluates GSD and visibility at voxel centres. It explicitly says this is a discrete approximation, not a guarantee over the entire voxel surface, and it does not evaluate downstream reconstruction quality, viewpoint separation, UAV reachability/dynamics or collision-safe trajectories. This motivates #7136, a stricter support-certification extension kept separate from faithful reproduction.

## 4. Source and data rights

The paper cites IvanNik17/python-3d-analysis-libraries, which is GPL-3.0 licensed. Its README names the Angel Statue and Rooster assets and links to Mendeley datasets. The GGG-BenchmarkSfM v4 data landing page states CC BY 4.0 (DOI 10.17632/bzxk2n78s9.4). The GGG — Rough or Noisy? v2 landing page states CC BY 4.0 (DOI 10.17632/xtv5y29xvz.2), but its currently indexed landing page indicates files unavailable / metadata only. Exact mapping between the two paper models and downloadable dataset files remains to be pinned.

Do not copy GPL-3.0 source into the AGPL Rust benchmark. Independently implement the algorithm. Keep software and data licensing distinct; pin each downloaded asset, provenance, DOI/version, SHA-256, attribution and modification record. If an exact paper asset cannot be retrieved with compatible rights, label any substitute as a substitute rather than claiming source-faithful reproduction.

## 5. Full-pipeline industry comparator

Pšenička and Edelkamp, “A GPU-Resident Pipeline for Multi-Robot 3D Inspection with Limited Perception,” arXiv:2610.26106, 19 October 2026: https://arxiv.org/abs/2610.26106.

The pipeline combines candidate view generation, GPU visibility, set cover, min-max VRP MIP, continuous-time MAPF, trajectory smoothing and post-smoothing collision/waypoint audits over 14 meshes (up to 7.2 million faces). It reports analytical/LP lower bounds and ablations. The authors explicitly note that decomposition sacrifices global optimality and that there is no shared multi-robot 3D-inspection benchmark for direct external comparisons. Only one tested object is a real underwater structure; other meshes vary topology/resolution but are not underwater domain benchmarks.

The public repository is https://github.com/krystofpsenicka/3D-Inspection. GitHub metadata at inspection time reported no declared repository licence and no LICENSE file at the default branch. Treat it as a research/reproducibility reference only; do not copy source or data without permission/licence. Our hardware constraints differ materially from the paper's RTX 3090 / CUDA / OptiX / RAPIDS / Isaac Sim environment.

## 6. Next paradigm shift: support-certified sensing

New child issue #7136 specifies conservative full-support GSD/FOV/incidence and swept-visibility checks. The first bound encloses a voxel or patch support in a ball of radius r; uses d_min=d0-r and d_max=d0+r; bounds ray-direction variation δ=asin(r/d0); combines δ with centre incidence/FOV angles and normal-cone uncertainty; then computes a conservative GSD upper bound. Any unknown or inconclusive bound abstains. A centre pass alone cannot contribute to full-support coverage.

This is deliberately a separate extension: #7002 reproduces the paper's centre-sampled method; #7136 measures the additional protection, abstention and false-block cost of full-support guarantees.

## 7. Benchmark ladder

| Stage | Benchmark | Required evidence |
|---|---|---|
| B0 | 2D contract / independent replay | Frozen cases, negative controls, exact-head replay, zero oracle disagreement |
| B1 | Exact small k-cover | <=20 candidates, exhaustive subset optimum, zero optimality gap |
| B2 | Published greedy comparison | Same nominal binary visibility matrix; same capped k targets; exact reference where tractable |
| B3 | Source 3D reproduction | Pinned models, exact preprocessing, candidate generation, GSD/FOV/occlusion and augmentation settings |
| B4 | Surface-support certification | Conservative interval bounds, dense-sample oracle, zero false-certified-visible controls |
| B5 | 3D mesh generalization | More topologies/resolutions, held-out geometry, mesh/point-cloud variation |
| B6 | Reachability/fleet | #5920 exact small oracle; #5892 source-faithful fleet baseline; collision/energy/charging/return checks |
| B7 | Decision usefulness | #6752/#4258 observation semantics; #6956/#2239 non-anticipative decision policies |
| B8 | Digital thread | AP242/QIF identity, datum/PMI/measurement persistence through Mycelix #6390 |
| B9 | Physical inspection | Instrument-specific calibration, independent truth, qualified NDE procedure and POD study |

## 8. Metrics and targets

Publish independently: requested-k attainment; capped-target attainment; candidate-pool shortfall; selected views; exact/MIP bound gap and certificate status; GSD distribution/max; incidence and FOV margins; support measure fully covered/partial/indeterminate; occlusion false-clear/false-block; runtime/memory; route length/makespan/energy; collisions/return reserve; calibration and uncertainty; false-clearance/missed indication; decision regret and post-repair verification.

Prospective targets, not achieved results:
- 100% independent oracle agreement for every <=20-view admitted instance;
- zero silent k reduction; original requested and attainable capped k always co-reported;
- for larger matched pools, target median cardinality gap <=3% and 95th percentile <=10% against certified ILP lower bounds, with timeouts and uncertified incumbents separate;
- zero false-certified-visible results on the support-certification adversarial suite;
- zero collision, battery, charger or return-reserve violations in route/fleet frozen suites;
- no physical NDE or structural-acceptance claim absent method-specific physical evidence.

## 9. Specialist ownership

- #6845/#6925: orbital geometry and view selection;
- #7002: source-aware 2026 GSD k-coverage reproduction;
- #7136: support-level sensing and occlusion certificates;
- #1782/#2239/#1892/#4997: active-view and decision-value semantics;
- #4258/#6752: observation operators and common-cause sensing;
- #6956: non-anticipative decision policies;
- #5920/#5892: routing, fleet and charging;
- #6846: 3D metrology and datum uncertainty;
- Mycelix #6390/#6519: AP242/QIF, provenance and authority evidence.

## 10. Execution order

1. Preserve #6925 exact-head evidence and keep it draft.
2. Complete file-level source admission and hashes for #7002; explicitly record any inaccessible dataset file.
3. Implement the full 3D voxel/surface-normal/candidate/visibility kernel with a tiny independent oracle.
4. Reproduce published greedy, overlap pruning and certified ILP outcomes on the same candidate matrices.
5. Add #7136 support-certification in parallel as an explicit stricter variant.
6. Transfer only route-reachable views to #5920 and keep #5892 source-faithful.
7. Connect measured outcomes and evidence through existing observation, decision and digital-thread owners.

The success criterion is more independently verified, decision-useful inspection per lifecycle cost under matched sensing, reachability and safety constraints—not a lower view count on a hand-built visibility matrix.
