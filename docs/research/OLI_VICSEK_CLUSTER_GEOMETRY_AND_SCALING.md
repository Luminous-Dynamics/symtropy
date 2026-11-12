# OLI Vicsek cluster topology and geometry: source profile v0

**Status:** source code/method audit pinned; full parameter and output reproduction pending  
**Related issue:** [OLI-VICSEK-TOPOLOGY-001 #4842](https://github.com/Luminous-Dynamics/symtropy/issues/4842)  
**Base simulator:** [Vicsek active-matter PR #4670](https://github.com/Luminous-Dynamics/symtropy/pull/4670)  
**Machine-readable profile:** `benchmarks/oli/vicsek-cluster-scaling-source-profile-v0.json`

## Research objective

Reproduce Rahman et al. (2026), *Topology and geometry of clusters in the Vicsek model across scales*, using the 27 October 2026 erratum, then extend the measurements to periodic, multiscale geometry and trajectory-level uncertainty.

The paper studies connected clusters and their graph/geometric properties across scale. Its reported observables include average interaction degree, average path length, clustering coefficient, cluster area and perimeter, with scaling in cluster mass.

A key methodological requirement is to preserve the exact update rule: the erratum corrects the neighbour set for heading at (t+dt) to use positions at (t+dt), while averaging headings/velocities at (t). The corrected implementation is indistinguishable from the forward update; the backward update gives different cluster-scaling results.

## Source code successfully inspected

Pinned author repository commit: `68dfc94192d3fb7324eb980ef095da373ddf5ec1`.

| File | Method | Blob SHA |
|---|---|---|
| `scripts_1_simulation_upto_steady_state/sml_template.cpp` | Forward update | `bb483123de3351f75712877ffcf51c21d3b724ec` |
| `scripts_3a_cluster_shapes/shapes.cpp` | Largest valid component and graph/shape extraction | `108f1f30152c958b3b682e5651b6755491eb037e` |
| `scripts_3_cluster_analysis/scripts_3c_area/cluster_area_calc.cpp` | Rasterized disk-union area | `308f753d57f31ef6e61aec436ef5bec54808d80f` |
| `scripts_3_cluster_analysis/scripts_3d_boundary/boundary.m` | Alpha-shape area/perimeter and boundary/inside/leaf classification | `6edef2b81dc46d48638251f015a9e5490731bbdf` |
| `scripts_4_post_processing/pp_5_boundary/post_perimeter.cpp` | Log-binned two-stage aggregation | `08a16ef8ab20e19c3f82a87c98e25340721815dd` |

The repository has no visible LICENSE file at this pinned commit. This work references file paths and blob hashes; no source code or data are copied or redistributed.

## Two different area methods — do not conflate them

### 1. Rasterized union of radius-1 disks

The C++ area tool stamps a disk of radius 1.0 around each particle on a 0.05-unit pixel lattice and counts unique pixels. It skips singleton clusters. This is a dilated point-set union area; it is not the same mathematical object as the alpha-shape area.

### 2. MATLAB alpha shape

The boundary analysis uses `alphaShape(x,y,0.5)`, MATLAB's `perimeter` and `area`, then `boundaryFacets` and `inShape` to partition boundary, inside and leaf points. It excludes clusters smaller than three and applies `min_x>1` and `min_y>1` filters.

The source's boundary degree calculation is Euclidean and does not apply periodic minimum-image distances. That is a legacy method to reproduce, not a valid torus-aware definition.

### 3. Largest-cluster display extraction

The C++ shapes workflow chooses the largest connected component passing a coordinate-edge filter, computes its centroid and mean degree, and writes positions/edges for visualization. This is a largest-cluster shape view, not an all-cluster statistical summary.

## Source aggregation semantics

The post-processing file:
- constructs 32 logarithmic mass bins over [1,32768];
- calculates means within each source file and mass bin;
- then averages those file-level means and computes population standard deviations across files;
- separately reports total cluster counts.

These standard deviations are not cluster-level standard errors or seed-level confidence intervals. The OLI reanalysis must retain the source file/trajectory hierarchy and use independent simulation seeds as the statistical unit.

## Source configuration still to recover

The inspected forward template uses:
- (N=512), (r=1), (L=sqrt{N/ho});
- (10^4) update steps;
- five independent realizations per noise value.

The author README describes nine density-speed pairs and reports N=32768 post-processed comparisons in the update-rule analysis. The exact critical noise values, density/speed pairs, sampling windows and sibling backward/corrected source-file blobs still need to be pinned. No complete paper reproduction is claimed.

The C++ template uses float32 and `std::mt19937` seeded from `std::random_device`; OLI uses deterministic f64/xorshift64. The source's scalar noise `nu` maps to a full angular interval of (2pi
u) radians. Comparisons must therefore use statistical distributions and declared numerical tolerances, not bitwise trajectories.

## OLI's intended improvement: periodic, multiscale cluster geometry

The paper-faithful legacy method and the OLI extension must be separate outputs.

1. **Periodic seam-aware components:** shifting a flock across the box seam must not change component identities, masses or geometric results.
2. **Multiscale topology:** track connected-component merge scales as the interaction radius varies; do not depend on one arbitrary threshold alone.
3. **Separate geometry operators:** report point-set area, disk-dilated area and alpha-shape area/perimeter as different quantities.
4. **Periodic concave boundary:** develop a torus-aware alpha-shape or equivalent concave geometry, with explicit tiling and resolution sensitivity.
5. **Per-cluster measures:** retain cluster mass, mean degree, mean path, clustering, area, perimeter, orientation and boundary/inside/leaf statistics.
6. **Uncertainty and scaling:** fit power-law, polylogarithmic and alternative scaling models with trajectory-level uncertainty and model comparison.
7. **Update-rule sensitivity:** compare corrected/forward and backward conventions in separate strata.

The H0 interaction-radius filtration is a useful OLI extension: each particle begins as a component, and components merge as the interaction radius grows. The resulting persistence profile reveals whether observed clusters are robust across scales or are artifacts of one selected radius.

## Proposed qualification

- Source results reproduced within pre-registered tolerances once source grids and outputs are recovered.
- 100% parity on frozen small graph/component fixtures.
- Periodic translation and seam invariance within numeric tolerance.
- Geometry convergence curves under pixel/mesh refinement.
- Independent seeds/trajectories used for confidence intervals; snapshots and clusters from one trajectory never count as independent runs.
- Compare power-law, polylogarithmic and alternative scaling laws with uncertainty, rather than selecting by visual straightness.
- No scaling exponent is declared before the full source profile is reproduced and finite-size effects are evaluated.

## Claim ceiling

This is a source-reproduction and simulation methodology benchmark for one Vicsek model family. Cluster topology, shape and scaling do not establish biological life, agency, intelligence or consciousness. The OLI periodic geometry is a new simulation method, not a claim that the published source used a torus-aware alpha shape.
