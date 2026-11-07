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


## 9. Orthogonal 24-hour photoperiod / PPFD comparator: continuous low-intensity lighting

Palmitessa et al., *Continuous lighting at low PPFD improves energy efficiency while preserving growth and quality of lettuce in vertical farming systems*, Frontiers in Plant Science (4 March 2026), DOI 10.3389/fpls.2026.1783548:
- Full text: https://www.frontiersin.org/journals/plant-science/articles/10.3389/fpls.2026.1783548/full
- DOI: https://doi.org/10.3389/fpls.2026.1783548

This is a distinct 24-hour schedule comparison and must not be merged with the 27-hour circadian treatment study above. The three treatments all had the same target DLI (14.4 ±0.4 mol m^-2 day^-1):
- 16:8 at 250 ±10 µmol m^-2 s^-1;
- 12:12 at 340 ±12 µmol m^-2 s^-1;
- 24:0 at 170 ±8 µmol m^-2 s^-1.

The trial used one walk-in growth chamber, three racks and a split-plot layout. The schematic shows three blocks with all three lighting treatments; each shelf hosted 16 plants (eight per cultivar) at 33.3 plants m^-2. Lighting treatments were the main plots; cultivars (Falstaff and Copacabana) were subplots. Four plants per experimental unit were destructively measured at 26 days after transplant. Two trials were conducted, but the paper reports the second controlled-chamber trial only. It is not multi-facility validation.

### Reported outcomes (second trial)

| Metric | 16:8 | 12:12 | 24:0 |
|---|---:|---:|---:|
| Fresh weight (g/plant) | 38.5 ±1.8 | 38.0 ±1.8 | 42.0 ±2.2 |
| Yield (g/m²) | 1,271 ±44.2 | 1,255 ±43.9 | 1,390 ±54.9 |
| Dry matter (%) | 7.1 ±0.31 | 6.9 ±0.34 | 7.1 ±0.38 |
| LED EUE (g fresh mass / LED kWh) | 9.8 ±1.07 | 9.0 ±0.98 | 11.4 ±1.38 |
| Light-use efficiency (g/mol photons) | 3.4 ±0.21 | 3.4 ±0.22 | 3.8 ±0.22 |
| LED energy per m² (source table values) | 129 ±10.02 | 140 ±17.29 | 122 ±12.25 |
| Tariff-weighted LED cost (€/m²) | 18.2 ±1.53 | 19.8 ±2.44 | 16.3 ±1.79 |

The Results text states that the photoperiod treatments did not significantly influence yield (p = 0.08), morphology or dry matter. It reports significantly higher LED-only EUE under 24:0 (11.4 vs 9.8 / 9.0 g/kWh) and higher LUE (3.8 vs 3.4 g/mol). LED consumption itself was not significantly different among schedules (P marked ns). The authors attribute the tariff-weighted lighting-cost difference to the Italian hourly electricity tariff applied to the schedules.

### Source-boundary cautions

- EUE is explicitly based on plant fresh weight divided by LED electricity; it is not whole-facility kWh/kg. The methods state that HVAC, pumps and fertigation were monitored, but the headline EUE / Table 4 does not report a complete whole-facility energy comparison.
- The Table 4 energy unit is printed as kW m^-2, while the methods describe energy consumption in kWh and the results prose states approximately 130 kWh m^-2 over the growth cycle. This is an internal source-label inconsistency. The fixture preserves the values and records the conflict; it does not silently relabel them as power or energy.
- Tariff-weighted euro cost is location- and time-of-use-specific; do not transfer Italian tariffs or savings to another electricity market.
- A reported EUE improvement does not imply significant yield gain or lower total facility energy. Peak kW, LED kWh, HVAC/dehumidification kWh, pumping, fixed loads and tariff exposure are separate axes.
- The study is lettuce / two cultivars / one walk-in chamber. Continuous light is not a universal recommendation for all species or cultivars.

### Integration rule

Treat lighting optimization as a product space with orthogonal dimensions:
- endogenous cultivar clock and external zeitgeber cycle length;
- photoperiod and PPFD waveform within that cycle;
- spectral composition over time;
- DLI per biological cycle and per elapsed civil day;
- LED electrical load, peak demand, tariff and carbon-intensity timing;
- climate / latent loads and full-facility energy;
- accepted fresh mass, dry biomass, marketable fraction and nutritional quality.

Do not combine continuous 24:0, dynamic spectrum and 27 h circadian alignment into one treatment label. Pre-register which dimension is changed and hold the others fixed in a confirmatory contrast.


## 10. Resource-timed light and climate co-scheduling

The crop schedule is a time series, not a single photoperiod scalar. Compose the biology, photon delivery, facility climate and utility signals while preserving independent units and ownership:

- #4060 owns cultivar clock / phase, cycle duration, photoperiod, crop quality and calendar-time service.
- #3985 owns delivered photon flux / spectrum, DLI, LED electrical load and peak power.
- #4065 owns climate-zone dynamics, sensible/latent load, HVAC/dehumidification, and heat rejection.
- #3957 owns the integrated whole-facility ledger, tariff, meter reconciliation, accepted crop output and final service metrics.
- #1916 owns multi-period thermal integration / storage dispatch where applicable; #1970 remains the lunar energy-reserve/dispatch owner.

Compare fixed 16:8, equal-DLI 12:12, equal-DLI 24:0 (only crop-qualified), source-qualified 27 h circadian-aligned schedules, dynamic spectra, tariff-responsive schedules, carbon-responsive schedules and joint climate/peak-aware schedules under identical crop/service, budget and weather/price inputs.

The objective is not to maximize arbitrage. Hard constraints include crop-specific biological limits, DLI and spectrum, accepted-food quality, climate / condensation, power caps, reserves and preservation/safety rules. A lowest-price-only policy is a negative control and must be rejected when it violates any such constraint.

Keep the following axes distinct:
- LED kWh, instantaneous/interval LED kW and peak demand;
- whole-facility kWh and peak kW;
- tariff-weighted energy cost and demand charges, with currency/time/contract version;
- operational emissions only when an interval-matched, geographically applicable, source-qualified grid carbon-intensity series exists;
- useful heat/cooling, rejected heat and dehumidification / water burden;
- crop fresh mass, dry biomass, marketable fraction and nutritional service;
- kg/m²/calendar-year, crop-cycle duration and outage continuity.

Tariff arbitrage is not energy efficiency. Shifting load to another interval is not automatically emissions reduction. LED-only EUE is not whole-facility EUE.

### Equal-budget benchmark

Freeze paired weather, tariff, grid-carbon and outage profiles. Compare:
1. fixed schedule under incumbent control;
2. crop-only schedule without price/carbon response;
3. tariff-responsive schedule with hard crop/climate constraints;
4. carbon-responsive schedule where carbon data are qualified;
5. joint schedule with measured climate and peak-power feedback;
6. greedy lowest-price / lowest-carbon policy as an adversarial baseline;
7. exact oracle on small, declared discrete schedule cases.

Charge optimizer computation, forecasts, storage losses, failed runs, measurement and actuation overhead. Hold out entire weather / tariff / carbon profiles. Use independent plant/crop data for validation. Preserve schedule decisions and outcome traces by interval.

Advance only if a candidate improves a pre-registered Pareto dimension (whole-facility cost, peak demand, or time-matched operational emissions) without degrading biological/quality, climate, reliability, energy or water constraints beyond approved tolerances. Do not set a universal percentage target before actual facility tariff, grid, climate and crop response data are admitted.
