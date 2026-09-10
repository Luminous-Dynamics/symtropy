# symtropy-player-building

PB-01 is the dependency-light proposal layer for player-authored construction.

It exists to make freeform building deterministic, portable, replayable, and suitable for transport behind an explicitly bounded ingress **without** becoming a second physical authority.

PB-01's current Serde representations are **network-ready semantics, not a complete hostile-network decoder**. Semantic collection bounds are revalidated after decode, but direct untrusted ingress still requires the bounded framing/decoding theorem tracked in #439.

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
- renderer state;
- an unbounded or unauthenticated network ingress boundary.

The intended authority crossing is a later narrow adapter:

```text
PB-01 ConstructionIntent / ConstructionPlan
    -> PB-02 exact adapter
    -> qualified symtropy-construction / fabrication / matter authority
```

An unknown `AdapterProposal` profile is inert proposal data. There is intentionally no generic execute-payload API.

## Scale rule

Large structures are composed from bounded intents. A city, shipyard, or orbital habitat should not become one enormous atomic proposal merely because PB-01 can represent many elements. Hierarchical planning is the scaling mechanism.

## Open semantic hardening

#439 tracks the remaining pre-stability questions discovered during static review:

- exact existing targets must be declared by intent before repair/removal/new-to-existing joins can be planned;
- intent/plan identity should move from JSON-derived bytes to an explicit canonical binary preimage;
- content-addressed ancestry must support legitimate concurrent fork merges;
- hostile network ingress needs bounded framing/decoding before allocation;
- rotation equivalence must be explicit before snapping/CAD equivalence is frozen.

## Evidence status

The source and tests on this branch are implementation candidates until an exact-head qualification run executes successfully. Parent PB-00 is a separate documentation contract and makes no runtime PASS claim. Even a green pre-hardening run would qualify only its exact executed head, not the stronger #439 theorems.
