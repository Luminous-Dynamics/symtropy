# EXO-12 — single-spend resource settlement

Resource arbitration alone is not sufficient authority. A numeric grant could be accidentally applied more than once, applied to the wrong subsystem, or replayed after a transport retry.

EXO-12 adds a settlement layer after arbitration:

`caller-owned budget -> arbitration receipt -> epoch-bound settlement -> consumption result -> subsystem mutation`

A `ResourceEpoch` binds settlement to a named budget source, authoritative simulation tick, and source-local generation. Each consumption has a stable identity and may spend only the remaining grant for its declared subsystem.

An identical retry returns the original receipt with `ConsumptionDisposition::Replay`; a new spend returns `New`. Integrations execute downstream state changes only for `New`. Reusing a consumption identity with different semantics fails closed, and a second identity cannot overdraw an exhausted grant.

`ResourceSettlement` is serializable so an owning persistence layer can retain consumption history across restart rather than reopening already-spent grants.

## Boundary

Settlement does not own the upstream battery or generator and does not mutate a shield, motor, cooling system, sensor, communications system, or life-support system directly. It only establishes whether a particular share of an already-authorized allocation is newly spendable.

This is fictional/game resource authority, not a recommendation for real exoskeleton power electronics.
