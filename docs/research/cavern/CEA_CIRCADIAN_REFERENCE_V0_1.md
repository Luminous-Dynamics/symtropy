# CEA circadian-cycle reference and calendar-time normalization v0.1

**Parent:** [CEA-CIRCADIAN-001 #4060](https://github.com/Luminous-Dynamics/symtropy/issues/4060)  
**Composes:** #3957 whole-facility accounting, #3985 photon delivery, #4002 analogue protocol.

**Status:** source-derived analytical reference and benchmark protocol. Not a crop-growth model, facility measurement, or independent replication.

## 1. Source reference

Anton-Sales et al., *Matching circadian rhythms to light–dark cycles increases lettuce yield by 29% in vertical farms without additional energy input*, Journal of Experimental Botany, published 12 May 2026, DOI 10.1093/jxb/erag222.

- Full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC13529320/
- DOI: https://doi.org/10.1093/jxb/erag222

The study tested 20 lettuce accessions, including previously characterized long (~27 h) and normal (~24 h) circadian phenotypes, under 24 h and 27 h environmental cycles. It reports >2,000 lettuce heads across six treatments in a commercial vertical-farm setup. Matched-photon comparisons found selected long-clock accessions accumulated more fresh mass under 27 h treatments.

## 2. Exact source-derived comparison data

The following long-clock mean comparisons are reported after 30 treatment cycles:

| Treatment | 24 h reference | 27 h resonant |
|---|---:|---:|
| Standard photoperiod | 16:8 at 250 µmol m⁻² s⁻¹ | 16:11 at 250 µmol m⁻² s⁻¹ |
| Extended photoperiod, matched light dose | 20:4 at 200 µmol m⁻² s⁻¹ | 20:7 at 200 µmol m⁻² s⁻¹ |
| Fresh mass, standard | 61 g | 68 g |
| Fresh mass, extended matched-dose | 56 g | 72 g |
| Photon dose per treatment cycle | 14.4 mol m⁻² | 14.4 mol m⁻² |
| Total photon dose over 30 cycles | 432 mol m⁻² | 432 mol m⁻² |
| Total light hours over 30 cycles | 480 h (16 h × 30) or 600 h (20 h × 30) | identical within matched pair |
| Real elapsed duration | 30 × 24 h = 720 h | 30 × 27 h = 810 h |

These are published study means, not Symtropy measurements. The 27 h treatment is 12.5% longer in wall-clock time.

## 3. Calendar-time normalization: two different results must not be conflated

The paper itself includes a **separate time- and area-corrected annual-yield analysis** in Supplementary Fig. S4. It reports 12.2 versus 10.8 crop rotations per year for 24 h versus 27 h cycles, and concludes that projected annual yield per m² was equivalent or greater for long-clock accessions grown under 27 h cycles than for the same accessions under 24 h cycles. It reports no such compensation for normal-clock accessions.

That is the paper's source-reported annualization result. It must be preserved in our benchmark and must not be replaced by a simplified ratio based on one table of 30-cycle group means.

The machine-readable fixture also includes a **limited arithmetic sanity check** using the published 30-cycle group means:

`single_harvest_mean_ratio × (24 / 27)`

- Standard 16 h photoperiod: (68 / 61) × (24 / 27) = **0.9908925**.
- Extended 20 h matched-photon photoperiod: (72 / 56) × (24 / 27) = **1.1428571**.

These are ratios for the reported mean fresh mass at the 30-cycle harvest, simply divided by elapsed crop-cycle duration. They are **not** the authors' full time- and area-corrected annual-yield calculation, are not derived from the underlying plant-level supplement, and cannot overturn or reproduce Supplementary Fig. S4 by themselves. In particular, do not summarize the paper as showing that the standard 16:11 resonant treatment reduces annual productivity: the authors' separate analysis reports equivalent or greater projected annual yield per area for long-clock accessions under 27 h cycles.

The source also notes that a 27 h cycle is 12.5% longer than a 24 h cycle, increasing climate-control time and associated cost, while lighting is estimated in the paper to represent 60–80% of vertical-farm operating expenses. This is the authors' argument about likely cost impact, not a measured whole-facility energy audit. We must measure all energy streams rather than assume the climate-control penalty is negligible.

For a true reproduction, ingest the article's raw supplementary observations and exact Supplementary Fig. S4 calculation method, preserve plant density changes and harvest-time basis, then independently recompute time-corrected and area-corrected annual yield. The present v0.1 fixture records the source's reported annualization and the simple 30-cycle arithmetic separately; it does not claim to reproduce the full annual analysis.

## 4. Why equal photon dose is not equal elapsed-time service

The paper explicitly reports that 30 cycles at 27 h require 33.75 calendar days versus 30 days at 24 h. Total light hours and cumulative photons can be equal within a matched treatment pair while the 27 h treatment occupies the growing area longer.

For a 14.4 mol m⁻² dose per treatment cycle:
- 24 h equivalent photon dose per 24 elapsed hours = 14.4 mol m⁻²;
- 27 h equivalent photon dose per 24 elapsed hours = 14.4 × 24/27 = 12.8 mol m⁻².

Therefore the benchmark must report (a) photons per biological treatment cycle, (b) photons per 24 elapsed hours, (c) crop duration, and (d) accepted output per calendar time. A result cannot be represented by only one of these values.

## 5. Important evidence boundary

The published setup used two vertical racks with five horizontal levels per rack. Treatments were assigned to individual levels separated by light-blocking curtains; plant positions were randomized with a row–column design. Harvest phenotyping reports n≥7–16 plants per accession/treatment. The accessible methods do not clearly document independently replicated room/compartment treatment units for every treatment.

Do not treat every plant as an independent treatment replicate. Our physical replication should add independent controlled compartments / bays, record rack and position effects, and use a prospective power analysis at the treatment-assignment level.

The paper's biological comparison is not a whole-facility energy study. It does not provide our needed HVAC, dehumidification, pumping, CO₂, controls, cold-storage and base-load metering. "No additional lighting energy" must not be rewritten as "no additional facility energy."

High-DLI treatments at 18 mol m⁻² per treatment cycle achieved high biomass but were reported with morphological aberrations and increased tipburn; quality and marketable fraction therefore remain hard constraints, not secondary details.

## 6. Required benchmark outputs

Report all of the following separately:
- fresh mass per treatment cycle;
- accepted edible mass after rejection / quality disposition;
- elapsed calendar time;
- crop cycles per calendar year, with actual occupancy and transition time;
- accepted edible kg/m²/year;
- photons per crop cycle and per 24 elapsed hours;
- lighting kWh per accepted edible kg;
- whole-facility electricity kWh per accepted edible kg;
- HVAC / dehumidification / pumping / CO₂ / controls / cold-storage energy;
- water, nutrient, thermal, labour and maintenance consequences;
- bolting, tipburn, morphology and quality;
- cultivar-specific treatment response and uncertainty.

Do not infer electrical consumption from PPFD alone. Meter fixture wall-plug power and facility subsystems. Preserve the distinction between measured light, modelled photon conversion and measured electricity.

## 7. Replication protocol

The next experiment should:
1. Reproduce the two equal-photon comparisons before adding new schedules.
2. Include both long-clock and normal-clock accessions; do not assume a commercial cultivar's internal period if unknown.
3. Randomize treatment to independent compartments and block by rack / location.
4. Keep plant samples nested within compartment; do not use plant count as substitute for independent treatment replication.
5. Estimate between-compartment and between-cycle variance in a pilot.
6. Select one confirmatory primary contrast and run a prospective power analysis; adjust for multiplicity if several contrasts are confirmatory.
7. Repeat across crop cycles and hold out at least one cultivar / seed lot for transfer.
8. Meter the whole facility for the full elapsed treatment duration, including the extra dark-period hours.
9. Preserve all unsuccessful, low-quality and aborted crop cycles.

## 8. Claim ceiling

This reference reproduces arithmetic from a published controlled-environment study and sets a more complete benchmark contract. It does not reproduce the article's raw data, validate its crop physiology, establish a general 27 h recommendation, predict annual farm output, or prove an energy advantage for an underground facility.
