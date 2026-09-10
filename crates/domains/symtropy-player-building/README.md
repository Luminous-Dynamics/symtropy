# symtropy-player-building

PB-01 is the dependency-light proposal layer for player-authored construction.

It exists to make freeform building deterministic, portable, replayable, and safe to network **without** becoming a second physical authority.

## Owns

- immutable content-bound `ConstructionIntent` revisions;
- exact authoring-frame binding;
- integer-micrometre / turn-phase authoring poses;
- bounded built-in geometry plus exact external geometry references;
- non-authoritative material requests;
- exact planner/context binding;
- deterministic acyclic proposal operation graphs;
- machine-readable advisory vs hard-failure reports;
- read-only `BuildProjection` values that must be regenerated from exact intent + plan.

## Does not own

- conserved matter or inventory;
- reservations or physical execution;
- structural safety/truth;
- fabrication/process truth;
- terrain/world truth;
- technical commissioning;
- ownership/title;
- civic permission;
- room/place/home identity;
- renderer state.

The intended authority crossing is a later narrow adapter:

```text
PB-01 ConstructionIntent / ConstructionPlan
    -> PB-02 exact adapter
    -> qualified symtropy-construction / fabrication / matter authority
```

An unknown `AdapterProposal` profile is inert proposal data. There is intentionally no generic execute-payload API.

## Scale rule

Large structures are composed from bounded intents. A city, shipyard, or orbital habitat should not become one enormous atomic proposal merely because PB-01 can represent many elements. Hierarchical planning is the scaling mechanism.

## Evidence status

The source and tests on this branch are implementation candidates until an exact-head qualification run executes successfully. Parent PB-00 is a separate documentation contract and makes no runtime PASS claim.
