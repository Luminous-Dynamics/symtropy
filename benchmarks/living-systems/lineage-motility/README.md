# Archaea and protist movement benchmark anchors

This portfolio adds three mechanism-specific reference lanes to the broader
cross-taxon life-systems programme. It is a benchmark contract, not a
single universal locomotion solver.

## Why these three anchors

| Lineage | Organism / dataset anchor | Mechanism |
|---|---|---|
| Archaea (Haloarchaea) | *Haloferax* sp. Boulby Mine and *Haloarcula* sp. Great Salt Lake | Archaellum swimming and chemotaxis |
| Amoebozoa | *Dictyostelium discoideum* | Actin-driven pseudopod crawling, cAMP chemotaxis and collective development |
| Alveolata (ciliates) | *Paramecium tetraurelia* | Ciliary propulsion, calcium-mediated avoidance and backward/forward swimming |

These are not three “kingdoms”. Archaea is a domain; Amoebozoa and ciliates
are eukaryotic lineages. Taxon concepts must be resolved against a pinned
provider/release using the versioned identity contract in
`symtropy-lifesim-core`. A name alone is not an identity key.

## Archaea: Haloferax / Haloarcula environmental isolates

Thornton et al. (2020), *Nature Communications*, DOI
[10.1038/s41467-020-18253-7](https://doi.org/10.1038/s41467-020-18253-7),
report three-dimensional holographic tracking and Brownian-dynamics modelling
of haloarchaeal chemotaxis. The paper reports slow, directional chemotaxis
with very long runs, archaellar directional persistence and low power
requirements. The paper's results are not a generic bacterial run–tumble
parameterization.

The University of York archive,
[10.15124/6b44e8e4-a22c-4a55-a2fc-b2976a06d2b4](https://doi.org/10.15124/6b44e8e4-a22c-4a55-a2fc-b2976a06d2b4),
is listed as CC BY and contains both experimental and simulation results.
Before use, separate those files, verify file-level notices/attribution,
record the inventory and checksums, and preserve strain/replicate metadata.

A separate comparator, Kinosita et al. (2016), directly observes rotation
and steps of the archaellum in *Halobacterium salinarum*:
[10.1038/nmicrobiol.2016.148](https://doi.org/10.1038/nmicrobiol.2016.148).
Do not represent this as the same source dataset as the environmental
Haloferax/Haloarcula chemotaxis archive.

## Amoebozoa: Dictyostelium

The Dryad dataset,
[10.5061/dryad.b5mkkwhbd](https://doi.org/10.5061/dryad.b5mkkwhbd),
contains actin-labelled confocal image sequences intended for cell-contour
and track analysis. The record reports 500/1000-frame sequences at 1-second
cadence and calibrated field sizes. The dataset landing page inspected for
this benchmark did not expose a verifiable license field. Its current status
is therefore NOT_ADMITTED until the exact license, data files, checksums and
metadata are reviewed.

Keep the biological questions separate:
- stochastic amoeboid crawling and pseudopod morphology;
- cAMP-directed chemotaxis;
- oxygen-directed aerotaxis under hypoxia;
- collective aggregation/development and cell-state synchronicity.

Useful comparators:
- random crawling, membrane deformation and polarity:
  [PLOS ONE 2018](https://doi.org/10.1371/journal.pone.0201977);
- pseudopodium-growth direction and chemotaxis:
  [Scientific Reports 2017](https://doi.org/10.1038/s41598-017-12656-1);
- hypoxia/aerotaxis:
  [BMC Molecular and Cell Biology 2025](https://doi.org/10.1186/s12860-025-00557-5);
- early cAMP signalling and multicellular developmental synchronicity:
  [Communications Biology 2026](https://doi.org/10.1038/s42003-026-09806-5),
  GEO GSE305468 and Zenodo data/code records in the manifest.

Single-cell motility trajectories do not establish multicellular aggregation
or developmental synchrony. Those require separate observation channels and
independent experimental units.

## Ciliates: Paramecium

Elices et al. (2023), *PLOS Computational Biology*, DOI
[10.1371/journal.pcbi.1010899](https://doi.org/10.1371/journal.pcbi.1010899),
couple electrophysiology, intracellular calcium and ciliary kinematics to
model free swimming, avoidance, gradient following and collective behaviour.
The paper's selected behavioural set contains 554 trajectories, after
documented filtering and manual track inspection.

Data are separately deposited:
- electrophysiology / video / PIV:
  [Zenodo 6074166](https://doi.org/10.5281/zenodo.6074166);
- behavioural data and code:
  [Zenodo 6074480](https://doi.org/10.5281/zenodo.6074480);
- source model:
  [romainbrette/Paramecium-model](https://github.com/romainbrette/Paramecium-model).

The source model repository is GPL-3.0 and the article is CC BY. The retrieved
Zenodo landing pages did not expose license fields for the two datasets.
Do not infer dataset terms from the article or repository. Pin each record's
version, per-file terms, file digests and attribution before ingestion.

The critical comparator is not another bacterial renewal law: it is the
coupling of membrane excitation and calcium to ciliary beat/turning kinematics,
including backward swimming and the avoidance reaction.

## Matched benchmark design

Each organism gets its own source-specific model and scorecard. The shared
evaluation envelope reports:

- trajectory displacement, speed, directional persistence and turning;
- mode-transition and event timing;
- cell shape/contour and tracking error where available;
- sensory response latency and stimulus-response curves;
- ciliary/PIV or archaellum rotation statistics when measured;
- chemotactic/aerotactic index and held-out stimulus response;
- finite-window MSD, escape/first-passage distributions and uncertainty;
- independent recording-level calibration and performance cost.

Do not pool all three into a raw-speed ranking. Their mechanisms operate
through different structures and physical scales.

## Rights and validation status

The exact record identifiers and source-reported metadata are pinned in
`manifest.toml`. No external data have been ingested, redistributed or
fitted here. The haloarchaeal York record lists CC BY; the Dictyostelium and
Paramecium dataset licenses remain unverified from the retrieved record
metadata. Source paper, software and dataset licenses are treated separately.

A successful benchmark must reproduce each reference on the observed
measurement window, preserve experimental-unit splits, report calibration
and failures, and validate on held-out conditions. The portfolio does not
claim biological reproduction, a universal life model or SOTA.
