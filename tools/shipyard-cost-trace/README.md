# Shipyard schedule-to-cost trace (V0)

Dependency-free Rust reference kernel for Symtropy issue #2402. This is a deliberately narrow implementation of the event-ledger contract, not a scheduler, ERP connector, market-price database, vessel simulator, or production instruction system.

## Run

~~~sh
cargo test --locked --offline --manifest-path tools/shipyard-cost-trace/Cargo.toml
cargo run --locked --offline --manifest-path tools/shipyard-cost-trace/Cargo.toml --example synthetic
~~~

The nested Cargo workspace isolates this slice from the main Symtropy workspace and uses no external crates.

## Paradigm shift: plan, simulation, observation and finance are different facts

Each event binds a build, exact vessel-configuration SHA-256, task, stage, event identity, source reference, source-permission disposition, and explicit related-event links. Quantity axes are keyed by semantic identity and unit. Quantity events may carry an optional, versioned unit-rate card; direct financial actuals are a separate event kind and are accepted only at Observed stage.

The audit returns three non-additive views:

1. Quantity reconciliation by build/configuration/task/axis/unit. Observed-minus-planned variance is available only when both an explicit planned quantity and an explicit observed quantity exist. Missing events remain None, never a substitute zero. Different configuration digests do not reconcile against one another.
2. Quantity × rate-card cost, grouped by build/configuration/stage/money basis/rate-card revision. These are estimates at every stage, including rates applied to observed quantities.
3. Direct observed financial transactions, grouped by build and declared currency basis.

Do not add view 2 to view 3. A time record costed at an hourly rate and the invoice/payroll record for that work are alternative views until explicit, source-backed accounting allocation reconciles them. Credit notes may be represented by finite negative financial actuals. Currency basis is not converted implicitly.

## Fail-closed conditions

- Duplicate event IDs are invalid.
- Duplicate line-level financial transaction allocation keys within a build are invalid. Use a source invoice/transaction identifier plus line allocation, not only the invoice header, for the stable key.
- Financial actuals must be Observed; they cannot be entered as plans or simulations.
- Empty source references, unresolved rights/access disposition, malformed configuration digests, missing rate-card basis/revision, negative or non-finite quantities, invalid rates and arithmetic overflow are rejected.
- Missing plans/observations, mismatched configuration revisions, and unresolved related-event references are not auto-filled.
- The implementation checks that source references are present; it does not fetch/authenticate sources, validate license terms, prove payroll was allocated to a task, or establish that a rate is commercially correct.

## Test contract

The library unit suite contains 21 authored tests: known-answer quantity variance; simulated-vs-observed separation; missing-plan semantics; configuration-revision mismatch; rate-card estimate vs invoice separation; duplicate event and transaction identities; stage/evidence constraints; malformed digest/source; unresolved rights; missing rate metadata; overflow; unresolved lineage; cross-build/configuration/stage lineage mismatches; lineage-cycle rejection; configuration-scoped financial actuals; and input-order invariance. Three integration tests in tests/oracle.rs independently check all 16 pairs of planned/observed integer quantities from 0 through 3, separate cost views, and missing-plan semantics.

The next expansion should add an independently written tiny-ledger oracle and frozen fixtures for material movements, rework, overtime, credits, partial execution, and revision propagation. The three oracle tests are executed by the package's dedicated verification workflow; do not claim the oracle gate passed until that exact-head workflow succeeds.

## Industry and scientific boundary

The 2026 review of shipyard production planning reports that hybrid discrete-event simulation plus optimization is comparatively operationally credible under bounded conditions, while publicly reproducible high-resolution shipyard datasets and longitudinal yard-wide KPI validation remain limited ([Bordbar et al., 2026](https://doi.org/10.3390/jmse14040396)). The purpose here is to close a narrow execution-to-economics traceability gap, not to claim the ledger itself improves productivity.

## Benchmark targets (goals, not achieved results)

- 100% agreement with independent hand-calculated quantity/cost fixtures.
- 100% rejection of frozen duplicate, stage-mixing, missing-basis, unit and overflow negative controls.
- Zero absent quantity observations represented as zero.
- 100% of monetary outputs linked to a source reference and declared rate-card revision or direct financial source.
- Deterministic reports under event input permutation.
- Measure p50/p95 audit time and peak memory at 20, 200, 2,000 and 20,000 events.
- Report planned quantity, observed quantity, variance, simulated quantity, rate-card estimate, direct financial actual, currency basis, and unallocated/unknown scope independently.

Every target remains unmeasured until execution and independent review. Synthetic events are never proof of real shipyard savings, manufacturing quality, physical feasibility or vessel certification.
