# EHD-PIV-TRACER-003 — exact-pressure PIV tracer feasibility screen

**Status:** deterministic Rust lookup kernel and eleven tests authored; Rust execution and exact-head CI pending  
**Parent:** EHD-PIV-TRACER-002 / Noelle et al. 2025 low-density air PIV cohort  
**Observer qualification:** EHD-DPM-001

## Why make the table executable?

The Noelle et al. study reports a useful ten-point pressure-by-tracer PIV interrogation-window table. A prose statement such as "SiO2 works to 0.3 atm" hides important differences:
- whole-field extraction versus edge-case-only vectors;
- a small interrogation window versus coarse spatial resolution;
- unavailable seeding versus a row where no value was reported;
- optical feasibility versus electrical passivity;
- a velocity map versus a validated boundary-layer field.

The kernel turns the paper's table into an exact-pressure eligibility screen while preserving those distinctions.

## Screening contract

Inputs:
- exact pressure (atm);
- tracer family;
- largest acceptable interrogation-window size in pixels;
- whether full-field coverage is required;
- whether boundary-layer detail is required.

The screen performs exact lookup only. A pressure between the ten source points returns UnsupportedSourcePressure; it is never interpolated.

For source notation:
- a plain value (e.g. 64) is whole-field availability;
- a parenthesized value before a dash is edge-case availability, not whole-field;
- a pair such as (64)-128 retains both edge-case 64 and whole-field 128;
- a bare parenthesized value is edge-case-only;
- a dash is unavailable;
- "no data" is a separate state.

The result can say FullFieldResolutionAvailable, EdgeCaseOnly, InsufficientResolution, UnavailableAtPressure, NoSourceData, UnsupportedSourcePressure, OutOfEnvelope or BoundaryLayerNotResolved.

Even a FullFieldResolutionAvailable result is only an optical/interrogation-window feasibility result. Every result explicitly requires an independent observer-perturbation receipt; electrical passivity remains NotAssessed.

## Source-bounded examples

| Pressure | Tracer | Requested maximum window | Screen result |
|---:|---|---:|---|
| 0.30 atm | SiO2 | 128 px, full field | Full-field resolution available |
| 0.30 atm | Incense sticks | 64 px, full field | Edge-case only; not eligible |
| 0.30 atm | DEHS | 128 px | Unavailable |
| 0.50 atm | Vaporized fog | 128 px | Unavailable |
| 0.80 atm | Vaporized fog | 64 px, edge-case accepted | Edge-case only |
| 0.45 atm | SiO2 | 128 px | No source data |
| 0.33 atm | SiO2 | 128 px | Unsupported pressure; no interpolation |

The paper notes that its minimum tested 32×32 px window was still insufficient to resolve its boundary layer. Any request for boundary-layer detail therefore returns BoundaryLayerNotResolved, even when the broader velocity field is source-feasible.

## Paradigm shift

The selector separates four different questions:
1. Is there enough optical seeding for a vector field?
2. Does the reported interrogation scale satisfy the intended flow-field resolution?
3. Does the chosen method perturb the plasma/flow state? This is not assessed by this screen.
4. Does the spatial footprint resolve the specific physical feature being claimed?

A tracer is not "best" in the abstract. The source shows a pressure-, geometry-, optics- and objective-dependent feasibility envelope.

## Next benchmark work

- Bind the screen to an observer-perturbation receipt and a separate Pitot / hot-wire calibration receipt.
- Add an image-correlation quality score only when raw images and masks are available; do not infer it from interrogation-window labels.
- Add a boundary-layer resolution criterion tied to local measured thickness and interrogation footprint.
- Expand to a second geometry and test whether the pressure/tracer envelope transfers.
- Add a controlled corona/tracer passivity test over the intended voltage range; the source's 8 kV control was deliberately a low-thrust condition.
- Keep low-density air separate from Martian CO2, true vacuum and atmospheric corona cohorts.

The present kernel is a deterministic source-table screen, not a learned tracer recommender and not an experimental authorization.
