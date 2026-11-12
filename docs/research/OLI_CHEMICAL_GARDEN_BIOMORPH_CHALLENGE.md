# OLI chemical-garden biomorph and taphonomy challenge v0

**Status:** literature-pinned benchmark specification; raw data access pending  
**Owner:** Symtropy / OLI abiotic controls  
**Related:** [OLI abiotic look-alike tournament #4572](https://github.com/Luminous-Dynamics/symtropy/issues/4572)  
**Machine-readable manifest:** `benchmarks/oli/chemical-garden-ferrihydrite-v0.json`

## Why this benchmark matters

Two complementary studies create an unusually strong, evidence-backed challenge for morphology-based biosignature interpretation.

1. Hirsch et al. (2026), *Geobiology*, DOI [10.1111/gbi.70045](https://doi.org/10.1111/gbi.70045), report that laboratory iron-mineralized chemical-garden filaments retain recognizable morphology through hydrous pyrolysis at temperatures up to 300°C. Complete filaments were not clearly discernible at 350°C. The authors explicitly state that the specific laboratory growth procedure is not geochemically naturalistic and that iron-mineralized chemical-garden formation has not been directly observed in nature. The data underlying the 2026 paper are available from the corresponding author upon reasonable request; a 2.8 MB supporting-information DOCX is listed by the publisher.

2. Podbielski et al. (2025), *Geobiology*, DOI [10.1111/gbi.70021](https://doi.org/10.1111/gbi.70021), compare ferruginous chemical gardens with two Leptothrix sources: cultured *L. cholodnii* SP-6 and a wild *Leptothrix* sp. isolate. Their evidence suggests discriminators beyond superficial filament morphology: the diameter distribution, cross-sectional microtexture, iron redox state and context of formation.

A separate preservation comparator is Hirsch et al. (2025), *Astrobiology*, DOI [10.1089/ast.2024.0098](https://doi.org/10.1089/ast.2024.0098), which investigates the maturation response of biological iron-oxidizing bacterial sheaths.

## The key paradigm shift: from morphology classification to causal/taphonomic discrimination

A classifier that sees a hollow filament, branching, a circular cross-section or a sinuous trajectory cannot infer biogenicity from those features alone. Abiotic chemical gardens can reproduce them.

The benchmark therefore tests four layers:

1. **Morphology:** diameter distribution, aspect ratio, tortuosity, bending, branching, anastomosis and filament continuity.
2. **Wall microtexture:** inner versus outer surface density, roughness and porosity, with cross-sectional orientation preserved.
3. **Mineralogy/redox:** ferrihydrite and silica phases, Fe²⁺/Fe³⁺ ratios, Mössbauer spectra, EDS and XRD.
4. **Formation and preservation history:** precursor salt, silicate versus carbonate, pH, oxygen exposure, maturation temperature/pressure/time, silica content, sample age and preparation history.

This is not an assertion that one of these features is a universal biosignature. It is a test of whether a combination of independent evidence, conditioned on geological and experimental context, can discriminate the specific known mechanisms in the benchmark.

## What the published data actually say

### 2026 artificial-maturation study

- Chemical gardens were generated from iron(II) sulfate heptahydrate and alkaline sodium-silicate solution.
- The artificial maturation series used seven tubes and seven temperature treatments, 50–350°C, for 72 hours under sealed, anoxic, hydrous conditions.
- Up to 50 SEM images were acquired per sample, and 300 filaments per sample were measured.
- Mean filament lengths reported for unheated, 50, 100, 150, 200, 250 and 300°C samples were 46.2, 42.39, 41.77, 39.23, 41.6, 24.94 and 28.79 μm respectively.
- Mean maximum widths reported were 2.63, 2.66, 2.45, 2.55, 4.06, 3.26 and 3.83 μm.
- Filament lengths were not measurable at 350°C because complete filaments could not be confidently discerned.
- XRD identified ferrihydrite through 250°C; at 300°C, cristobalite/tridymite were present and minor peaks were tentatively assigned to haematite; at 350°C iron-bearing phases could not be confidently established because strong silica peaks obscured interpretation.

**Experimental-unit caveat:** 300 filaments are nested measurements, up to 50 image fields, within each treatment sample. The article describes seven glass tubes across seven temperatures but does not establish an independently replicated tube/batch count per temperature in the examined text. The 300 filaments must not be treated as 300 independent treatment replicates. The authors' reported filament-level Student t-tests should be reproduced as source outputs but not used as independent-treatment uncertainty estimates without clarifying sample hierarchy.

### 2025 direct chemical-garden versus Leptothrix study

- The biological samples were *L. cholodnii* SP-6 (ATCC 51168) and a wild *Leptothrix* sp. isolate. This is not a four-species experimental panel.
- Mature *Leptothrix* sheaths had external diameters up to 1.7 μm, median 1.1 μm (n=98), whereas the chemical gardens ranged to 11.8 μm with median 3.7 μm in that experiment.
- *Leptothrix* showed a narrower diameter/tortuosity range, but bending and tortuosity overlap; no single scalar is a perfect discriminator.
- Leptothrix sheaths typically showed smoother, denser inner walls and rougher, more porous outer walls. Chemical-garden tubules tended toward the reverse.
- Mössbauer spectroscopy reported Fe²⁺/Fe_total of 0% in *L. cholodnii* SP-6, 4.1% in carbonate gardens and 19.8% in silicate gardens (estimated error approximately 2%).

**Redox context caveat:** the paper notes that the chemical-garden experiment was aerobic. Under anoxic preparation and protected analysis, Fe redox could more closely reflect the starting salt. Silicate versus carbonate chemistry also changes Fe redox. Therefore Fe²⁺ is a context-conditioned feature, not a universal life marker.

### Biological preservation comparator

The 2026 paper reports that the *Leptothrix* bacterial sheaths in the 2025 hydrous-pyrolysis study could not be observed after maturation at 280°C, while chemical-garden filaments remained discernible at 300°C. This is a comparison between particular materials and experimental conditions, not a general law that abiotic filaments outlast all biological structures.

## Benchmark tasks

The JSON manifest specifies six tasks:
- morphology-only false-positive challenge;
- diagenetic-preservation robustness;
- multimodal discrimination;
- partial-observability stress testing;
- conditioned multimodal discrimination;
- redox context shift.

Natural geological materials shown in the 2025 paper (moss agate, limestone and ironstone tubules) are **ambiguous/unresolved cases**, not known-truth abiotic or biological labels. They should be retained as open-set challenge data and only relabelled with independent evidence.

## Required dataset hierarchy and anti-leakage

The benchmark must preserve:
`preparation batch → reaction vessel / maturation tube → sample → image / field of view → filament`.

Training/test splits and confidence intervals must be grouped at the highest genuinely independent physical unit. Image crops or individual filaments from one specimen may never be distributed across train and test. If the 2026 source has only one tube per temperature, leave-temperature-out analysis is a descriptive stress test, not a replicated generalization estimate.

For the 2025 comparison, preserve biological strain/source, culture age, growth medium, anion, oxygen history, imaging modality, and independent preparation batch. Geological samples remain OTHER/UNRESOLVED.

## Data admission

- 2026 sample-level images and data: available from corresponding author upon reasonable request.
- 2026 supporting information: publisher lists `gbi70045-sup-0001-FigureS1-S4.docx` (SEM/EDS/XRD figures). Automated retrieval returned HTTP 403 during this pass; no file was downloaded.
- 2025 direct comparison: obtain raw SEM/TEM/optical images, morphometry tables, Mössbauer spectra and sample-level metadata separately.
- 2025 biological preservation comparator: obtain its image/morphometry/mineralogy data and exact sample hierarchy separately.

No data have been redistributed or materialized by this contribution. The manifest records each source and pending data-admission obligation.

## Qualification targets

These are proposed, not achieved:
- 100% traceability of every image/measurement to a physical sample and acquisition configuration.
- Zero train/test leakage across physical samples, preparation batches or maturation tubes.
- Explicit per-source and per-condition calibration and risk-coverage curves.
- Zero morphology-only positive-biogenicity promotions on the abiotic chemical-garden controls.
- Demonstrated incremental value of microtexture/mineralogy/redox beyond morphology alone, with an ablation and uncertainty intervals.
- Robust abstention when oxygen history, anion chemistry, sample hierarchy or required modality is unavailable.
- No false claim that the *Leptothrix* comparison represents all biological microstructures.

## Claim ceiling

This is a literature-grounded laboratory abiotic look-alike benchmark specification. It does not show that chemical gardens occur naturally, does not prove that any geological filament is abiotic, does not demonstrate extraterrestrial life, and does not create a universal fossil classifier.
