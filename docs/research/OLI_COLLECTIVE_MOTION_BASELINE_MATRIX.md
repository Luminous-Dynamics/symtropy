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

A high score on one dimension cannot substitute for evidence on another.

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
