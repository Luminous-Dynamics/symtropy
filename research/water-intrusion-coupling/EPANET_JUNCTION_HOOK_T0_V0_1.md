# EPANET Junction Hook T0 — Synthetic Snapshot Contract v0.1

**Programme:** WATER-MIXING-ENGINE-INTEROP-001 / [#5290](https://github.com/Luminous-Dynamics/symtropy/issues/5290)  
**Parent mixing qualification:** [#5216](https://github.com/Luminous-Dynamics/symtropy/issues/5216)  
**Transient operator:** [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274), draft [#5288](https://github.com/Luminous-Dynamics/symtropy/pull/5288)  
**Evidence ceiling:** synthetic hook-snapshot arithmetic; no EPANET engine extension or physical qualification

## 1. Why this T0 exists

The public OWA-EPANET Toolkit exposes a per-quality-step loop and computed node/link values, including read-only node quality, link flow, link quality, demand, source mass and leakage. It does not expose the actual incoming leading-segment records that the internal junction update consumes. A public Toolkit sidecar therefore cannot manufacture a faithful consumed-segment mass ledger from link averages or node quality.

The proposed engine-fork hook must emit those records from inside the junction quality update. This T0 validator defines and checks the shape and arithmetic of that proposed snapshot. It deliberately does not claim that the public Toolkit can produce it or that an EPANET fork already exists.

## 2. One quality-step snapshot

A snapshot binds:

- engine family and exact source revision;
- junction ID and geometry digest;
- local operator digest and snapshot digest;
- quality time and quality time step;
- time until the next hydraulic event;
- actual consumed incoming segment portions;
- supported external source mass/volume;
- every outgoing pipe and terminal withdrawal;
- local inventory volume/mass change;
- mass and volume uncertainty budgets.

Each incoming segment contribution contains link ID, segment ID, consumed volume, available segment volume, concentration and current direction into the node. Mass is computed only from the consumed portion:

\[
M_{in,segments}=\sum_s V_{s,consumed} C_s
\]

It is incorrect to multiply the entire leading segment volume by its concentration if only part of that segment is consumed in the current quality step.

## 3. External source semantics

The initial synthetic validator supports two explicitly integrated source forms:

- CONCEN: source volume for the current quality step and concentration; source mass is their product.
- MASS: already-integrated source mass for the current quality step, with no added volume.

FLOWPACED and SETPOINT are rejected in v0.1. These source modes modify concentration according to engine-specific post-mixing rules and must not be collapsed into a generic source-mass term without proving exact semantics. A fork must either preserve the original source logic in the same order as the base engine or refuse local-operator mode at that node.

The snapshot field for MASS is mass integrated over this quality step—not the configured source mass rate per minute. The adapter must perform the correct time integration.

## 4. Outgoing ports are more than pipes

A junction control-volume balance must include all outgoing channels:
- pipe parcels created on each outgoing link;
- delivered consumer demand;
- emitters;
- leakage or other explicit terminal withdrawals.

Each port has an identity, type, same-step water volume and operator-assigned mass. The validator derives concentration as mass / volume for positive volume. A zero-volume outlet cannot receive nonzero mass.

A missing or duplicate port ID is a contract failure. If the outgoing set is incomplete, the validator preserves known incoming/observed port information but returns a partial-boundary disposition with no closure residual.

## 5. Separate volume and tracer mass closure

For each quality step:

\[
R_V=V_{in}+V_{source}-V_{out}-\Delta V_{inventory}
\]

\[
R_M=M_{in}+M_{source}-M_{out}-\Delta M_{inventory}
\]

The signed residuals are kept independently. A volume-balance pass does not imply tracer mass closure, and vice versa. The validator does not rescale outlet concentrations to hide either residual.

The initial T0 supports only explicit source inputs, complete outlet port sets, known inventory deltas, positive uncertainty declarations and a k-sigma synthetic closure rule. It does not infer adsorption, storage, leakage, reaction, or instrument loss from a residual.

## 6. Baseline parity

A baseline_complete_mix mode computes the complete-mix concentration from incoming segment mass and explicit supported source mass divided by total entering volume (including CONCEN source volume):

\[
C_{mix}=\frac{M_{in}+M_{source}}{V_{in}+V_{source}}
\]

It then verifies every outgoing pipe/terminal port mass against C_mix times that port's same-step volume. A zero-delay complete-mix operator must pass this parity test before any outlet-resolved mode is considered.

An outlet_resolved mode accepts the local operator's per-port masses and validates the full volume/mass ledger. It does not claim that those outputs came from the EPANET engine; this is a synthetic snapshot contract test.

## 7. Time and state constraints

A snapshot quality step must be positive and cannot exceed the time remaining to the next hydraulic event. Engine revision, geometry, operator and snapshot identities must be digest-bound.

If flow reversal is present but not all port directions are observed, the validator abstains. If a port's current flow reverses, it must be reclassified by the engine's signed flow and its leading segment identity; an old inlet/outlet orientation cannot be reused.

The T0 checks the snapshot's source-revision and digest formats, but a later engine integration must additionally bind its fork build features, compile flags, unit convention, quality time step, reaction settings and source configuration.

## 8. Frozen cases

The v0.1 corpus contains ten known-answer cases:

1. Baseline complete mixing and zero-delay concentration parity.
2. Multiple leading segments where only consumed portions contribute.
3. Explicit MASS booster integrated exactly once.
4. CONCEN external inflow contributes both volume and mass.
5. FLOWPACED source refusal.
6. Duplicate outlet identity refusal.
7. Partial outlet set with no closure claim.
8. Signed open tracer-mass residual.
9. Open water-volume residual separated from mass residual.
10. Quality time step crossing a hydraulic event refusal.

Unit tests also mutate an outlet mass without updating the snapshot digest and require contract rejection, and attempt to assign mass to a zero-flow port.

## 9. What has and has not been demonstrated

- Authored: Rust hook-snapshot validator, test corpus, independent oracle, manifest and exact-revision workflow.
- Independently mirrored: all ten synthetic snapshot dispositions and their mass/volume calculations in a separate JavaScript implementation.
- Not demonstrated: retrieval of actual segment-consumption data via public Toolkit API, EPANET fork, engine callback integration, bitwise baseline parity, segment creation, flow reversal interaction, reactions, or physical tracer response.
- Native Rust fmt/test/Clippy and exact-head CI must pass before even the synthetic implementation is considered verified.

The public API limitation is intentional: this synthetic validator tests the future fork-hook contract, not a capability the public Toolkit currently exposes.
