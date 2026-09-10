# PB-01 review checklist

Review this tranche as a proposal/identity layer, not as a gameplay-completeness claim.

Before merge, confirm:

- no dependency on Bevy, renderer, physics, Fabrication, Construction, terrain, Symthaea, Mycelix, or networking;
- no API named or shaped like `execute`, `reserve`, `commit_physical`, `is_safe`, `is_built`, `owns`, or `is_home`;
- no floating-point field participates in exact proposal identity;
- exact frame/planner/context refs are revalidated after deserialize;
- vectors with set semantics are canonicalized and duplicate/conflicting identities fail closed;
- operation ordering is graph-derived rather than caller/render order;
- projection data cannot be deserialized as canonical state;
- hard/advisory findings stay faceted and do not create a scalar quality score;
- the dedicated lane executes the exact product head before any executable qualification statement;
- Cargo.lock reproducibility is handled separately before PB-02 crosses into physical authority.

The primary hostile review question is:

> Can any caller construct PB-01 bytes that cause another subsystem to believe physical work, conserved material allocation, safety, commissioning, ownership, permission, or home identity already exists?

For PB-01 the intended answer is **no**. If a downstream consumer treats these proposal bytes as any of those facts, that consumer is violating the boundary and must be repaired rather than strengthening the proposal's rhetoric.
