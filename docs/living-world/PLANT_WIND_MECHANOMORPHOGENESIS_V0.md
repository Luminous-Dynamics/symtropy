# Plant Wind Mechanomorphogenesis V0

## Status

Normative design contract separating canonical mechanical wind effects from high-frequency presentation wind. It does not claim a biomechanical solver or wind renderer exists yet.

## Problem

Vegetation realism needs both:

- visually rich high-frequency motion;
- history-bearing mechanical consequences.

If one shader owns both, renderer/FPS/LOD become biological authority. If canonical simulation tries to resolve every leaf flutter, world scale becomes unnecessarily expensive.

## Core split

```text
world wind / shelter / canopy state
        |
        +-> canonical low-frequency mechanical exposure
        |       -> stress/load history
        |       -> breakage / acclimation / growth response
        |
        `-> derived presentation wind
                -> branch sway
                -> phase lag
                -> leaf flutter
                -> high-frequency turbulence
```

Presentation wind cannot write canonical plant structure or developmental history.

## Canonical mechanical exposure

The authoritative layer needs only the information required by enabled mechanical/developmental processes.

Possible drivers include:

- mean directional load;
- gust severity/count;
- bending-moment proxy;
- exposure duration;
- prevailing direction;
- shelter factor;
- root anchorage/stability demand.

Exact driver semantics require versioned units/normalization and integration rules under `DEVELOPMENTAL_STIMULUS_PROVENANCE_V0`.

## Presentation wind

Presentation may add detail such as:

- coherent trunk sway;
- branch-depth phase lag;
- secondary branch turbulence;
- leaf flutter;
- small stochastic-looking motion;
- camera-distance simplification.

This motion is disposable/rebuildable unless a qualified contact/breakage adapter explicitly promotes a mechanical consequence to canonical simulation.

## Structural coherence

Derived wind motion should respect the plant structural graph.

Useful presentation metadata can be generated per structural element/cluster:

- branch depth/order;
- attachment coordinate;
- rest axis;
- stiffness proxy;
- mass/foliage-loading proxy;
- parent phase;
- local flutter coefficient.

This allows motion to propagate coherently from trunk to twig to leaf instead of applying unrelated noise to vertices.

## Mechanomorphogenesis

Long-term mechanical exposure may influence future growth.

Examples include authored tendencies such as:

- increased radial/structural allocation under persistent load;
- altered branch orientation/growth direction;
- asymmetric crown development under prevailing wind;
- reduced extension under repeated mechanical stress;
- stronger root/stability allocation.

These are developmental responses, not instantaneous mesh deformations.

A qualified mechanical exposure accumulator feeds species developmental bindings/reaction norms; resulting growth actions remain resource-constrained under `PLANT_GROWTH_SETTLEMENT_V0`.

## Breakage

Canonical breakage needs a coarse deterministic structural criterion sufficient for gameplay/ecology. It does not require FEM in V0.

Conceptually:

```text
canonical load estimate
vs
current structural resistance
+ damage / decay / hydration modifiers
-> intact or breakage proposal
```

A breakage commit changes the plant structural graph and may create detached/dead material stock through later settlement rules.

A render shader crossing a visual bend threshold cannot break a canonical branch by itself.

## Fidelity

Mechanical fidelity may scale by process need:

- distant stand: aggregate wind-damage risk/exposure statistics;
- individual plant: coarse segment loads / developmental exposure;
- interactive/hero plant: finer local branch/contact mechanics.

Promotion/demotion must preserve the information required by future structural consequences.

High-frequency visual sway can scale aggressively with distance because it owns no biological history.

## Frame-rate invariance

Canonical exposure and breakage derive from authoritative simulation ticks/cadences, never rendered frame `dt`.

A tree observed at 30 FPS and 240 FPS under the same canonical wind history must develop identically.

The screen-space motion need not be bit-identical; the biological state must be.

## Terrain / shelter coupling

Future wind exposure may depend on:

- terrain orientation;
- nearby canopy;
- buildings/infrastructure;
- forest edge position;
- local damage/openings.

Those couplings should enter through deterministic world/habitat adapters, not direct renderer occlusion queries.

## First benchmark

Use genetically/developmentally identical young trees under:

```text
A: sheltered control
B: persistent crosswind
```

After the same simulated growth interval:

- canonical structural graphs should differ according to the authored mechanical-growth rule;
- replay should reproduce the difference exactly;
- removing wind later should not erase already-grown R2/R3 structure;
- rendering both at different FPS/LOD should not alter the final canonical graphs.

## Qualification fixtures

1. same canonical wind history -> same mechanical exposure state;
2. renderer/FPS changes -> identical canonical exposure/development;
3. high-frequency presentation turbulence cannot mutate graph;
4. persistent wind changes only future growth according to qualified binding;
5. existing mature structure is not rewritten when current wind disappears;
6. breakage proposal is deterministic from canonical load/resistance state;
7. failed breakage/growth settlement leaves graph unchanged;
8. offscreen coarse wind exposure matches fine execution for explicitly qualified sufficient statistics;
9. structural motion remains parent/branch coherent in presentation validation;
10. wind-history provenance can explain characteristic structural asymmetry.

## Non-goals

V0 does not define CFD, FEM, exact drag coefficients, leaf aeroelasticity, atmospheric simulation, or shader implementation.

It freezes the rule that **wind may look extremely rich in the renderer while only qualified low-frequency mechanical information is allowed to shape the plant's persistent biological history**.
