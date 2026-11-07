# Photon Delivery Reference v0.1

A dependency-free Rust analytical budget tool comparing solar/optical photon delivery plus supplemental LEDs with LED-only delivery for a declared canopy DLI requirement.

It deliberately stops at photons delivered to the canopy. It does not predict crop growth or yield.

## Run

```sh
cargo test
cargo run --release
cargo run --release -- --input fixtures/synthetic-lunar-cavern-hybrid-v0-1.input
```

## Model

The source input is photosynthetic photon flux density (PPFD) at the declared collector plane, in µmol m^-2 s^-1. The code does not infer PPFD from broadband solar irradiance: spectrum must be measured or supplied by a separate model.

The mean solar DLI is:

```text
mean_DLI =
    source_PPFD
  × collector_area
  × illumination_seconds
  × collector_efficiency
  × relay_transmission
  × optical_availability
  ÷ canopy_area
  ÷ 1,000,000
```

A declared canopy-uniformity floor converts this mean to a conservative minimum-zone estimate. Supplemental LED light is sized to bring that minimum to the declared target DLI. The LED electrical input is computed from a declared fixture photon efficacy in µmol/J. Peak LED power is evaluated over the declared LED operating window; optical auxiliary energy and peak power are separately included.

## Frozen synthetic fixture

All numbers are synthetic arithmetic controls:
- target DLI: 20 mol m^-2 day^-1;
- canopy area: 1,000 m²;
- PPFD at collector plane: 1,000 µmol m^-2 s^-1 for 6 hours/day;
- collector area: 2,000 m²;
- collector efficiency: 0.8;
- relay transmission: 0.5;
- optical availability: 0.9;
- canopy minimum/mean uniformity factor: 0.8;
- LED efficacy: 2.5 µmol/J;
- LED operating window: 16 hours/day;
- optical auxiliary electricity: 20 kWh/day.

This arithmetic fixture yields:
- mean solar DLI: 15.552 mol m^-2 day^-1;
- conservative minimum-zone DLI: 12.4416 mol m^-2 day^-1;
- LED supplemental DLI: 7.5584 mol m^-2 day^-1;
- LED-only electricity: 2,222.222 kWh/day;
- hybrid LED electricity: 839.822 kWh/day;
- hybrid lighting plus optical auxiliaries: 859.822 kWh/day;
- hybrid LED peak plus declared optical peak: 53.289 kW.

Under the fixture's declared 40 kW power limit and 960 kWh/day lighting-energy budget, the hybrid design passes the daily energy check but fails the peak-power check. The model therefore demonstrates why average energy and peak power must not be collapsed. Storage or a different photoperiod/optical architecture could change that result, but storage dynamics and crop physiology are not modeled here.

These numbers do not describe an actual lunar site, actual collector or relay efficiency, an actual crop, or a qualified farm. The 20 mol DLI and 2.5 µmol/J LED efficacy are scenario assumptions, not universal crop or hardware specifications.

## Important omissions

- Site-specific ephemeris, topography/horizon, eclipses and seasonal illumination;
- measured solar spectrum and conversion from irradiance to PPFD;
- spatial ray tracing and optical thermal safety;
- actual mirror/fibre/light-pipe degradation, dust abrasion and cleaning intervals;
- crop-specific DLI, photoperiod, spectral, dark-period and growth-response models;
- LED dimming curves, fixture heat, driver efficiency and lifetime;
- battery charge/discharge efficiency, ageing, thermal limits and power dispatch;
- HVAC, dehumidification, heat rejection and full facility boundary;
- crop yield, nutrient composition, food safety and accepted edible output.

## Evidence ceiling

Evidence class: synthetic analytical budget. No optical-hardware performance, crop-yield, site suitability, lunar illumination, food-service, farm economics or flight-readiness claim is implied.
