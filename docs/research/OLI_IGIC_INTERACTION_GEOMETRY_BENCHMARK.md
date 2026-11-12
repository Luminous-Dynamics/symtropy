# OLI interaction-geometry inference benchmark: IGIC source profile v0

**Status:** paper and dataset source pinned; dataset not materialized; reproduction not run  
**Related:** [OLI Vicsek active-matter control PR #4670](https://github.com/Luminous-Dynamics/symtropy/pull/4670), [OLI causal influence follow-up #11614](https://github.com/Luminous-Dynamics/symthaea/issues/11614)  
**Machine-readable profile:** `benchmarks/oli/igic-interaction-geometry-source-profile-v0.json`

## Why this is a strong comparator

McKinney and Roy's 2026 Scientific Reports paper, *A correlation based framework for inferring interaction geometry in collective motion*, introduces Interaction Geometry Inference using Correlation (IGIC). It estimates interaction radius and sensing angle from time-resolved trajectories using lagged circular correlations between neighbour-averaged headings and subsequent focal-agent motion.

The authors report **99.2% overall recovery accuracy** on synthetic visual-Vicsek trajectories with known interaction geometry across a broad range of sensing angles and noise intensities. They also test topological Vicsek data, where no unique true metric radius exists, compare against transfer entropy and mutual information, and apply the method to Golden Shiner trajectories. This is a task-specific interaction-geometry result—not a 99.2% score for collective intelligence, biological-life recognition or causal discovery.

Paper: [DOI 10.1038/s41598-026-63720-8](https://doi.org/10.1038/s41598-026-63720-8).

## Public source dataset

The authors publish a dataset package:
- DOI: [10.17632/jwkv3cf7fs.1](https://doi.org/10.17632/jwkv3cf7fs.1)
- Repository page: [Embry-Riddle Research Data Commons](https://datacommons.erau.edu/datasets/jwkv3cf7fs/1)
- Reported contents: MATLAB code, synthetic visual/topological Vicsek simulations, empirical Golden Shiner trajectories and analysis results.
- Dataset page declares CC BY 4.0.

The data package has not yet been downloaded. The next admission step is to enumerate files and record per-file digests and rights metadata. Do not infer that the dataset's CC BY declaration is automatically the software package's standalone licence.

## Source-faithful reproduction comes first

The 99.2% result must be reproduced using the authors' exact:
- synthetic visual-Vicsek generator and parameters;
- sensing-angle and noise grids;
- candidate interaction geometries;
- time-lag range;
- trajectory length and sample selection;
- preprocessing and score aggregation;
- random seeds and holdout partition.

The exact split and seed law must be recovered from source. A newly invented split that happens to yield a similar score is not reproduction.

## OLI extensions

1. **Topology mismatch:** the source paper already tests topological Vicsek data where no unique metric radius exists. OLI should quantify calibration and require the output to be labelled an effective interaction scale or model mismatch—not a falsely precise physical radius.
2. **Common-driver confounding:** two particles may move predictably together because of a shared external signal without directly interacting. Correlation/transfer entropy may identify dependence; causal edge attribution requires intervention or a qualified structural model.
3. **Nonreciprocal interactions:** recover direction and strength when A influences B but not vice versa.
4. **Observability:** stress irregular cadence, missing tracks, low frame rate, occlusion and lag uncertainty.
5. **Interventional confirmation:** compare inferred observational geometry with the response graph measured by controlled single-particle perturbations and matched sham trajectories.

## Competitive benchmark design

Compare the source IGIC implementation, transfer entropy, mutual information, cross-correlation and a known-geometry oracle on the same trajectories and evaluation splits.

Report:
- exact reproduction of the paper's metric;
- radius absolute error and sensing-angle circular error;
- joint geometry recovery;
- lag/noise/cadence robustness;
- risk-coverage and model-mismatch calibration;
- interventional edge precision/recall and treatment-effect error;
- runtime, memory and trajectory length required.

Hold out entire simulation seeds, trajectories, parameter configurations and noise/sensing regimes. Never split frames or overlapping windows from the same trajectory across train and test. Keep the Golden Shiner empirical data as a separate evaluation stratum.

## Proposed OLI qualification targets

These are targets, not achieved results:
- independently reproduce the paper's source-faithful 99.2% claim within its reported uncertainty and exact test distribution;
- maintain calibrated radius/angle estimates on unseen seeds and parameter regimes;
- abstain or relabel the estimate as an effective scale under topological model mismatch;
- zero causal-edge claims in common-driver null cases without intervention evidence;
- recover asymmetric edge direction with confidence intervals on held-out nonreciprocal systems;
- quantify and expose sampling-cadence / lag aliasing rather than hiding it in a point estimate.

The primary opportunity is not to claim a higher score than IGIC on the same synthetic test. It is to extend interaction-geometry inference into a **causally checked, open-world and observability-aware** scientific workflow, preserving the source metric as a baseline.
