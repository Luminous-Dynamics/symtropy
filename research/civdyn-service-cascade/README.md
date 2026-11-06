# CivDyn Service Cascade v0.1

A deterministic service-dependency reference kernel for interdependent infrastructure.

## Boundary

This crate models **service functionality** in [0, 1], based on exogenous component-availability traces and typed dependencies. It does not model or duplicate physical inventories, production, transfers or conservation. Those remain in the CivDyn resource ledger.

## Dependency model

Each node has:
- a unique ID and service type;
- an optional beneficiary demand weight (zero for internal assets to avoid double-counting downstream consumption);
- zero or more dependency groups.

Each dependency input has a parent ID, the parent service fraction needed for full function, and a minimum operating fraction. Parent support is continuous between the minimum-operating and full-service levels. Groups are either ALL-of or ANY-of; multiple groups on one node are ANDed.

The graph is topologically sorted independent of declaration order. Cycles, unknown parent IDs, malformed availability and invalid thresholds fail closed. V0 deliberately rejects cycles instead of silently choosing an arbitrary fixed-point update order.

## Metrics

- Node service trajectories by tick.
- Minimum service, service-deficit area and time below 50% / 90%.
- Sustained recovery after the declared shock window.
- Demand-weighted service-deficit area over beneficiary endpoints.
- Counterfactual component criticality: additional demand-weighted service deficit when a node is forced offline for a declared interval.

The demand-weighted service-deficit area is a task-level service-loss measure, not a universal resilience score. Node-specific and beneficiary-level vectors remain visible.

## Synthetic reference

The 9-node graph connects grid, fuel, water treatment, pumping, telecom, road access, hospital, payments and household water service. A two-tick grid outage causes downstream water, healthcare and payment service loss, while OR redundancy preserves road access using fuel.

The reference outputs are artificial software controls, not a model of Johannesburg or any named utility.

## Run

```sh
cargo fmt --manifest-path research/civdyn-service-cascade/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-service-cascade/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-service-cascade/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-service-cascade/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-service-cascade/Cargo.toml --bin service-cascade-sweep
```

## Claim ceiling and next gates

This v0.1 checks deterministic DAG propagation, redundant alternatives, partial degradation, temporal service loss and component-outage counterfactuals. It does not estimate real network dependencies, model physical load redistribution, infer probabilities, optimize restoration schedules or establish real-world criticality.

V1 requires source-pinned network evidence, uncertainty in dependency existence/strength, buffer/storage exhaustion, stochastic shock timing, resource-constrained repair scheduling and held-out event hindcasts. Where specialist power/water solvers are relevant, they should remain authoritative physical submodels; this crate is a systems-level service consequence layer.
