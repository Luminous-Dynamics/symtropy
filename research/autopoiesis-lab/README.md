# Symtropy Autopoiesis Lab

A standalone, standard-library-only research model for testing a narrow artificial-life question:

Can a resource-bounded network of processes regenerate components and boundary conditions required to sustain that same network after controlled component loss?

This is an abstract catalytic-network model, not biological simulation and not evidence that a system is alive or conscious.

## Model

- Three coupled components: Catalyst A, Catalyst B and Boundary Membrane.
- Catalyst A helps regenerate Catalyst B; Catalyst B helps regenerate Catalyst A.
- Either catalyst helps regenerate the membrane.
- The membrane and at least one catalyst jointly enable uptake from a finite environmental resource store.
- Resource moves between environment, internal stock, components and waste.
- Component degradation is explicitly transferred to waste.
- A knockout perturbation transfers the removed component to waste; the simulation itself does not directly reset or restore it.
- All state changes are deterministic and the total abstract resource inventory is checked after every step.

The resource unit is abstract. It is not a joule, chemical free energy or measured biological material/energy. The model has no calibrated kinetics, molecular mechanism, temperature, entropy, spatial diffusion or stochastic chemistry.

## Run

cargo test --manifest-path research/autopoiesis-lab/Cargo.toml
cargo run --manifest-path research/autopoiesis-lab/Cargo.toml --bin autopoiesis-lab

## First experiments

- Knock out A while B and membrane remain.
- Knock out B while A and membrane remain.
- Remove the membrane while both catalysts and internal stock remain.
- Knock out both catalysts: the model should fail to self-repair.
- Remove environmental and internal resource: the model should not maintain itself.
- Compare recovery against a scripted-reset null model in a future campaign.

The model's successful single-component recovery is a designed consequence of its declared interaction graph. It is a toy test of operational causal closure, not a discovery of new life chemistry. Multi-component failure is an important negative control.

## Claim ceiling

A passing test demonstrates only that the implemented abstract network recovers under its declared equations and finite resource budget. It does not establish biological life, genuine autopoiesis in nature, consciousness, open-ended evolution, physical energy conservation, or an autonomous real-world organism.
