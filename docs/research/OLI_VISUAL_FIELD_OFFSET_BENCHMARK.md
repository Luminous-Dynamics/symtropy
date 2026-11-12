# OLI visual-field-offset collective-motion benchmark v0

**Status:** paper and public dataset pinned; file inventory and exact reproduction pending  
**Related implementation:** [Symtropy Vicsek control PR #4670](https://github.com/Luminous-Dynamics/symtropy/pull/4670)  
**Related causal benchmark:** [Symthaea OLI-ACTIVE-CAUSAL-001 #11614](https://github.com/Luminous-Dynamics/symthaea/issues/11614)  
**Machine-readable profile:** `benchmarks/oli/visual-field-offset-source-profile-v0.json`

## Source and central result

Ramlall, Butail and Roy, *Influence of visual field offset on collective behavior: An exploration using the two-dimensional Vicsek model*, Physical Review E 113(6), 064409 (17 June 2026), DOI [10.1103/bbl5-1yjb](https://doi.org/10.1103/bbl5-1yjb).

The paper studies a restricted visual field whose central direction is offset by a fixed angle from each particle's heading. Its abstract reports that a flipped/rear-facing visual field can produce near-perfect alignment and centre-of-mass clustering even under high noise—behaviour unlike the standard forward-aligned Vicsek setup.

The companion dataset is [10.17632/y8fx3shsvx.1](https://datacommons.erau.edu/datasets/y8fx3shsvx/1), published 27 May 2026. The Data Commons page describes MATLAB scripts and the simulation data analysed in the paper and declares CC BY 4.0. The public rendering exposes a Download All control but no individual file rows. The files have not yet been downloaded or hashed; file-level inventory and rights still need admission.

## Why this changes the model

A metric-radius model assumes reciprocal visibility: if A and B lie inside the interaction radius, both see one another. An offset visual field makes visibility focal-agent-specific. A can see B while B cannot see A. This changes the interaction graph from an undirected geometric neighbourhood to a directed sensory graph.

The Symtropy reference now supports:
- metric-radius reciprocal perception (the default, unchanged);
- visual-sector perception with explicit half-aperture and offset angles;
- focal-agent-directed visible-neighbour sets;
- explicit interaction-graph semantics in each metric snapshot;
- directed edge count, reciprocal pair count and interaction reciprocity fraction;
- weakly connected projection for connected-component and graph-topology statistics.

The radial interaction radius remains a range limit. The current visual-sector implementation is an OLI reference control; it is not yet a source-faithful reproduction of the paper's MATLAB code.

## Exact model semantics

For focal particle (i), target particle (j) is visible when:
1. minimum-image distance is no greater than the declared radial interaction range; and
2. the bearing from (i) to (j) lies within the half-aperture of the visual field centred at (	heta_i + phi), where (phi) is the visual-field offset.

Offset (phi=0) centres sensing forward; (phi=pi) flips it behind the heading. The exact source aperture and parameter grids remain unextracted and must not be guessed.

The output distinguishes:
- visible out-degree and local polar order (directed, focal-agent-specific);
- weak-component and shortest-path topology (undirected weak projection);
- directed interaction edge count;
- reciprocal pair count;
- reciprocity fraction (2N_{mutual}/N_{directed}), undefined when no directed edges exist.

This prevents an undirected component graph from silently becoming the claimed causal interaction graph.

## Benchmark tasks

1. **Source reproduction:** recover the source MATLAB files, README, parameter grid, seed law and measurement windows; reproduce first.
2. **Forward/side/rear offset sweep:** compare collective order, clustering, centre-of-mass aggregation and sensitivity to noise.
3. **Offset inference:** infer the visual offset and half-aperture from trajectories; compare the source IGIC geometry method against known parameters.
4. **Asymmetric visibility:** evaluate directed-edge recovery and reciprocity, not only weak connectivity.
5. **Intervention response:** rotate one particle and estimate downstream effects against matched sham trajectories under each perception geometry.
6. **Observability:** test cadence, occlusion, angular noise and limited field-of-view sampling.
7. **Model mismatch:** test topological neighbourhoods and nonreciprocal weights; allow effective-scale estimates or abstention rather than a falsely precise metric radius.

## SOTA comparison and qualification

IGIC reports 99.2% interaction-geometry recovery on its own synthetic visual-Vicsek validation task. That score is not a causal influence score and must not be compared directly to polar order or closed-loop trajectory error.

The correct sequence is:
- reproduce IGIC and the visual-field-offset source paper separately using their original code/data;
- compare interaction-radius, sensing-angle and offset estimation with matched trajectory splits;
- report angular error, parameter MAE, geometry recovery, calibration, topology-mismatch abstention and compute cost;
- separately score interventional causal effects, directed-edge recovery and false edges under common-driver controls.

Proposed qualification targets (not achieved):
- zero false reciprocal-edge claims on the exact one-way visual-sector controls;
- normalized angular offset MAE <= 0.05 radians on frozen known-truth synthetic cases;
- >=90% offset/edge-direction recovery on whole-seed held-out visual-sector regimes at a declared coverage floor;
- calibrated abstention when the interaction rule is outside the candidate family;
- exact source reproduction of the original paper before any claim of improvement over its results.

## Data admission and claim ceiling

No source MATLAB file or trajectory archive has been downloaded in this work pass. Obtain the package, verify each file's digest and rights, and preserve the exact README and source code before reproduction. The dataset page's CC BY 4.0 declaration is recorded at package level; check any file-level notices on acquisition.

This is a synthetic collective-motion benchmark. Collective order, rear-facing perception, asymmetric edges and information transfer do not establish biological life, cognition, agency, intention or consciousness.
