# Photon Delivery Reference v0.1

A dependency-free Rust analytical budget tool comparing solar/optical photon delivery plus supplemental LEDs with LED-only delivery for a declared canopy DLI requirement.

It separates **requested design dose** from **dose deliverable under declared optical-auxiliary, lighting-power and daily-energy limits**. A plan is not credited with photons from an active optical system if its tracking/relay auxiliary service cannot be powered.

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

A solar canopy-uniformity floor converts mean solar DLI into a conservative minimum-zone estimate. The LED supplement is sized to close that minimum-zone DLI gap. LED fixture efficacy is supplied at the declared electrical meter boundary; a separate LED uniformity floor converts its mean DLI into a conservative minimum-zone contribution.

### Resource-constrained delivery

The requested optical auxiliary service has both a peak-kW and daily-kWh requirement. The reference calculates a conservative first-order optical-service fraction:

```text
optical_service_fraction =
  min(1,
      available_lighting_power / optical_auxiliary_peak_power,
      available_lighting_energy / optical_auxiliary_daily_energy)
```

Zero auxiliary demand on a dimension uses a fraction of 1 for that dimension. The resulting fraction scales the declared optical availability and auxiliary energy/peak together. The remaining power and daily energy are then allocated to the supplemental LEDs. LED output is recomputed from the remaining DLI gap, the LED canopy-uniformity floor, the available LED energy and the remaining peak-power envelope.

This proportional throttle is an explicit simplified assumption. It is not a time-resolved tracking controller, battery model, optical pointing model or crop photoperiod simulation. More detailed dispatch must replace it when time-series/real hardware evidence becomes available.

The tool reports:
- requested solar and LED DLI;
- requested hybrid LED and auxiliary electricity / peak power;
- optical service fraction after power and energy constraints;
- resource-constrained solar DLI and auxiliary electricity actually supported;
- LED energy deliverable from the remaining budget;
- resource-constrained minimum-zone DLI and remaining shortfall;
- separate daily-energy and instantaneous-peak-power feasibility flags.

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

In the reference fixture the auxiliary service itself is supportable (optical service fraction = 1). However, the 40 kW total lighting limit leaves only 39.2 kW after the optical auxiliary peak. Over the 16-hour LED window this permits at most 627.2 kWh of LED energy. Therefore:
- delivered LED energy: 627.2 kWh/day;
- resource-constrained hybrid lighting energy: 647.2 kWh/day;
- delivered minimum-zone DLI: 18.0864 mol m^-2 day^-1;
- remaining DLI shortfall: 1.9136 mol m^-2 day^-1;
- daily-energy budget check: feasible;
- peak-power check: infeasible;
- target DLI actually achieved under these constraints: **no**.

A separate negative control halves available optical auxiliary peak power; the optical service fraction falls to 0.5 and the minimum-zone solar DLI halves to 6.2208 mol m^-2 day^-1 before any LED supplement is counted. Other tests show that daily-energy limits can bind independently, poor LED uniformity raises required power, and high optical auxiliary loads can make the hybrid request worse than LED-only.

The nominal/requested energy-saving percentage is a requested-service comparison, not evidence of an achieved equal-service farm improvement whenever resource constraints prevent DLI delivery.

These values do not describe an actual lunar site, actual collector/relay efficiencies, an actual crop, or qualified hardware. DLI target, photon efficacy, uniformity floors, optical auxiliary loads and power caps are scenario assumptions.

## Important omissions

- Site-specific ephemeris, topography/horizon, eclipses and seasonal illumination;
- measured solar spectrum and conversion from irradiance to PPFD;
- spatial ray tracing and optical thermal safety;
- actual mirror/fibre/light-pipe degradation, dust abrasion and cleaning intervals;
- crop-specific DLI, photoperiod, spectral, dark-period and growth-response models;
- time-resolved lighting schedules, overlapping solar/LED periods and photoperiod compliance;
- LED dimming curves, fixture heat, driver efficiency and lifetime;
- battery charge/discharge efficiency, ageing, thermal limits and power dispatch;
- HVAC, dehumidification, heat rejection and full facility boundary;
- crop yield, nutrient composition, food safety and accepted edible output.

## Evidence ceiling

Evidence class: synthetic analytical budget. No optical-hardware performance, crop-yield, site suitability, lunar illumination, food-service, farm economics or flight-readiness claim is implied.
