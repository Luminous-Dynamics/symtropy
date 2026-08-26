# Thermal HDC Semantics Research Protocol

## Purpose

This protocol governs thermodynamic semantic encoding in `symtropy-hdc-physics`.
The exact numerical simulation remains authoritative. The HDC layer derives a
versioned associative representation from validated thermal state so retrieval,
classification, novelty detection, and later continuous-time prediction can
reason about thermodynamic regimes without creating a second physics model.

This protocol extends `HDC_PHYSICS_RESEARCH_PROTOCOL.md`; all provenance,
dataset-split, baseline, negative-result, and claim-boundary requirements there
still apply.

## Three distinct contracts

Thermal-aware research must keep three concepts separate:

1. **Digestable state** — an exact bit-pattern identity can be computed for the
   current world, including invalid state, so a failure can still be named and
   reproduced.
2. **Valid semantic source state** — the world is numerically representable,
   every attached thermal reservoir validates, and modeled thermal/total energy
   accounting is complete and finite. Only this state may be encoded as a
   normal thermal semantic frame.
3. **Lifecycle-proven state** — the transition from an earlier authoritative
   state to the current state has explicit body/reservoir creation, attachment,
   detachment, removal, replacement, or representation-transition provenance.

A digest proves only the first property. Successful semantic encoding proves the
second property for the sampled state. Neither one proves the third property.

This distinction is deliberate. A thermal reservoir may appear at exactly `0 J`
and therefore leave a global numeric energy total unchanged while still
representing an unresolved state-topology/lifecycle transition.

## Versioning and exact provenance

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

Digest v2 intentionally remains defined for invalid thermal state. This allows a
failed authoritative state to retain a deterministic provenance identity instead
of becoming unnameable. Consequently:

- digest equality is not a validity certificate;
- digest inequality is not proof that a lifecycle transition was legitimate;
- a sequence of digests is not a substitute for a lifecycle/causal ledger.

This preserves reproducibility of historical v1 work while preventing a
thermal-aware run from treating hot and cold worlds as the same exact source
state.

The thermodynamic semantic overlay has its own fingerprint. A composite frame
fingerprint binds both the base physics-encoder fingerprint and overlay
fingerprint. Changing a semantic range, namespace, role, quantizer, or feature
set requires a new overlay fingerprint/schema contract.

## Fail-closed semantic source validation

`ThermalSemanticEncoder::encode_world` must reject rather than sanitize an
invalid authoritative source. Before a thermal semantic vector is emitted, the
source world must satisfy all of the following:

- every attached `ThermalBody` passes its physical/representability validation;
- no body counted by the authoritative diagnostics contains non-finite state;
- modeled thermal-energy accounting is complete;
- modeled thermal energy is finite;
- modeled total energy is finite;
- the thermal-overlay HDC specification is compatible with the base encoder.

Non-finite source values MUST NOT be mapped to zero or another ordinary semantic
value. Unknown or invalid physics is not low temperature, zero energy, low
novelty, or any other benign feature.

If semantic encoding fails, the exact digest may still be recorded so the
failure can be reproduced and correlated with diagnostics. The failed frame must
not enter a normal training/retrieval corpus as if it were a valid observation.

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

World energy features are encoded only when the authoritative diagnostics report
complete modeled energy accounting. A partial sum that silently skipped an
invalid reservoir is not a valid semantic energy feature.

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
- safer fidelity reduction;
- validated body/reservoir lifecycle provenance.

Those require separate intervention experiments against exact numerical
references, the adaptive-fidelity protocol, and the Phase-Zero world/reservoir
lifecycle contracts.

## Required negative controls

Thermal semantic implementation tests must include at least:

1. mechanically identical worlds whose only difference is temperature: v1 may
   match, while v2 and thermal semantic vectors differ;
2. invalid but finite thermal material/state: digest v2 remains available but
   semantic encoding is rejected;
3. non-finite thermal source state: semantic encoding is rejected and the value
   is never coerced to zero;
4. identity-free encoding: the thermal overlay does not reintroduce transient
   handles;
5. non-mutating encoding: semantic observation does not change authoritative
   world state.

As lifecycle APIs mature, add a paired control where two endpoint states have
identical numeric energy but differ in reservoir presence. Endpoint semantic
state may distinguish the presence marker, but lifecycle legitimacy must still
come from explicit provenance rather than inferred similarity.

## Research baselines

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

Invalid-source examples should be kept in a separate diagnostic/adversarial set,
not mixed into the ordinary semantic corpus by silently replacing invalid values.

## First validation campaigns

### TH-HDC-001 — provenance separation

Construct mechanically identical worlds with different temperatures and verify:

- v1 exact digests may remain equal by design;
- v2 exact digests differ;
- thermal composite semantic vectors differ;
- encoding does not mutate authoritative state.

### TH-HDC-001B — source validity firewall

Construct finite-invalid and non-finite thermal states and verify:

- v2 remains deterministic and available for failure provenance;
- semantic encoding fails closed;
- invalid values are never mapped to normal thermal semantic bins;
- incomplete modeled thermal totals are never emitted as complete semantic
  energy features.

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
predicted physical error satisfying `ADAPTIVE_FIDELITY_RESEARCH_PROTOCOL.md` and
complete authoritative evidence including accounting completeness and lifecycle
stability.

The recommended first runtime experiment is therefore shadow-only:

1. run the highest-certified thermal/physics model unchanged;
2. validate the source before thermal semantic encoding;
3. encode semantic state and novelty;
4. preserve missing accounting/lifecycle/error evidence as missing;
5. generate hypothetical fidelity recommendations only when the controller input
   is complete;
6. record exact physical error that would have resulted under cheaper models;
7. calibrate a separate continuous-time/error predictor;
8. only then test one-tier reductions in paired held-out intervention runs.

A semantic encoding failure is itself a reason to avoid reduction; it is not a
reason to substitute a zero vector, zero novelty, or another default observation.

## Future semantic extensions

Add features only after the corresponding authoritative physics exists and is
validated. Candidate extensions include:

- entropy production;
- causal energy-transfer kind and magnitude;
- explicit reservoir lifecycle events;
- latent energy and phase fraction;
- thermal gradients in spatial material fields;
- thermo-mechanical stress/damage coupling;
- fracture-surface energy;
- porous-flow and saturation state;
- chemical/radiative reservoirs.

The semantic layer follows physical capability; it does not invent it.