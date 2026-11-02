# symtropy-neural-adapter-core

Dependency-free admission contract for embodied neural sensorimotor adapters.

## Scope

This crate validates:
- model and mapping identity syntax;
- evidence-tier source requirements;
- declared receptor/body/homeostatic channel schemas;
- observation source, unit, range and freshness;
- body-snapshot binding for proprioceptive/contact samples;
- bounded muscle or motor-unit activation proposals;
- compatibility between model effectors and a supplied body-effector set.

It intentionally does not:
- run neural dynamics or a biological connectome;
- fetch or cryptographically authenticate datasets/digests;
- query world truth or renderer state;
- mutate physics, body transforms, contact state or animation;
- replace ANIMA, Basin perception, physiology or action arbitration.

SHA-256 strings are references whose syntax is checked. They must be generated and verified by the upstream evidence/provenance system.

## Evidence boundary

The included unit tests use synthetic fixtures only. Passing them qualifies this adapter contract, not animal biology, causal behaviour, a connectome, or gameplay fidelity.

Production perception integration remains subject to Symtropy's existing exact-head fauna-perception prerequisite.
