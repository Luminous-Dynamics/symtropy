# Optical ISRU v0.2 — separate QCM and optical-witness evidence v0.1

**Status:** standalone Rust reference; 10 tests authored, not compiled or executed  
**Parent:** Symtropy #5656 — dynamic optical contamination, fouling and recovery qualification  
**Related:** Symtropy #5612 — typed metrology evidence graph; #5682 — bitemporal calibration assessment; Symthaea #12496 / #12525 — sealed BO/EIG and decision-value selection

## Why this exists

Optical degradation and deposition mass are related physical phenomena, but they are not the same measurement. A quartz-crystal microbalance (QCM) reports mass deposited on its own crystal. It does not, by itself, establish the optical transmission of a production optic, identify deposit chemistry, or measure the impact of particles on another surface.

The 2025 ECSS contamination standard Rev.1 describes QCM and witness monitoring, and says witness temperature and location should be representative of the item being assessed. It also notes that mass-spectrometer data alone are not sufficient to monitor condensable contaminants. These are methodological foundations, not ISRU-specific thermal/vacuum acceptance limits: https://ecss.nl/standard/ecss-q-st-70-01c-cleanliness-and-contamination-control-15-october-2025/

## First implementation

`research/asteroid-optical-isru-v0.2/contamination_evidence.rs` defines independent typed observations and comparison functions:

### Optical witness

A direct spectral-transmission reading binds:
- witness identity and location;
- wavelength;
- orientation and temperature;
- measurement timestamp;
- transmittance and an absolute uncertainty bound;
- calibration eligibility at the measurement time; and
- an upstream calibration-receipt reference.

A before/after comparison requires the same witness, location and wavelength, and constrains temperature and orientation mismatch. If the conservative uncertainty interval does not establish a positive or negative change, the result is **not distinguished from zero**; it is not silently reported as no change.

The uncertainty bound uses the sum of the two absolute bounds multiplied by a caller-declared factor. This avoids assuming independent measurement errors. It is conservative and may be loose; a future covariance-aware model must use traceable covariance evidence rather than assume independence.

### QCM

A deposition comparison binds the same QCM crystal, location, temperature/orientation envelope, timestamps, calibration eligibility and calibration-receipt references. It reports only a local QCM areal-mass change.

The implementation does not derive optical transmission from QCM mass. It does not identify molecular/particulate species, infer deposition on a different optic, estimate deposition flux from one paired difference, or assume a linear mass-to-optical-loss relation.

## Adversarial examples encoded in tests

1. QCM deposition increases while directly measured witness transmission change remains unresolved.
2. Optical-witness transmission decreases while QCM deposition is not distinguished from zero.
3. A weak apparent optical decline remains inside the conservative uncertainty bound.
4. A calibration status of Unknown blocks comparison.
5. Different witness identities, wavelength bands, temperatures or orientations cannot be compared as one trace.
6. Reversed time order, invalid uncertainty and invalid comparison factors are rejected.

These tests are important because both false positives matter: a QCM can increase without a demonstrated optical effect, and an optical path can degrade without a corresponding QCM increase at the monitored location.

## Claim boundary

The status `EligibleAtMeasurementTime` is an upstream declared result from the bitemporal calibration/evidence path. This file does not authenticate the verifier or prove physical calibration.

A measured decrease in witness transmittance establishes a change on that witness under the matched conditions. It does not prove that the production optic changed by the same amount or that contamination caused the change. Temperature, surface mismatch, morphology, particulate geometry, illumination distribution and witness representativeness remain experimental questions.

The code does not command cleaning, heating, bakeout, shield deployment, beam exposure or any other physical intervention. Candidate interventions require a rig-specific hazard review and separate authorization.

## Reproduce locally

From the repository root:

```sh
rustc --edition=2021 --test research/asteroid-optical-isru-v0.2/contamination_evidence.rs -o /tmp/isru-fouling-tests
/tmp/isru-fouling-tests
```

**Validation honesty:** ten Rust tests are authored but have not been compiled or executed in the authoring environment because rustc/cargo are unavailable there. No physical contamination, QCM sensitivity, optical recovery, acceptance threshold or lifetime result is claimed.

## Next work under #5656

1. Add actual surface geometry, representative witness placement and the production-optic mapping as an independently reviewed applicability record.
2. Add multiple spectral bands, transmittance/reflectance/scatter and source/reference-channel lineage.
3. Add explicit common-cause groups for source, optical path, calibration, power, timebase and environmental episode.
4. Add QCM crystal sensitivity/temperature/calibration records and independently characterized witness analyses.
5. Model accumulation and recovery as measured trajectories, including partial, zero and adverse recovery.
6. Link the measurements to the typed campaign/material/sample graph under #5612.
7. Give Symthaea separate likelihoods for apparatus-state and material-state hypotheses. Do not fuse QCM mass and optical loss into one feature without a validated causal model.
8. Benchmark campaign accepted-product yield per total campaign hour, including contamination checks, maintenance, requalification and downtime.

This tranche separates the measurements and prevents an invalid inference. It is not yet a dynamic fouling model or physical metrology qualification.
