# Photon Delivery Reference v0.1

A dependency-free Rust analytical budget tool comparing solar/optical photon delivery plus supplemental LEDs with LED-only delivery for a declared canopy DLI requirement.

It separates **requested design dose** from **dose physically deliverable under the declared lighting power and daily-energy caps**. This distinction matters: a plan can have enough daily kWh but insufficient peak kW, leaving the canopy below its target DLI.

The tool deliberately stops at a simplified source-to-canopy photon budget. It does not predict crop growth or yield.

## Run

```sh
cargo test
cargo run --release
cargo run --release -- --input fixtures/synthetic-lunar-cavern-hybrid-v0-1.input
```

## Model

The source input is photosynthetic photon flux density (PPFD) at the declared collector plane, in µmol m^-2 s^-1. The code does not infer PPFD from broadband solar irradiance: spectrum must be measured or supplied by a separate, source-versioned model.

Mean solar DLI is:

```text
solar_mean_DLI =
    source_PPFD
  × collector_area
  × illumination_seconds
  × collector_efficiency
  × relay_transmission
  × optical_availability
  ÷ canopy_area
  ÷ 1,000,000
```

The solar canopy-uniformity floor converts mean solar DLI into a conservative minimum-zone estimate. The LED supplement is then sized to close the minimum-zone DLI gap. LED fixture efficacy is supplied at the declared electrical meter boundary; a separate LED uniformity floor converts its mean DLI into a conservative minimum-zone contribution.

The tool reports:
- the LED power and energy required to meet the target;
- the available LED power and energy after reserving optical auxiliary loads;
- the actual LED energy deliverable under both limits;
- the resource-constrained DLI and remaining shortfall;
- independent daily-energy and instantaneous-peak-power feasibility flags.

A nominal/requested DLI is **not** marked achieved merely because the lighting plan asks for it. The `dli_met` result is based on resource-constrained delivered DLI.

## Frozen synthetic arithmetic control

All values are synthetic arithmetic controls, not measured performance:
- target DLI: 20 mol m^-2 day^-1;
- canopy area: 1,000 m²;
- PPFD at collector plane: 1,000 µmol m^-2 s^-1 for 6 h/day;
- collector area: 2,000 m²;
- collector efficiency: 0.8;
- relay transmission: 0.5;
- optical availability: 0.9;
- solar canopy minimum/mean uniformity floor: 0.8;
- LED efficacy: 2.5 µmol/J;
- LED minimum/mean canopy uniformity floor: 1.0;
- LED operating window: 16 h/day;
- optical auxiliary energy: 20 kWh/day;
- optical auxiliary peak power: 0.8 kW;
- available lighting power: 40 kW;
- available lighting energy: 960 kWh/day.

The requested target-dose comparison gives:
- mean solar DLI: 15.552 mol m^-2 day^-1;
- conservative minimum-zone solar DLI: 12.4416;
- requested LED supplement: 7.5584 mol m^-2 day^-1;
- LED-only energy for the same target: 2,222.222 kWh/day;
- requested hybrid LED energy: 839.822 kWh/day;
- optical auxiliaries: 20 kWh/day;
- requested hybrid lighting energy: 859.822 kWh/day;
- requested hybrid peak: 53.289 kW.

However, the available 40 kW lighting limit leaves only 39.2 kW after the optical auxiliary peak. Over the 16-hour LED window this permits at most 627.2 kWh of LED energy. Therefore:
- delivered LED energy: 627.2 kWh/day;
- resource-constrained hybrid lighting energy: 647.2 kWh/day;
- delivered minimum-zone DLI: 18.0864 mol m^-2 day^-1;
- remaining DLI shortfall: 1.9136 mol m^-2 day^-1;
- daily-energy budget check: feasible;
- peak-power check: infeasible;
- target DLI actually achieved under these constraints: **no**.

This is the key demonstration: an energy budget can pass while peak power prevents the required crop-light dose from being delivered. The nominal energy-saving percentage is a requested-service comparison, not evidence of an achieved equal-service farm improvement in this peak-constrained case.

These values do not describe an actual lunar site, actual collector/relay efficiencies, an actual crop, or qualified hardware. The DLI target, photon efficacy, uniformity floors and power caps are scenario assumptions.

## Important omissions

- Site-specific ephemeris, topography/horizon, eclipses and seasonal illumination;
- measured solar spectrum and conversion from irradiance to PPFD;
- spatial ray tracing and optical thermal safety;
- actual mirror/fibre/light-pipe degradation, dust abrasion and cleaning intervals;
- crop-specific DLI, photoperiod, spectral, dark-period and growth-response models;
- LED dimming curves, fixture heat, driver efficiency and lifetime;
- battery charge/discharge efficiency, ageing, thermal limits and power dispatch;
- HVAC, dehumidification, heat rejection and full facility boundary;
- time-resolved lighting schedules, overlapping solar/LED periods and photoperiod compliance;
- crop yield, nutrient composition, food safety and accepted edible output.

## Evidence ceiling

Evidence class: synthetic analytical budget. No optical-hardware performance, crop-yield, site suitability, lunar illumination, food-service, farm economics or flight-readiness claim is implied.
