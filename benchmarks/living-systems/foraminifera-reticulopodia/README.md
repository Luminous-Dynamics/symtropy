# Foraminifera reticulopodial movement benchmark

This benchmark covers benthic foraminiferal reticulopodia as a dynamic, branching transport and locomotion system. It is part of Rhizaria/Retaria coverage, not a new taxonomic kingdom.

## Paradigm shift: three separate measurement layers

1. **Reticulopodial network morphology:** extensions, branch points, anastomoses, fusions, retractions, attachment and detachment.
2. **Internal cytoplasmic transport:** bidirectional particle/organelle streaming where it can be observed and tracked.
3. **Whole-organism motility:** body/test displacement, attachment to container substrate, and motile/non-motile state over longer experimental windows.

Reticulopodia can form an interconnected network with bidirectional cytoplasmic flow. They can contribute to locomotion and feeding; in benthic forms, they also support substrate attachment. However, the current PANGAEA record is a mixture of final-timepoint microscope videos/images and coarse longitudinal attachment/motility observations. It does not by itself provide continuous, calibrated flow fields for every specimen.

## Primary source data

### PANGAEA 10.1594/PANGAEA.974619 — final timepoint images and videos

Schmidt et al. (2025), “File names of the images and videos recorded at the microscope from the Bleaching Ecophysiology Assessment experiment at final observations.” The record includes epifluorescence images and microscope video observations of pseudopodial movement for *Sorites orbiculus* and *Amphistegina lobifera*, plus species IDs, treatment, date/time, pseudopodia presence and study type. The PANGAEA record lists 450 data points and CC BY 4.0. The video component is at final observations of the BEA experiment; it is not a full continuous trajectory archive.

Record: [10.1594/PANGAEA.974619](https://doi.org/10.1594/PANGAEA.974619).

### PANGAEA 10.1594/PANGAEA.974621 — 4-week BEA motility

Longitudinal record for *Amphistegina lobifera* with 1,794 data points from 19 November to 16 December 2019. It records individuals attached to culture-jar walls, total specimens and the motile fraction, with comments/study type. License: CC BY 4.0. These are aggregate/longitudinal response measures, not frame-level reticulopodial network tracks.

Record: [10.1594/PANGAEA.974621](https://doi.org/10.1594/PANGAEA.974621).

### Related records in the bundled experiment

- [10.1594/PANGAEA.974622](https://doi.org/10.1594/PANGAEA.974622) — *A. lobifera* through the 6-week Menthol Concentration Comparison; 1,064 data points; CC BY 4.0.
- [10.1594/PANGAEA.974623](https://doi.org/10.1594/PANGAEA.974623) — *A. lobifera* motility through the High Concentration Trial; 925 data points; CC BY 4.0.
- [10.1594/PANGAEA.974618](https://doi.org/10.1594/PANGAEA.974618) — bundled study record covering *A. lobifera* and *S. orbiculus*; contains five component datasets and declares CC BY 4.0.

Preserve the experimental treatment, species, symbiont/bleaching status and time variables. These are menthol/DCMU bleaching experiments, not neutral baseline motility trials.

## Mechanistic references

- Travis & Allen (1981), *Journal of Cell Biology*, [10.1083/jcb.90.1.211](https://doi.org/10.1083/jcb.90.1.211), describes the ultrastructure of the *Allogromia laticollaris* reticulopodial network, including repeated branching/anastomosis and continuous cytoplasmic particle streaming.
- Greco et al. (2023), *Journal of Plankton Research*, [10.1093/plankt/fbad031](https://doi.org/10.1093/plankt/fbad031), describes reticulopodia and other cytoplasmic projections in planktonic foraminifera. It distinguishes retractile/branching protrusions and describes reticulopodial networks and bidirectional flow; not every cytoplasmic projection is a reticulopodium.
- Sandin et al. (2026), *BioEssays*, [10.1002/bies.70148](https://doi.org/10.1002/bies.70148), proposes an integrated Retaria research framework connecting Foraminifera and Radiolaria while identifying major gaps in the biology of living representatives.

## Matched model hierarchy

1. **Empirical image/record baseline:** annotate what is actually visible; no inferred flow or network function.
2. **Static network snapshot:** topology at one time; a negative control because it has no dynamics or transport.
3. **Independent pseudopod agents:** extension/retraction without anastomosis or shared flow.
4. **Dynamic reticulopodial network:** branch extension, fusion, retraction and attachment.
5. **Flow-coupled network:** dynamic topology plus bidirectional particle streaming, only in videos where particles can be tracked.
6. **Body–network coupling:** test/body movement and attachment linked to reticulopodial change, only when body position/contact can be observed.

Do not equate this with fungal hyphal networks or *Physarum* plasmodial tubes. Shared abstractions (branches, nodes, edge lifetimes) may support comparison, but the cellular mechanics differ.

## Metrics

### Network morphology
- pseudopod segmentation/boundary error;
- branch-tip localization, branching and retraction event precision/recall;
- anastomosis/fusion events and event timing;
- connected components, branch order, edge lifetime and network coverage;
- true fusion versus apparent 2D crossing/occlusion, with unresolved states.

### Streaming
- particle/organelle localization and identity-linking error;
- local flow direction and velocity vector error;
- evidence for counter-streaming/bidirectional flow;
- flow reversal frequency and relation to branch geometry;
- no flow metric where particle tracking is unsupported.

### Whole-organism response
- attachment fraction and motility fraction over time;
- body/test displacement where measured;
- response across species, treatment, menthol concentration, symbiont status and time;
- replicate-level uncertainty and censoring.

## Data admission and experimental design

- All cited PANGAEA records above report CC BY 4.0; retain each dataset's exact citation and record-level attribution.
- Pin file list, record revision, checksums, coordinate/time scales and specimen/run metadata before deriving annotations.
- Split by complete experimental run/specimen/culture unit, not by pseudopod pixels or neighbouring video frames.
- Treat the 1,794 motility points as observations, not automatically as 1,794 independent biological replicates.
- Separate frame-level pseudopod videos from longitudinal count/attachment tables.
- Carry treatment, bleaching, symbiont condition, species and time as covariates.
- In 2D images, apparent branch crossings, out-of-plane branches and ambiguous particle identities remain unresolved.

## Claim ceiling

This benchmark qualifies specific reticulopodial observations and motility responses in the declared foraminiferal species/experiments. It does not establish all Rhizaria mechanisms, infer cytoplasmic flux from topology alone, quantify biogeochemical export, or generalize to Radiolaria without distinct data.