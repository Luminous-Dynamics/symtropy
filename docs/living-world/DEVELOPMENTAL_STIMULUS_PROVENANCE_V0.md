# Developmental Stimulus Provenance V0

## Status

Normative Living World developmental-authority contract. This document constrains future consumers of PR #225's deterministic reaction norms. It does not claim PR #225 already defines species assets, environmental exposure history, or organism state.

## Problem

A reaction norm maps an authored stimulus coordinate to an authored response coordinate.

The arithmetic can be perfectly deterministic while the biology is still meaningless if the stimulus coordinate is not semantically bound.

The integer value `500` could otherwise be reinterpreted as:

- normalized shade;
- cumulative drought dose;
- degree-hours above a thermal threshold;
- wind exposure;
- nutrient deficit;
- infection burden;
- milliseconds since injury;
- a renderer-derived convenience value.

Even within one driver, changing units or normalization can change every organism's developmental trajectory without changing any reaction-norm knots.

## Core rule

A future-bearing developmental response must bind its reaction norm to an explicit **developmental stimulus specification**.

Conceptually:

```text
DevelopmentalStimulusSpec {
    driver_kind,
    source_authority_kind,
    source_schema_version,
    unit_or_normalization,
    integration_semantics,
    cadence_or_window,
    calibration_version,
}
```

The exact Rust representation is non-normative. These semantic dimensions are normative.

## Driver kind

The driver identifies the biological/environmental meaning of the coordinate.

Examples:

- `ShadeExposure`
- `DroughtExposure`
- `MechanicalWindLoad`
- `ThermalDevelopment`
- `NutrientStress`
- `BrowsingDamage`
- `InfectionBurden`
- `CompetitionPressure`

These are examples, not a frozen enum.

A driver kind is not interchangeable merely because another driver uses the same integer range.

## Source authority

The stimulus must identify what canonical/derived representation supplied it.

Examples:

- authoritative habitat sample;
- qualified physical field;
- qualified ecological index;
- accumulated organism physiology;
- disturbance/biography state.

Presentation-only renderer values cannot become developmental authority.

Derived F-I ecological indices may drive development only when their derivation and update semantics are explicitly qualified for that biological process. Determinism alone is not sufficient.

## Units / normalization

The stimulus coordinate requires an explicit scale.

Examples:

```text
0..1000 normalized shade exposure
millikelvin-hours above threshold
microgram-equivalent toxin dose
fixed-point water-deficit integral
quantized mechanical load exposure
```

A unit/normalization change is a schema/calibration change. Existing reaction norms must not silently reinterpret their knot coordinates.

## Integration semantics

Development often depends on history rather than an instantaneous sample.

At minimum distinguish semantics such as:

- instantaneous sample;
- cumulative dose/integral;
- time-weighted mean;
- threshold exceedance / degree-time;
- bounded moving window;
- exponentially decayed memory;
- peak exposure;
- event count/severity accumulation.

Two integration modes using the same units are different developmental drivers.

## Cadence / window

The authoritative simulation cadence or integration window is part of the stimulus semantics.

A coarse/off-screen representation may not replace 100 fine-grained environmental samples with one endpoint sample unless the developmental process explicitly proves that this is a sufficient statistic.

Preferred pattern:

```text
canonical environment/history
    -> qualified driver accumulator / sufficient statistic
    -> developmental stimulus coordinate
    -> reaction norm
    -> developmental response
```

## Fidelity invariance

Changing render distance, frame rate, ECS materialization, or Level-P projection must not change developmental stimulus history.

Active/coarse fidelity changes may change the implementation used to accumulate a driver only if the replacement preserves the process-required information within its declared qualification tolerance/exactness semantics.

If it cannot, collapse is C1/C2/C3 under the existing collapse contract rather than silent history loss.

## Calibration version

Changing any of the following requires explicit versioning/migration or a new developmental lineage root:

- sensor/field adapter;
- normalization constants;
- threshold values;
- unit scale;
- integration method;
- cadence interpretation;
- source schema;
- reaction-norm response scheme when semantics change.

The same reaction-norm knots under a different calibration are not automatically the same biological model.

## Multi-driver traits

Some morphology depends on several drivers, for example:

```text
branch architecture = f(light, wind, water, injury, age)
```

V0 does not require a generic multivariate surface.

Preferred early composition is explicit and reviewable: independent qualified drivers feed named response contributions under a versioned species/development rule.

Avoid an untyped vector of anonymous normalized inputs.

## Causal legibility

The developmental pipeline should preserve enough provenance to answer:

> Why is this trait different?

A useful future explanation chain is:

```text
branch.internode_length
  <- shade response +143 q
  <- ShadeExposureV1 = 682 q
  <- canopy/light history over ticks 18,000..24,000
  <- qualified habitat/light adapter
```

This is central to the Living World goal that organism appearance exposes biography rather than arbitrary variation.

## Qualification fixtures

Minimum future evidence should include:

1. same hereditary seed + same stimulus history -> same developmental response;
2. same hereditary seed + changed qualified exposure -> expected characteristic response change;
3. changing renderer/FPS does not change stimulus history;
4. changing stimulus unit/schema without migration fails closed;
5. instantaneous and cumulative drivers cannot be interchanged under the same identity;
6. save/reload preserves accumulated stimulus state and future response;
7. coarse/fine execution produces equivalent driver state where the coarse closure is declared sufficient;
8. an insufficient coarse representation blocks collapse rather than discarding required developmental history;
9. presentation-only values cannot be admitted as canonical driver authority;
10. explanation/provenance identifies driver kind, calibration and source interval for a changed trait.

## Relationship to PR #225

PR #225 should remain a pure deterministic response-math layer.

It intentionally does not know whether `stimulus_q = 500` means shade, wind, drought, or thermal time.

The future species/development layer binds that mathematical response to this semantic stimulus provenance.

## Non-goals

This contract does not define:

- the complete species asset schema;
- multivariate neural developmental models;
- organism biography persistence bytes;
- real-world biological calibration values;
- Bevy rendering parameters;
- CRK event identity.

It freezes the rule that **developmental plasticity becomes causal biology only when every reaction-norm stimulus is bound to a versioned source, unit/normalization, integration history, cadence and calibration—not merely to an integer value**.
