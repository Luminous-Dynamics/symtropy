# World Continuation Core v0.1 — Handoff

## Review scope

This tranche implements only dependency-light continuation identity primitives in `symtropy-sim-contracts` plus their golden vectors and standalone qualification helper.

It deliberately does **not** implement snapshot storage, Universal Matter adapters, domain continuation digests, network handoff, inactive-world evolution, or world restore orchestration.

## Review priorities

1. canonical binary encoding is serializer-independent;
2. `ResumeIdentityClass` prevents a physical digest from being silently promoted to a stronger continuation claim;
3. same semantic domain/child sets hash identically regardless of arrival order;
4. malformed deserialized identities fail during validation/digesting;
5. v0.1 domain checkpoints are exact-time with the enclosing manifest;
6. lifecycle parent rules fail closed;
7. distributed-authority context is optional but identity-significant when present;
8. fixed-step time mapping uses checked integer arithmetic only;
9. golden vectors are stable and independently derived;
10. no code in this PR owns terrain/water/ecology/domain mutable truth.

## Qualification

Preferred Tier A command:

```bash
bash scripts/qualify-world-continuation-core-v0.1.sh
```

This runs the core crate as a temporary standalone package so the still-pending root workspace lock reconciliation cannot be mistaken for a core-contract failure.

A Tier A PASS is not Q2.

Q2 still requires the exact Universal Matter replay lineage, continuation-sensitive domain repairs, world suspend/restore implementation, and the evidence capsule described by #84.

## Known follow-ons

- #82 canonical causal event identity;
- #83 world manifest assembler/restore orchestration;
- #84 Q2 evidence capsule;
- #85 inactive-world evolution;
- #86 residency lifecycle;
- #87 snapshot artifact semantics;
- #88 broader golden-vector matrix;
- #89 same-world/fork policy;
- #90 child scope/ownership conflicts;
- #94 distributed authority handoff;
- #95 timebase bridge adoption;
- #72 native CUF adapters after Q2.
