# Thermal HDC Semantics Research Protocol

## Purpose

This protocol governs thermodynamic semantic encoding in `symtropy-hdc-physics`.
The exact numerical simulation remains authoritative. The HDC layer derives a
versioned associative representation from exact thermal state so retrieval,
classification, novelty detection, and later continuous-time prediction can
reason about thermodynamic regimes without creating a second physics model.

This protocol extends `HDC_PHYSICS_RESEARCH_PROTOCOL.md`; all provenance,
dataset-split, baseline, negative-result, and claim-boundary requirements there
still apply.

## Versioning and provenance

The original `ExactStateDigest` algorithm version 1 predates body thermodynamic
state. It covers the established mechanical/contact/solver/shape state but does
not distinguish two otherwise-identical worlds whose only difference is
`ThermalBody`.

Thermal-aware experiments MUST use `exact_world_digest_v2` and
`ThermalSemanticEncoder`.

Digest v2 is deliberately compositional:

1. compute the established exact digest v1;
2. bind v1 to every body's ordered thermal-state presence marker;
3. when present, hash the exact bit patterns of:
   - temperature in kelvin;
   - effective thermal mass;
   - specific heat capacity;
   - thermal conductivity;
   - emissivity.

This preserves reproducibility of historical v1 work while preventing a
thermal-aware run from treating hot and cold worlds as the same exact source
state.

The thermodynamic semantic overlay has its own fingerprint. A composite frame
fingerprint binds both the base physics-encoder fingerprint and overlay
fingerprint. Changing a semantic range, namespace, role, quantizer, or feature
set requires a new overlay fingerprint/schema contract.

## Encoded authoritative features

The current overlay encodes only quantities already owned by
`symtropy-physics`:

- optional thermal-state presence;
- temperature;
- thermal mass;
- specific heat capacity;
- thermal conductivity;
- emissivity;
- world modeled sensible thermal energy;
- world modeled total energy.

The overlay inherits the base encoder's `IdentityPolicy`. It MUST NOT silently
reintroduce transient body handles when structural/identity-invariant retrieval
was requested.

No HDC value is authoritative physical state.

## Claim boundary

Permitted claims, when supported by held-out evidence, include:

- thermal-regime retrieval accuracy;
- similarity ranking for thermodynamic scenes;
- hot/cold or material-property classification;
- anomaly/novelty detection;
- prediction of future regime labels or solver error;
- compute-allocation decisions evaluated in shadow mode.

The overlay by itself does NOT justify claims of:

- improved heat-transfer accuracy;
- improved conservation;
- improved phase-change accuracy;
- faster simulation;
- safer fidelity reduction.

Those require separate intervention experiments against exact numerical
references and the adaptive-fidelity protocol.

## Required controls

At minimum, thermal semantic studies should compare:

1. HDC thermal overlay;
2. base HDC frame without the thermal overlay;
3. standardized raw thermal feature nearest-neighbor;
4. summary-statistic nearest-neighbor;
5. random projection at the same output dimension;
6. overlay without material properties;
7. overlay without world energy features;
8. identity-aware versus identity-free encoding when persistent identity is not
   part of the task.

## Generalization splits

Do not split adjacent frames from the same thermal trajectory across train and
test sets. Strong tests should reserve at least one of:

- unseen initial temperatures;
- unseen thermal masses;
- unseen heat capacities;
- unseen conductivities;
- unseen emissivities;
- unseen geometry or body count;
- unseen heating/cooling schedules;
- unseen contact/conduction topology;
- unseen random seeds.

Later phase-change work must additionally reserve unseen phase-transition
trajectories rather than only interpolating within one melt/freeze campaign.

## First validation campaigns

### TH-HDC-001 — provenance separation

Construct mechanically identical worlds with different temperatures and verify:

- v1 exact digests may remain equal by design;
- v2 exact digests differ;
- thermal composite semantic vectors differ;
- encoding does not mutate authoritative state.

### TH-HDC-002 — temperature retrieval

Generate complete episodes across held-out temperature bands with identical
mechanics. Compare thermal HDC retrieval against the required baselines.

### TH-HDC-003 — material retrieval

Hold geometry and temperature trajectories constant while varying heat
capacity/conductivity/emissivity. Measure whether the overlay retrieves the
correct material-regime family without overfitting transient body identity.

### TH-HDC-004 — conduction precursor retrieval

Use validated two-body conduction scenarios and ask whether early temporal
windows retrieve later trajectories with similar equilibration behavior.
Prediction claims must be evaluated by horizon and against non-temporal
baselines.

## Adaptive-fidelity handoff

Thermal HDC similarity or novelty may promote fidelity or request exact fallback
through the Physics Epistemic Firewall.

It MUST NOT reduce fidelity by itself. Any reduction requires a calibrated
predicted physical error satisfying `ADAPTIVE_FIDELITY_RESEARCH_PROTOCOL.md`.

The recommended first runtime experiment is therefore shadow-only:

1. run the highest-certified thermal/physics model unchanged;
2. encode semantic state and novelty;
3. generate hypothetical fidelity recommendations;
4. record exact physical error that would have resulted under cheaper models;
5. calibrate a separate continuous-time/error predictor;
6. only then test one-tier reductions in paired held-out intervention runs.

## Future semantic extensions

Add features only after the corresponding authoritative physics exists and is
validated. Candidate extensions include:

- entropy production;
- causal energy-transfer kind and magnitude;
- latent energy and phase fraction;
- thermal gradients in spatial material fields;
- thermo-mechanical stress/damage coupling;
- fracture-surface energy;
- porous-flow and saturation state;
- chemical/radiative reservoirs.

The semantic layer follows physical capability; it does not invent it.
