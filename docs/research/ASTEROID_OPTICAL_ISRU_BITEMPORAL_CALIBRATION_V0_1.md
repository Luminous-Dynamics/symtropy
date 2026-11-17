# Optical ISRU v0.2 — bitemporal calibration assessment kernel v0.1

**Status:** standalone Rust reference; 11 tests authored, not compiled or executed  
**Parent:** Symtropy #5612 — typed batch, metrology and calibration evidence graph  
**Related:** Symtropy #5656 — dynamic optical contamination and fouling recovery; Symtropy PR #5589 — source-faithful observation register; Symthaea #12483 / PR #12496 — sealed experiment-selection tournament

## Why this exists

A measurement can be numerically plausible and still be unusable because its calibration was not yet valid, had expired, was superseded, or was later revoked with an earlier effective date. Looking up only the calibration's current status loses the historical distinction between what was true at measurement time and what the evidence system knew at a particular point in time.

This reference introduces a small bitemporal kernel. It does not replace the v0.1 observation register and does not mutate historical observations. It returns a separate status that downstream inference can use to decide whether a measurement is eligible.

## Temporal model

Each measurement binds an instrument, channel, measurement timestamp and calibration reference. Each calibration declares:

- opaque calibration, instrument and channel identities;
- a half-open validity interval, valid_from ≤ measurement_time < valid_until;
- revocation and supersession events, each with an effective timestamp and a recorded timestamp.

The two event timestamps answer different questions:

- **Effective time:** from when does the event invalidate measurements?
- **Recorded time:** from when could the evidence system know about the event?

For a measurement made at time 150, suppose a revocation is effective at 140 but was recorded at 220. A query using knowledge time 219 reports the calibration as within its declared validity interval; a query using knowledge time 220 reports the measurement as revoked. The raw measurement record remains unchanged in both cases. The second query captures what is now known about that earlier measurement.

Revocation takes precedence over supersession when both known events apply. The kernel returns explicit statuses for not-yet-valid, expired, revoked, superseded, missing, unresolved, ambiguous, instrument-mismatched and channel-mismatched calibration references. Structural defects such as an empty identity or an invalid validity interval return errors.

## Important assurance boundary

**WithinDeclaredValidity is only a temporal result.** It does not mean that:

- the calibration issuer is authentic or authorized;
- the calibration certificate has a valid signature;
- the instrument was physically calibrated correctly;
- the calibration covers the measurement's wavelength, temperature, geometry, range or operating condition;
- the measurement is accurate or uncertainty-qualified;
- two instruments are independent.

External authority verification, certificate revocation, custody and issuer trust belong to Mycelix or another independently governed verifier. Symtropy should consume the verifier's exact receipt and status; it must not manufacture trust from a non-empty ID or hash.

The prototype uses UTC Unix seconds for timestamps. A laboratory-grade contract still needs a typed high-resolution time representation, clock identity, synchronization state and timestamp uncertainty. It also needs calibration scope/range applicability and explicit shared-cause lineage before it can qualify a measurement for physical decisions.

## Relationship to dynamic optical fouling

Calibration status and optical contamination state must remain separate evidence dimensions:

- A QCM can report deposition on its crystal, not automatically on an optic.
- An optical witness can report spectral reflectance/transmission change at its own surface.
- A reference channel can help separate source variation from optical-path degradation only when its ancestry and shared failure causes are explicit.
- A measurement can have temporally valid calibration and still be biased by a fouled optic; conversely, a clean optic does not repair an expired calibration.

The Symthaea planner should receive the resulting qualification and uncertainty information as evidence, not as a self-authorized fact. A reference check or maintenance action may be valuable, but this module does not command either.

## Authored test coverage

The 11 tests cover validity-window boundaries, not-yet-valid and expired states, retrospective revocation with separate knowledge time, future-effective revocation, supersession, instrument/channel mismatch, missing/unresolved/ambiguous references, invalid intervals, and event-order invariance.

### Reproduce locally

From the repository root:

```sh
rustc --edition=2021 --test research/asteroid-optical-isru-v0.2/calibration_time.rs -o /tmp/isru-calibration-tests
/tmp/isru-calibration-tests
```

**Validation honesty:** the 11 Rust tests are authored but have not been compiled or executed in the authoring environment because rustc/cargo are unavailable there. No CI pass, issuer verification, physical calibration claim, optical contamination measurement, or laboratory qualification is claimed.

## Next integration steps

1. Add these typed calibration and measurement references to the Symtropy v0.2 graph without editing the source-faithful v0.1 observations.
2. Preserve both event time and recorded time in the evidence store so later revocation does not erase what was known during the original campaign.
3. Add clock IDs/uncertainty, calibration applicability ranges, verifier receipt references, and instrument/channel common-cause groups.
4. Link QCM and optical-witness records to surface location, temperature, orientation, spectral band and exposure window; do not infer optical loss from QCM mass alone.
5. Add negative controls for shared calibration ancestry, reference-path contamination, stale revocation caches, and calibration range mismatch.
6. Feed only externally verified, temporally eligible measurements to likelihood estimation; retain rejected/unknown records for audit and failure analysis.

This is a narrow first implementation for issue #5612. It does not complete the metrology evidence graph or dynamically model deposition and optical degradation.
