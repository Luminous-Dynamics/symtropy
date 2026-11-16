# Garment Material Passport: NIR / Hyperspectral Identification and Care-Routing Assurance v0.1

**Status:** benchmark/data-contract proposal; no model trained, no physical scans and no process qualification.  
**Tracker:** [GARMENT-MATERIAL-PASSPORT-001 #5549](https://github.com/Luminous-Dynamics/symtropy/issues/5549).  
**Composes with:** LAUNDRY-SERVICE-001 #5471, morphology #5518, professional-care automation #5519, surface/chemistry #5527 and microfiber #5495.

## 1. Thesis: the care label is not the material truth

A care label is a manufacturer declaration and treatment instruction, not a current measurement of fibre proportions, local layers, coatings, repairs, wear or contamination. Wash count is not a measurement of present composition either.

Build an **evidence-carrying, versioned material-and-condition passport**:
1. declared identity and fibre composition;
2. instrument observations of the present textile surface;
3. model-estimated fibre fractions with uncertainty and explicit scope;
4. region/layer-specific construction observations;
5. measured change after wear, washing, repair and coating treatment;
6. process compatibility as a separate human-governed decision;
7. current material recovery / end-of-life route.

Keep separate evidence classes: manufacturer-declared, supplier-reported, instrument-observed, model-estimated and laboratory-verified. Never overwrite a declaration with a prediction or treat prediction as chemical ground truth.

## 2. Public research assets

### NIST NIR-SORT 2.0

NIST released NIR-SORT 2.0 on 19 March 2026. Its public programme describes 90 known-provenance fabrics, 12 pre-consumer fabrics and 11 post-consumer examples. It adds custom blends, dyed fabrics, new fibre types, handheld NIR, standoff benchtop measurements and colour/polarized microscopy. NIST states that digital data supports model development, while physical Research Grade Test Materials and an interlaboratory study are needed for integrated system validation.

References:
- Release: https://www.nist.gov/news-events/news/2026/03/nist-releases-nir-sort-20-enhancing-model-validation-textile-feedstock-0
- Dataset page: https://www.nist.gov/mml/mmsd/security-technologies-group/near-infrared-spectra-origin-defined-and-real-world-textiles
- Dataset DOI: https://doi.org/10.18434/mds2-3325
- Data.gov catalogue: https://catalog.data.gov/dataset/near-infrared-spectra-of-origin-defined-and-real-world-textiles-nir-sort-a-spectroscopic-a
- Scientific Data descriptor: https://doi.org/10.1038/s41597-026-07434-6

The catalogue describes nominally seven benchtop replicates in the 833–2500 nm range and handheld data in the 1550–1950 nm range. The versioned dataset README and instrument metadata remain authoritative at import time.

### OpenTextile-NIR (2026)

The 2026 OpenTextile-NIR data descriptor reports 71 post-industrial textile samples, more than 11 million spectra and more than 6 million annotated spectra, with NIR hyperspectral imagery, RGB photographs and fibre/colour metadata. It is a separate domain challenge, not an interchangeable extension of NIST data.

References: https://doi.org/10.1016/j.dib.2026.112559 and https://pmc.ncbi.nlm.nih.gov/articles/PMC12925456/

### Composition drift in used workwear (2025)

A 2025 study assessed 30 used hospital workwear garments made from a common polyester/cotton blend at manufacture. NIR hyperspectral imaging predicted present blend composition with RMSE below 0.5 wt-% against ISO-standardized chemical analysis; laundering-cycle count alone had RMSE above 1.4 wt-% in that cohort.

Reference: https://doi.org/10.1016/j.rcradv.2025.200282

This is a strong narrow-cohort reference, not a universal performance guarantee. Reproduce the declared scope first; then test broader fibres, instruments, regions and wear patterns.

## 3. Why this is more than another sorter

Public NIR datasets and textile-sorting studies address feedstock identification, mainly for recycling. Our added problem is connecting measured material evidence to **garment care without overclaiming what a spectral measurement proves**.

A garment is not a homogeneous swatch:
- shell, lining, interlining, padding, seam tape and trims may be different materials;
- coatings, dyes, soil, moisture and wear may alter measured spectra;
- a surface scan may not see hidden layers;
- an NIR fibre estimate does not establish dye fastness, adhesive compatibility, stain chemistry or solvent safety.

Use multiple local scan regions and an explicit heterogeneous/unknown state. A whole-garment average must not conceal an incompatible component.

NIR/HSI can improve material estimates and route hypotheses; it cannot independently certify dry-clean suitability or authorize a process.

## 4. Benchmark tasks

A. Fibre-family classification: cotton, polyester/PET, polyamide, polypropylene, wool, viscose/lyocell, acrylic and relevant blends, including elastane-containing fabrics where represented.

B. Blend regression: estimate current component mass fractions and uncertainty with closure constraints appropriate to the validated reference method.

C. Colour/finish robustness: custom-dyed, dark, patterned, printed, coated, finished, moisture-affected and soiled textile.

D. Instrument generalization: benchtop, handheld and standoff devices; sensor vendor/model, spectral range, illumination, distance, angle, calibration session and site.

E. Used-garment change: estimate present blend separately from original production declaration; account for differential fibre loss, contamination and coating/finish changes.

F. Region localization: distinguish shell, lining, trims and multilayer areas rather than averaging them into a false homogeneous answer.

G. Safe decision: calibrated confidence, out-of-distribution detection and abstention on unseen fibres, coatings, wet/soiled surfaces, mixed materials and unknown instruments.

## 5. Leakage-resistant validation

- Split by physical specimen, garment, source lot and acquisition session. Never randomly split spectral pixels, tiles, repeated scans or crops from one specimen across train/test.
- Keep NIST, OpenTextile-NIR and the workwear ageing cohort separately reported. Do not pool them and call the result independent validation.
- Hold out instrument models and acquisition sites; report within-instrument and cross-instrument performance.
- Use nested calibration/validation and publish preprocessing/model/pipeline hashes.
- Compare label-only, RGB-only, NIR-only, HSI-only and fused modalities; ablate colour, polarization, microscopy and metadata.
- Report per-class precision/recall, confusion matrix, macro-F1, calibration error, selective risk/coverage, abstention rate, blend RMSE/MAE/bias, interval coverage and worst-group performance.
- Test scan repeatability, operator reproducibility, distance/angle, illumination, temperature, humidity, moisture, wrinkle state, contamination and garment region.
- Blend-regression ground truth requires qualified chemical analysis on held-out specimens. Supplier labels may be provenance/prior fields but are not independent ground truth.
- Freeze dataset versions, licences, source metadata and permitted derived-artifact use before training. Do not ingest proprietary textile spectra without permission.

## 6. Metrics and proposed targets (hypotheses only)

| Metric | First target hypothesis | Stretch direction |
|---|---:|---|
| Same-scope polyester/cotton workwear blend RMSE | Reproduce published <0.5 wt-% result | Lower uncertainty without narrowing declared scope |
| Broader matched-family blend RMSE | <=1.0 wt-% | <0.5 wt-% across families/instruments, only if reference chemistry supports it |
| In-domain fibre classification macro-F1 | >=0.95 | >=0.98, with worst-class performance disclosed |
| 95% prediction-interval coverage | >=0.95 | Narrower calibrated intervals at same coverage |
| OOD challenge sensitivity | >=99.5% | Zero false-safe route assignments in the frozen adversarial corpus |
| Instrument-held-out performance | Establish baseline | Minimize domain-shift degradation |
| Region/layer attribution | Establish baseline | High-confidence localization with abstention on occlusion |

No target is accepted without pilot variance, calibration, chemical-reference uncertainty, class balance and independent holdout design.

## 7. Versioned material passport contract

Each scan event must bind:
- pseudonymous garment/specimen token (never owner identity);
- declared composition, source, confidence and label digest where available;
- garment region, construction/layer identity and known repairs/treatments;
- sensor modality, make/model/serial class, wavelength range and acquisition software version;
- calibration ID/date/reference artefact and certificate digest;
- acquisition geometry, illumination, temperature, humidity, moisture and surface condition;
- raw observation digest and immutable source URI/location;
- preprocessing pipeline digest and model artifact digest/version;
- estimated component fractions, units, uncertainty and validation domain;
- ground-truth method/lab/certificate digest, if available;
- OOD/abstention/review outcome and operator approval reference;
- timestamp and relation to prior scan, wash, repair, coating or treatment events.

Raw spectra and garment images can be privacy- or commercially sensitive. Local processing must work offline; uploading raw observations is not required for normal care decisions.

## 8. Safety and authority

- NIR/HSI and RGB are advisory material evidence, not proof every garment part tolerates a process.
- Unknown dyes, adhesives, coatings, lining, trims, stains or contamination require abstention or human review.
- No spectral model may authorize solvent, heat, steam, pressure, drum, robot or chemical actuation.
- No dataset or synthetic spectrum may be represented as a physical observation from our programme.
- Physical protection and process interlocks remain independent deterministic device controls.
- Mycelix stores provenance/custody; it does not turn a model estimate into truth.

## 9. Luminous integration

Symtropy owns scan workflow, model uncertainty, validation, process compatibility and Pareto evaluation. Symthaea proposes material hypotheses and next measurements only. Mycelix owns garment/material lot identity, supplier claims, repair/care event provenance and evidence custody. Sol-Atlas consumes qualified present-state composition for lifecycle, sorting, repair and recovery projections.

**Claim ceiling:** literature-backed benchmark and data-contract qualification only. No classifier has yet been trained or physically validated in this programme; dry-cleaning or wet-cleaning suitability cannot be inferred from NIR alone.
