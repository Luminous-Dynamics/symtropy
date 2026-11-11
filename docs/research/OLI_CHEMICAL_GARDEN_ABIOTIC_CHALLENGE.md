# OLI ABIOTIC-02: ferrihydrite chemical-garden preservation challenge v0

**Status:** literature and protocol pinned; sample-level data not yet acquired  
**Challenge class:** laboratory-known-abiotic morphological look-alike  
**Related:** [OLI-ABIOTIC-001 #4572](https://github.com/Luminous-Dynamics/symtropy/issues/4572)

## Why this challenge exists

Hirsch et al. (2026), *Tricks of Nature From the Ancient Earth and Early Mars: Chemical Gardens Generate Biomorphs With High Preservation Potential*, Geobiology, DOI [10.1111/gbi.70045](https://doi.org/10.1111/gbi.70045), demonstrates that laboratory-generated iron-mineralized chemical gardens form hollow, branched, sinuous filaments with circular cross-sections, consistent diameters and anastomosis—morphologies commonly invoked in arguments about microbial fossils.

The paper also finds that the abiotic filaments remain recognizable after hydrous pyrolysis up to 300°C and undergo comparatively limited degradation versus the biological comparator used in the study.

This is a high-value challenge because it attacks a subtle failure mode: **morphology and preservation can look biological even when the known laboratory formation mechanism is abiotic.**

Scientific boundary: the authors state that formation of iron-mineralized chemical gardens has not been directly observed in nature. This benchmark is therefore a laboratory-known-abiotic control, not proof of natural occurrence on Earth, Mars or an ocean world.

## Source-pinned preparation

Machine-readable protocol and task manifest: `benchmarks/oli/chemical-garden-ferrihydrite-v0.json`.

The article's preparation method is recorded as follows:

- Polycrystalline iron(II) sulfate heptahydrate granules, sieved below 63 µm.
- Aqueous sodium silicate solution at 100 g/L, pH 12.1.
- Iron salt granules submerged without stirring.
- Reaction duration: 24 hours.
- Solution replaced with distilled water three times by pipetting.
- Samples dried at room temperature and pressure for approximately three days.

The article does not specify the solution volume in the method passage captured for this profile. The manifest deliberately leaves that value null rather than guessing.

## Artificial maturation protocol

- Approximately 5 mg chemical-garden material in 0.4 mL ultrasonically degassed deionized water.
- Borosilicate tube frozen with liquid nitrogen and flame-sealed under vacuum to remove oxygen.
- Tube placed in a 75 mL stainless-steel pressure reactor containing 30 mL deionized water.
- Hydrous pyrolysis for 72 hours.

| Temperature | Estimated pressure |
|---:|---:|
| 50°C | Not reported |
| 100°C | 0.1 MPa |
| 150°C | 0.5 MPa |
| 200°C | 1.6 MPa |
| 250°C | 4.0 MPa |
| 300°C | 8.7 MPa |
| 350°C | 16.6 MPa |

Pressures are estimates derived from water vapour pressure under the authors' stated assumption that gas production is negligible.

## Measurement protocol and benchmark tasks

The source describes SEM photomicrographs, filament length/width measurements, EDS elemental mapping and XRD.

- SEM: Hitachi TM4000, 15 kV, average working distance 7 mm, normally ×500 magnification. Up to 50 images per sample; 300 filaments measured per sample. Lower magnification was used where a filament exceeded the field of view.
- EDS: Fe, Si and O composition and mapping.
- XRD: Mo source, 50 kV, 40 mA; transmission geometry; 2–39.5° 2θ; 0.015° step; 2,300–5,000 seconds per step.

The manifest freezes four challenge tasks:

1. **Morphology-only false positives:** can morphology-only detectors distinguish abiotic filaments from a biological comparator?
2. **Diagenetic preservation robustness:** does the classifier remain calibrated as morphology changes under artificial maturation?
3. **Multimodal discrimination:** do EDS/XRD observations add information beyond morphology alone?
4. **Partial observability:** how do image resolution, magnification, field of view and missing modalities change risk and abstention?

The biological comparison uses *Leptothrix ochracea* sheaths. The authors explicitly warn that this is not representative of all biological microstructures. OLI must not overgeneralize one comparator into a universal biological morphology class.

## Anti-leakage and qualification rules

- Split by physical sample, preparation batch and maturation run. Image crops, individual filaments and repeated EDS spectra from the same specimen are not independent samples.
- Hold out entire maturation temperatures or independent preparation runs when claiming generalization.
- Preserve instrument, magnification, field-of-view, sample-preparation and maturity metadata.
- Report confusion matrices by temperature and modality, calibration, false-positive rate, false-negative rate, risk-coverage, and abstention.
- Do not let a model train on one image crop and test on another crop from the same physical specimen.
- Morphology alone must never be treated as conclusive proof of biogenicity.
- Include chemical/isotopic or geological context as complementary evidence when morphology remains ambiguous.

## Data admission and rights

The article reports that supporting data are available from the corresponding author upon reasonable request. Supporting information includes SEM photomicrographs and EDS/XRD figures.

The authors state a CC BY licence was applied to the Author Accepted Manuscript version. That statement is not a blanket licence for raw images, unpublished tables or derived sample-level measurements. This branch contains no original SEM, EDS, XRD or measurement table. Data acquisition must request exact files, sample IDs, metadata, checksums and artifact-specific terms before ingestion or redistribution.

## What this unlocks

The Gray–Scott control challenges abiotic spatial self-organization in a simulated chemical field. The chemical-garden challenge adds a laboratory-anchored mineral precipitation system with a real preservation protocol and microscopy/mineralogy evidence. Together they begin to cover two very different routes to life-like structure:

- dynamical pattern formation in a driven reaction–diffusion field;
- mineralized abiotic filament growth with preservation through simulated geological maturation.

The next family should not just produce another attractive image. It should force a detector to distinguish mechanisms using perturbations, material composition, temporal development, preservation history and observation limits.

## Claim ceiling

This contribution establishes a source-pinned benchmark specification and acquisition protocol only. It does not reproduce the paper's experiment, contain the underlying sample-level data, demonstrate natural chemical gardens, or validate an OLI classifier. The output is not a life verdict.
