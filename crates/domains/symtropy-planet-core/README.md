# symtropy-planet-core

Dependency-light PLANET-01 primitives for explicit planetary forcing and assumption identity.

This crate does **not** determine habitability or create life. It records the exact planetary/model assumptions that downstream physical, climate, chemical, ecological, evolutionary, and observational systems may consume.

## V0 rules

- no implicit Earth defaults;
- non-finite or impossible bulk/orbital values fail closed;
- model identities include version and evidence class;
- optional atmosphere/ocean/interior models remain explicitly absent when unknown;
- authority identity uses a domain-separated canonical byte grammar rather than Serde representation;
- weaker materially causal model evidence propagates to the aggregate forcing profile;
- `symtropy-world` remains the owner of world/body-cell identity and spatial mapping;
- climate, geology, hydrology, ecology, evolution, biosignatures, and civilization remain owned by their respective domains.

See `docs/design/ASTROBIOLOGY_AUTHORITY_AND_UNCERTAINTY_V0.md` in the parent ASTRO-00 stack for the governing epistemic contract.
