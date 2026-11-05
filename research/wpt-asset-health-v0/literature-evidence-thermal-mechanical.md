# Literature evidence addendum — coupled pavement thermal/mechanical sensing

## A. Experimental binder-grade benchmark

**Source:** Pedram Balooch Sirgani, Rajib B. Mallick and Soheil Nazarian, “Thermally induced mechanical response of asphalt pavements with embedded wireless power transfer systems: influence of mixture properties,” *Applied Thermal Engineering*, August 2026, article 132099, DOI [10.1016/j.applthermaleng.2026.132099](https://doi.org/10.1016/j.applthermaleng.2026.132099).

### What was tested

Two Superpave asphalt mixtures were compared:
- PG 64–22 conventional binder.
- PG 76–22 polymer-modified binder.

The thermal stimulus used embedded closed-loop heating pads to reproduce the pavement-side heating field; these were electrically passive thermal surrogates, not energized WPT pads. Temperature was measured using embedded thermocouples and infrared imaging. Repeated loading used MMLS3, with longitudinal tensile strain recorded through distributed fibre-optic sensing. Dynamic modulus was characterized by temperature and frequency.

### Reported result

At up to a 10 °C surface-temperature rise at the slab centre, mean tensile-strain amplification was reported as 88% for PG 64–22 and 50% for PG 76–22. A peak surface temperature of approximately 44 °C was reported directly above the heating pads. The paper reports approximately 34% lower tensile strain for the PG 76–22 mixture and lower amplification under thermal and wheel-wander effects.

### Correct interpretation

This is experimental thermo-mechanical evidence under a thermal surrogate and a particular MMLS3 loading protocol. It is not a direct active WPT energy-loss/thermal experiment, universal binder selection rule, million-wheel lifetime result or transferable percentage for other mixtures. Preserve the two mixture arms and the specific thermal stimulus. Do not ingest the article's summary percentages as raw per-sensor observations.

### Benchmark consequences

The asset-health observation contract must carry:
- binder grade and mixture-design lineage;
- co-registered spatial temperature and strain samples;
- fibre-optic sensor identity and decoupling/calibration method;
- spatial coordinate frame, depth, direction and wheel-wander state;
- thermal-stimulus classification (ACTIVE_WPT_LOSS versus PASSIVE_THERMAL_SURROGATE).

A single segment maximum temperature or mean strain cannot represent localized peaks above the WPT footprint. Keep spatial profiles as records, not only scalar summary fields.

## B. Emerging distributed multi-parameter asphalt sensing

**Source:** Jiwen Zhang et al., “A novel distributed fiber optic multi-parameter sensor for asphalt pavement monitoring: Design, fabrication, and performance characterization,” *Sensors and Actuators A: Physical*, article 118304, published 1 November 2026, DOI [10.1016/j.sna.2026.118304](https://doi.org/10.1016/j.sna.2026.118304).

The published abstract describes a symmetrically packaged distributed optical fibre sensor for simultaneous strain, temperature and curvature measurement in asphalt pavement, a superposition/cross-correlation method to decouple these quantities, and a Frenet–Serret / Runge–Kutta reconstruction for two-dimensional rutting morphology. Its demonstrated applications include indoor experiments and four-point bending specimen testing. This is a sensor-method reference, not validation of a sensor installed above an operating WPT transmitter or in a long-duration public roadway.

### Benchmark consequences

- Preserve raw optical channels and decoupled strain/temperature/curvature outputs separately.
- Track sensor packaging, embedding, bond/interface and installation lineage.
- Test cross-sensitivity and calibration drift using independent reference instruments.
- Register each spatial profile to a declared roadway/pad coordinate frame.
- Evaluate spatial reconstruction resolution and uncertainty, not just scalar RMSE.
- Treat the sensor's decoupling algorithm as a measurement model, not as independent ground truth.

## C. Schema and scenario extension

The observation contract includes optional binder grade and mixture digest, fibre-optic/sensor revision, wheel-wander offset, spatial strain profiles, curvature profiles and spatial temperature profiles. The scenario manifest adds a published-measured binder comparison, synthetic spatial registration, and a cross-sensitivity negative control.

These additions are a schema/benchmark design response to the cited work. They do not claim the papers' raw datasets have been acquired or independently reproduced.
