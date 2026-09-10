# FEP-04 — local perceived-world inputs v0.1

The launcher FEP agent keeps its established six-dimensional observation schema while removing three authoritative global-state shortcuts.

## Before

NPC cognition directly consumed:

- `LeviathanState.phase` as danger;
- `SettlementMetrics.water`;
- `SettlementMetrics.power`.

The danger gradient also used the Fusion Core position as `danger_source`, even though the Fusion Core is not the Leviathan.

## After

The six channels are:

1. agent energy self-state;
2. agent allostatic-load self-state;
3. **local danger/risk cue strength**;
4. agent caution self-state;
5. **local water estimate**;
6. **local power estimate**.

External channels are produced through `PerceivedWorldFrame` rather than by copying global aggregate resources.

### Local danger

Danger is currently the strongest distance-attenuated local `NoiseEmitter` cue within the configured game-space sensing range. This represents locally observable *risk cue strength* because noise is what drives the hidden Leviathan state machine.

A zero value means **no current local danger cue**. It does not mean that the NPC knows the Leviathan is dormant or that the world is safe.

There is no physical Leviathan entity/location in this slice. Therefore FEP-04 removes the previous Fusion-Core-as-danger-source vector and passes no invented threat position.

### Local water and power

Nearby water pumps and power junctions provide bounded local samples. Distance determines observation confidence. Hidden explanatory causes such as `WaterPump::is_sabotaged` are not projected into the FEP estimate.

If no new infrastructure sample is locally available, the NPC retains its previous estimate until its adapter-local staleness generation expires. Unknown/stale infrastructure falls back to an explicit neutral 0.5 prior rather than zero.

## Time authority

Perception memory uses an **adapter-local generation counter**. It is intentionally not called a world tick or simulation timestamp. It controls only AI forgetting/staleness and has no authority over world state.

## Fixed point

Perceived scalar values and confidence are stored internally as 0..10,000 basis points. Floating-point values are produced only at the existing FEP boundary.

## Query safety

The NPC query explicitly includes `With<CrewNpc>` while the secondary noise query uses `Without<CrewNpc>`. This makes their mutable/read `NoiseEmitter` access provably disjoint to Bevy rather than relying on the fact that `CrewNpc` appears in query data.

## Remaining epistemic debt

FEP-04 fixes the six-channel danger/water/power path and the crisis-water shortcut. It does **not** claim that the whole authored NPC behavior system is epistemically clean yet. Remaining direct reads include, among others:

- archetype-specific scans for damaged infrastructure;
- medic access to other NPC psychological state;
- global well/harmony inputs;
- some authored tutorial/task knowledge.

Those should move through capability-appropriate observation/communication/task surfaces in later narrow tranches.

## Qualification

Implemented/static only. The `fep-ai` feature path still requires exact-head format/check/test/Clippy and gameplay regression execution before PASS is claimed.
