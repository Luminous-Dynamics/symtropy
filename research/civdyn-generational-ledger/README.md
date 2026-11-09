# CIVDYN Generational Incidence Ledger v0.1

A cohort-indexed accounting layer over the existing CIVDYN resource ledger. It does not create a second simulator or resource stock-flow engine.

## Scientific objective

Answer, with explicit evidence, which cohorts receive service benefits, which inherit closing stocks, which are assigned future obligations, and which quantities remain unassigned.

The first implementation is deliberately an accounting contract, not a complete model of intergenerational welfare. It must expose unknown incidence instead of inventing a population-proportional allocation.

## Four separate ledgers

1. **Service incidence:** demand, served and unmet amounts by cohort, region, resource and tick.
2. **Inherited stock:** closing water (m³), energy (kWh) and food (kg) assigned to a cohort or explicitly unassigned, including asset identity.
3. **Physical obligations:** a decision-linked quantity, named measure and unit, creation/due ticks, cohort assignment, uncertainty and source reference.
4. **Financial obligations:** amount, currency, price base year, kind, liable cohort and recipient cohort, stored independently from physical accounts.

A separate capital-record type can describe produced, human, natural and institutional capital with an explicit measure, unit, condition and uncertainty. It is not automatically added to resource stocks.

## Reconciliation contract

For each source-ledger region and each of water, energy and food, the assembler requires:

- sum of cohort-assigned plus explicitly unassigned demand = resource-ledger demand;
- sum of served amounts = resource-ledger served amount;
- sum of unmet amounts = resource-ledger unmet amount;
- sum of cohort-assigned plus explicitly unassigned closing stocks = resource-ledger closing stock.

The original resource ledger's independent conservation checks must also pass. A financial obligation cannot compensate for a physical conservation residual. No metric combines m³, kWh, kg and currency.

All records carry an action/asset identifier and a source-evidence reference. An incomplete allocation fails closed; it is not silently prorated.

## Metamorphic and negative controls

The library tests:

- cohort split invariance: splitting one identical cohort into two preserves aggregate physical totals;
- reassignment: moving an item between cohorts changes incidence but not physical totals;
- discount invariance: changing the normative discount rate changes only a derived discounted-service view, never source stocks, obligations or conservation;
- future inheritance: a not-yet-born cohort receives the previous step's closing stock rather than a reset baseline;
- unassigned incidence: shared/public assets remain visible as explicitly unassigned;
- separate units: all three resource components reconcile independently;
- fail-closed behavior for incomplete allocations, invalid obligations and duplicate record identities.

## Run

```sh
cargo fmt --manifest-path research/civdyn-generational-ledger/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-generational-ledger/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-generational-ledger/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-generational-ledger/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-generational-ledger/Cargo.toml --bin generational-incidence-sweep
```

The binary emits one reconciliation row per region/resource, one service/inheritance row per cohort/resource, and separate physical- and financial-obligation rows.

## Limitations and next steps

- The synthetic assignments are inputs, not inferred facts. Real incidence requires measured or source-backed allocation evidence.
- The resource ledger's aggregate service report cannot identify within-region cohort distribution by itself; an explicit cohort allocator or richer source evidence is required.
- Currency conversion, inflation, taxes and discounting require separate declared protocols; this version does not net financial values against physical quantities.
- Demographic dynamics, migration, survival, cohort size and overlapping-generations welfare are not simulated here.
- No social discount rate or universal fairness score is endorsed.
- Maintenance, restoration and contingent liabilities need domain-specific units and uncertainty ranges; they must remain visible without pretending all obligations are fungible.

The research tracker is [CIVDYN-GENERATIONAL-001 #4189](https://github.com/Luminous-Dynamics/symtropy/issues/4189). This tranche is a synthetic accounting and protocol reference, not evidence of real-world intergenerational fairness.
