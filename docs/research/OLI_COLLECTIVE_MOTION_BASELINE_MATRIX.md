# OLI collective-motion baseline matrix v0

**Status:** literature-pinned, task-matched research plan; source reproductions pending  
**Related implementation:** [Symtropy Vicsek active-matter PR #4670](https://github.com/Luminous-Dynamics/symtropy/pull/4670)  
**Related causal benchmark:** [Symthaea OLI-ACTIVE-CAUSAL-001 #11614](https://github.com/Luminous-Dynamics/symthaea/issues/11614)  
**Machine-readable profile:** `benchmarks/oli/collective-motion-baseline-matrix-v0.json`

## Why one SOTA score is the wrong comparison

Collective-motion research has several distinct inverse and forward problems. A method can estimate an interaction radius accurately while failing to identify heterogeneous edge types; it can infer a graph while producing unstable long-horizon rollouts; it can predict trajectories accurately while its learned attention weights do not identify causal influence.

OLI therefore separates five tracks and preserves each baseline's original task and metric.

## Baseline landscape

### 1. IGIC — interaction geometry

McKinney & Roy (2026), *Scientific Reports*, DOI [10.1038/s41598-026-63720-8](https://doi.org/10.1038/s41598-026-63720-8).

IGIC estimates metric interaction radius and sensing angle using lagged circular correlations between neighbour-averaged headings and subsequent motion. The authors report 99.2% geometry-recovery accuracy on their synthetic visual-Vicsek test. They also test topological Vicsek data and empirical Golden Shiner trajectories.

Public data/code package: DOI [10.17632/jwkv3cf7fs.1](https://doi.org/10.17632/jwkv3cf7fs.1), dataset page declares CC BY 4.0. It contains MATLAB source, synthetic visual/topological Vicsek data, Golden Shiner trajectories and analysis results. It has not yet been downloaded or independently reproduced.

**OLI use:** exact source reproduction first, then whole-seed/regime holdouts, topology mismatch, cadence and intervention checks.

### 2. CRI / Evolving-CRI — heterogeneous, correlated and evolving edges

Han, Fink & Kammer (2024), *Nature Communications* 15, 3191, DOI [10.1038/s41467-024-47098-7](https://doi.org/10.1038/s41467-024-47098-7).

CRI jointly infers types of incoming edges while modelling correlation among them. Evolving-CRI updates the inferred edge distribution when neighbourhood topology changes. The paper reports performance against independently inferred-edge baselines including NRI, and tests interpolation/extrapolation for evolving-topology particle/crystallization systems. In the particle experiment, the cutoff radius is assumed known; this must remain explicit in a fair comparison.

Code: [ETH public GitLab](https://gitlab.ethz.ch/cmbm-public/toolboxes/cri).  
Particle interaction dataset: [ETH Research Collection handle 20.500.11850/610139](https://www.research-collection.ethz.ch/handle/20.500.11850/610139). The dataset page could not be fetched during this research pass; inventory and rights remain pending.

**OLI use:** heterogeneous and evolving interaction edges, including direction and interaction type. Do not call this a radius-inference baseline.

### 3. GDP — graph recovery under undersampling

Pan, Shi & Dokmanic (2024), *AAAI* 38(13), 14508–14516, DOI [10.1609/aaai.v38i13.29366](https://doi.org/10.1609/aaai.v38i13.29366).

The Graph Dynamics Prior combines a shallow one-step model with a polynomial multi-step model over a shared graph topology. The paper reports stronger graph recovery and robustness to under-sampling than earlier relational-inference methods on its evaluated tasks.

Code: [DaDaCheng/GDP](https://github.com/DaDaCheng/GDP). Dataset versions, environment and exact splits still need pinning before reproduction.

**OLI use:** test whether interaction graphs remain recoverable when frame rate is insufficient to observe immediate neighbours or direct dynamics.

### 4. PR-GNN — closed-loop collective trajectory prediction

Liu, Li, Wang et al. (2026), *Complex & Intelligent Systems*, DOI [10.1007/s40747-026-02566-z](https://doi.org/10.1007/s40747-026-02566-z), published 26 October 2026.

PR-GNN builds focal-centred dynamic interaction graphs, learns neighbour importance, decomposes response into self-motion, velocity alignment and spatial interaction components, and trains with multi-step rollout and collective-behaviour regularization. On six self-propelled-particle datasets, the authors report lower closed-loop position error than five evaluated baselines while preserving polarization, rotational organization, mean speed and spatial spread.

The accessible main article lists a 32.315 MB supplementary PDF and an open-access CC BY-NC-ND 4.0 licence. The examined page did not provide a code/data availability statement. Code, data and supplementary artifacts remain to be inspected or requested; do not assume that the article licence grants rights to third-party datasets or software.

**OLI use:** closed-loop stability and macro-scale group organization. Attention weights alone are not causal edges.

## OLI's differentiator: four questions, four outputs

Every benchmark receipt must keep separate:
1. **Geometry:** who could interact under what metric/field-of-view rule?
2. **Relational mechanism:** which directed edge types or response functions operate?
3. **Prediction:** how well does the model reproduce individual and group trajectories in closed loop?
4. **Causal influence:** what changes under a controlled intervention versus a matched sham?

A high score on one dimension cannot substitute for evidence on another. Visual-sector geometry and directed-visibility recovery are now a separate task track; weak connectivity must not be reported as reciprocal awareness.

## Paradigm shift: from static graph discovery to intervention-checked mechanism discovery

The proposed loop is:

`observed trajectories -> candidate geometry -> candidate edge/response mechanism -> closed-loop replay -> intervention/sham -> model-adequacy challenge -> qualified mechanism or abstention`

This combines the transparency of IGIC, correlated edge inference from CRI, undersampling robustness from GDP, and long-horizon macro-behaviour fidelity from PR-GNN. It should reuse the existing Symtropy physics reference and Symthaea model-adequacy/experiment-selection infrastructure rather than build a second generic graph learner.

## Matched evaluation protocol

- Reproduce every source result on its original dataset, preprocessing, seeds, splits, lag range and metric.
- Split by complete trajectory, independent simulation seed, parameter configuration and source sample—not by frames or overlapping windows.
- Reserve whole interaction-rule families, topology regimes, sensing-angle/noise ranges and sampling cadences for sealed evaluation.
- Keep empirical Golden Shiner data independent of synthetic training and tuning.
- Compare model prediction and inferred mechanism separately: a wrong graph can sometimes predict a short horizon.
- Add common-driver nulls and randomized interventional cases. Statistical dependence and transfer entropy do not establish causal influence.
- Report calibration, risk-coverage and explicit model-mismatch/abstention rates.

## Proposed qualification targets

Targets only; no result has yet been achieved:
- Reproduce the original IGIC metric before any “better than 99.2%” statement.
- Maintain IGIC-level results on the source test distribution, then improve selective risk/calibration on whole-seed and whole-regime holdouts.
- On matched PR-GNN data, target at least 10% lower multi-horizon normalized rollout error than the strongest reproduced baseline without degrading registered macro-observables beyond their uncertainty.
- Reach at least 90% direction recovery on held-out asymmetric known-truth systems at a pre-registered sampling envelope.
- Zero false causal edges on 299 independent common-driver/no-coupling null systems as a proposed route to a one-sided 95% exact upper false-positive bound below 1%.
- Zero false model-adequacy passes on registered omitted-mechanism controls.
- Abstain under topology mismatch or inadequate observation cadence instead of emitting false precision.

All numerical targets are proposals for future qualification, not current performance claims.

## Data admission

The machine-readable matrix records each source's data/code status. Until source revisions, file inventories, per-file hashes, environment versions, splits and licensing are pinned, no source reproduction is complete and no cross-method SOTA claim is allowed.


## 5. Directional visual perception: a new 2026 baseline

Ramlall, Butail and Roy (2026), *Physical Review E* 113, 064409, DOI [10.1103/bbl5-1yjb](https://doi.org/10.1103/bbl5-1yjb), investigate a restricted visual field offset from a particle's heading. Their reported flipped/rear-facing configuration exhibits near-perfect alignment and clustering at the centre of mass even under high noise. The companion dataset is DOI [10.17632/y8fx3shsvx.1](https://doi.org/10.17632/y8fx3shsvx.1), a MATLAB/scripts-and-data package whose Data Commons page declares CC BY 4.0. File inventory, exact source grids and per-file digests are still pending.

This motivates a separate **DIRECTIONAL_VISUAL_PERCEPTION** task:
- infer visual offset and half-aperture;
- recover directed visibility edges and reciprocity;
- distinguish a weakly connected component from mutual awareness;
- test forward, side and flipped/rear perception under matched noise/density;
- measure intervention effects separately from statistical information transfer.

The current Symtropy reference includes `VicsekPerception::VisualSector`, focal-agent-specific out-neighbour counts and explicit directed-edge/reciprocity metrics. Weakly connected components are clearly tagged as a topology projection, not as the directed interaction graph.

**Competitive rule:** do not compare the new Rust control against the paper's plotted outcomes until the public MATLAB package is downloaded, file-hashed, and its exact aperture/offset/noise/density/seed grid is reproduced. The paper's result is a comparator reference, not an OLI result.



## 6. SCM — small-swarm cooperative search and dynamic sensor localization

Nitti, de Tullio, Federico & Carbone (2025), *Nature Communications* 16, 6572, DOI [10.1038/s41467-025-61985-7](https://doi.org/10.1038/s41467-025-61985-7).

The Swarm Cooperation Model (SCM) is a valuable task-level baseline because it couples a fitness-gradient term with all-to-all sinusoidal consensus interaction and adaptive Gaussian forcing. It targets bounded static and time-varying scalar landscapes, and is also used as a controller for a simulated AUV contaminant-localization task.

### What the paper reports

- On low-dimensional 2D/3D landscapes with (M\leq16), SCM matched or exceeded PSO/MIPA success rate on 22 of 33 cases.
- The static benchmark reports 100 replications with 100 different Wiener processes. Success is measured by whether the final swarm centroid is within (0.05L) of the true global optimum.
- The reported success-rate advantage has a cost: about one order of magnitude more function evaluations overall; SCM uses significantly more evaluations on 24 of the 33 cases.
- The paper explicitly notes that SCM does not retain a global best-known position. This is disadvantageous on static landscapes, but may be useful when the landscape changes in time.
- The AUV proof-of-concept simulates five vehicles searching a 6 km square domain in Gallipoli Bay, at 4–6 m depth under 0.05–0.2 m/s currents. Across 100 simulations it reports 86% success, 20.8 ± 3.82 hours mean search time and 93.8 ± 17.3 km mean travel per vehicle. This is a **computational proof of concept**, not a physical sea trial.

### Why this is a useful OLI comparator

SCM shifts the comparison from whether a group moves together to whether a small group achieves a useful task under limited agent count and costly physical measurements. It is especially relevant to OLI's dynamic-observation and active-experiment-selection work.

OLI should compare:
- success probability and confidence intervals;
- sensor/function evaluations per success;
- time-to-solution and distance/energy per success;
- communication bytes, rounds, latency and packet-loss sensitivity;
- tracking error when the signal peak moves or the landscape changes;
- recovery following agent loss, sensor failure or environmental disturbance.

Do not claim SCM is universally inferior because it uses more evaluations: in a physical search task, a sensor sample may be cheap compared with mission failure. Conversely, do not call a success-rate advantage a win while ignoring battery, travel, range, mission time and bandwidth.

### Reproduction and source admission

Machine-readable source profile: `benchmarks/oli/scm-collective-optimization-source-profile-v0.json`.

The linked MATLAB repository is pinned at commit `d9ff09d1a8b26df01cc5937e8774018028eebe79`; key file blob SHAs and the Zenodo v1.0 archive digest are recorded in the profile. The repo has no visible LICENSE file, and the inspected Zenodo Rights field gives no license value. The article's CC BY-NC-ND licence must not be inferred to license the separate code. No MATLAB source has been copied or redistributed.

The equation audit is now resolved: the published discrete update equation and pinned `SCM/tstep.m` both use `sin((pi/2) * delta_x)`. The apparent `2/pi` alternative was a misreading and has been removed from the independent Rust reference. The source repo's file is pinned by blob SHA `d7970b76fc23e5f562df0acbfabdc46424643053`. A genuine reproduction sensitivity remains in the discrete consensus-window indexing: the paper describes equal disjoint half-windows, while MATLAB's inclusive endpoints use `tau_steps + 1` samples and begin the first update one tick later. The Rust reference preserves that window distinction separately.

The static replication script uses the circle-packing coordinates in `utils/csq_coords/csq8.txt` when present, 100 repetitions, eight agents, a Rastrigin 2D default, `dt=0.1`, `tau=60*dt`, `omega=0.2`, `sigma0=0.05`, and per-agent/per-dimension noise seeds determined by `2*i + M*(j-1) + nr - 1`. These are the inspected repository script defaults, not a substitute for reproducing every paper figure/table and its supplementary dataset protocol.

### Independent Rust reference now started

A clean-room, CPU-first reference has been added at `crates/domains/symtropy-lifesim-core/src/scm.rs`, exported from the crate, with a separate machine-readable control set at `benchmarks/oli/scm-reference-cases-v0.json`.

The implementation exposes both the paper-rendered sine kernel and the inspected MATLAB-source sine kernel. It also exposes the paper's equal half-window consensus convention separately from the inclusive-endpoint indexing observed in the MATLAB replication loop. It adds finite handling for zero mean gradient, zero swarm spread, equal fitness, and a flat consensus trend, plus a transactional update: candidate positions, RNG state, observations and counters commit only after all successor observations succeed.

The independent Rust RNG is not MATLAB's RNG; the module is a mathematical reference and adversarial-control starting point, not a bitwise port. Twelve unit tests were added, but rustfmt, compilation, test execution and exact-head CI remain pending. No SCM benchmark result has been generated.
 
### OLI extension opportunities

1. **Cost-aware objective:** optimize probability of mission success under energy, travel, sampling, time and communication budgets—not success rate alone.
2. **Dynamic source tracking:** measure lag, drift and re-localization cost when advection moves the peak.
3. **Communication realism:** replace the paper's full-network exchange assumption with delay, bandwidth, packet loss, intermittent contact and stale measurements.
4. **Common-driver and false-consensus controls:** test whether a shared environmental signal makes independent agents appear to coordinate.
5. **Robustness to sensor bias:** calibrate against heteroskedastic noise, cross-sensitivity, fouling, drift and detection limits.
6. **Ablate best-known memory:** quantify the static-exploitation advantage and dynamic-tracking cost of retaining versus discarding a global best-so-far state.
7. **Ablate gradient access:** test when agents have scalar fitness only, finite-difference gradients, noisy gradients, or gradients with acquisition costs.
8. **Small-swarm scaling:** report (M=3,5,8,16) with matched budgets and uncertainty, rather than assuming a method that works at large N transfers to real small fleets.

The strongest potential OLI result is not merely to match SCM's small-swarm success rate. It is to preserve the useful dynamic search behaviour while reducing costly measurements and operating reliably under realistic communication and sensor constraints—on the same source-defined tasks and independent dynamic-environment holdouts.



## 7. Industry and deployment comparison: compare operating architectures, not slogans

The 2025 SCM paper itself identifies an operational gap: many deployed multi-robot groups rely on centralized control for predictability, while consensus-based distributed methods are widely used for formation/rendezvous but may be constrained to simpler environments and neighbour-limited tasks. Metaheuristics such as PSO/ABC/DE are established for computational optimization, but do not automatically transfer to fixed-size physical fleets or account for vehicle dynamics and costly sampling. SCM is a hybrid attempt to connect optimization and vehicle control, with a small-swarm success-rate advantage and an explicit evaluation-cost penalty.

For OLI, this yields four fair industry/method baselines:

| Operating family | Strengths | Known trade-offs to measure | OLI comparison |
|---|---|---|---|
| Centralized fleet orchestration | Global coordination, easier global planning, predictable fleet constraints | Communication dependency, central failure point, latency and bandwidth | Keep as a strong upper bound; measure loss under disconnection and delayed state |
| Distributed consensus/formation control | Local coordination, rendezvous, formation and shared-state agreement | May not solve multimodal search or maintain task diversity by itself | Measure whether consensus helps task success or merely makes agents agree |
| Metaheuristic optimization (PSO/MIPA/ABC/DE) | Mature task-specific search baselines and established test suites | Agent-count/sampling cost, parameter sensitivity, loss under dynamic objectives | Reproduce exact objective and cost baselines with matched initialization |
| Hybrid small-swarm search (SCM) | Designed for small mobile groups and evolving scalar landscapes | Higher function evaluations, all-to-all data exchange, source equation/index discrepancies | Reproduce both static and dynamic tasks; expose communication and sensor cost |

These are method-family comparators, not claims that every industrial fleet product uses the same architecture. OLI must not compare a high-level product specification against a laboratory optimizer result or infer a commercial deployment win from a synthetic benchmark.

### Where OLI can differentiate

1. **Information-budget transparency:** every agent's local view, shared field, global coordinator input, communication round and stale-data allowance are declared and metered.
2. **Task outcome plus operating cost:** success probability, samples, distance, energy, time, comm bytes and return-to-safe-state are independent reported axes.
3. **Causal and adversarial qualification:** distinguish a true response to interaction from common-driver correlation, centralized information leakage or a passive environmental pattern.
4. **Open-world recovery:** reserve explicit capacity for a signal, source or mechanism not present in the current task model; measure whether the archive can be reanalysed after a new hypothesis arrives.
5. **Model-adequacy and abstention:** when the environment or interaction rule leaves the validated envelope, reduce authority and report why rather than confidently extrapolating.
6. **Evidence-bearing deployment:** preserve sensor calibration, observation coverage, intervention/sham results, software/configuration digests and independent verification records through the engineering lifecycle.

### Competitive qualification targets

The matrix now carries three proposed SCM-relative targets:
- At matched success probability and safety envelope, reduce sensor/function evaluations or energy-distance cost by **at least 20%** in two reproduced benchmark strata; alternatively improve success by **at least 5 percentage points** at equal total cost.
- On dynamic moving-peak/advection cases, reduce median re-localization lag by **at least 20%** without increasing mission-abort rate.
- Under explicit communication constraints, publish the success/cost break point as bandwidth, latency, packet loss, stale-state and intermittent-contact severity increase.

These are qualification targets, not current results. The strongest result would be a Pareto improvement under matched agent count, initialization, sensors, environment, safety envelope and independent random-process holdouts—not a victory on one weighted score.

