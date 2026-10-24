# Shipyard schedule-to-cost trace (V0)

Dependency-free Rust reference kernel for Symtropy issue #2402. This is a deliberately narrow implementation of the event-ledger contract, not a scheduler, ERP connector, market-price database, vessel simulator, or production instruction system.

## Run

~~~sh
cargo test --locked --offline --manifest-path tools/shipyard-cost-trace/Cargo.toml
cargo run --locked --offline --manifest-path tools/shipyard-cost-trace/Cargo.toml --example synthetic
cargo run --release --locked --offline --manifest-path tools/shipyard-cost-trace/Cargo.toml --example scale
~~~

The nested Cargo workspace isolates this slice from the main Symtropy workspace and uses no external crates.

## Paradigm shift: plan, simulation, observation and finance are different facts

Each event binds a build, exact vessel-configuration SHA-256, task, stage, event identity, source reference, source-permission disposition plus a permission/terms reference, and explicit related-event links. The permission status is not self-authenticating: the kernel checks that a reference is present but does not validate the referenced licence or authorization. Quantity axes are keyed by semantic identity and unit. Quantity events may carry an optional, versioned unit-rate card; direct financial actuals are a separate event kind and are accepted only at Observed stage.

**Important intake boundary:** `QuantityEvent` now carries an explicit `QuantityAggregationSemantics` marker, and `audit` rejects `CumulativeSnapshot` values rather than silently summing them. `event_semantics::normalize_quantities` is the source-record normalization boundary: records declare `Delta`, `CumulativeSnapshot`, `Correction`, or `Reversal`, use a stable stream ID and sequence, and preserve both source event time and system-recorded time. Both event-time and recorded-time cutoffs must include a record before it affects an as-of report. Thus cumulative snapshots of 12 hours and then 14 hours normalize to 14, while additive deltas of 12 and 14 normalize to 26. Corrections replace one earlier delta, reversals cancel one earlier delta, and the original records are not mutated. Duplicate identities/sequences, causal or cross-stream corrections, ambiguous corrective actions, mixed snapshot/delta semantics, contradictory snapshot sequence/time, and multiple snapshot streams for one quantity identity fail closed. `NormalizedQuantity` returns the effective value plus source record IDs, stream IDs, and cutoff; the caller must still carry those provenance links into its ledger event and mark the normalized scalar as an additive contribution. This is a normalization API, not an ERP/MES connector or a source-authentication mechanism. See [SHIPYARD-EVENT-SEMANTICS-001 #2458](https://github.com/Luminous-Dynamics/symtropy/issues/2458).

The audit returns three non-additive views:

1. Quantity reconciliation by build/configuration/task/axis/unit. Observed-minus-planned variance is available only when both an explicit planned quantity and an explicit observed quantity exist. Missing events remain None, never a substitute zero. Different configuration digests do not reconcile against one another.
2. Quantity × rate-card cost, grouped by build/configuration/task/quantity axis/unit/stage/money basis/rate-card revision. These are estimates in the declared minor-unit denomination at every stage, including rates applied to observed quantities; a fractional estimate is possible for fractional quantities.
3. Direct observed financial transactions, grouped by build/configuration/task and declared currency basis.

Do not add view 2 to view 3. A time record costed at an hourly rate and the invoice/payroll record for that work are alternative views until explicit, source-backed accounting allocation reconciles them. Direct financial actuals use exact signed integer minor units, including credits; checked integer addition rejects overflow. Every monetary entry carries a currency basis ID plus an explicit minor-unit exponent from 0 through 6. The basis ID should identify currency, price year, region and denomination; reusing one basis ID with conflicting exponents is invalid. Rate-card output uses the same minor-unit denomination but remains a floating-point estimate, potentially fractional for fractional quantities. Currency basis is not converted implicitly.

## Fail-closed conditions

- An empty ledger is invalid; empty evidence is not interpreted as a zero-cost build.
- Duplicate event IDs are invalid.
- Duplicate line-level financial transaction allocation keys within a build are invalid. Use a source invoice/transaction identifier plus line allocation, not only the invoice header, for the stable key.
- Financial actuals must be Observed; they cannot be entered as plans or simulations. Minor-unit exponents outside 0..=6 or conflicting exponents under the same currency-basis ID are rejected.
- Empty source references, missing rights/terms reference, unresolved rights/access disposition, malformed configuration digests, missing rate-card basis/revision, negative or non-finite quantities, invalid rates and arithmetic overflow are rejected.
- Missing plans/observations, mismatched configuration revisions, unresolved references, and cyclic lineage are not auto-filled.
- Financial actual totals remain partitioned by vessel configuration revision and sum in exact integer minor units. Rate-card estimates remain floating-point estimates and use stable event-ID accumulation order for repeatability; this does not make model estimates exact decimal accounting.
- The implementation checks that source references are present; it does not fetch/authenticate sources, validate license terms, prove payroll was allocated to a task, or establish that a rate is commercially correct.

## Test contract

The core accounting tests cover quantity variance, simulation/observation separation, missing-plan semantics, configuration-revision mismatch, estimate-vs-invoice separation, duplicate identities, evidence and stage constraints, malformed digests/source, rights, rate metadata, overflow, lineage consistency/cycles, exact minor-unit totals/credits, denomination conflicts, empty-ledger rejection, a 5,000-event chain, input-order invariance, and rejection of cumulative snapshots. The event-semantics module adds 14 unit tests for delta/snapshot behavior, correction/reversal, bitemporal cutoffs, lineage, ambiguity, stable source identity, and invalid sequence/time combinations. Test counts are authored coverage, not verification evidence until a workflow passes on the exact head.

Seven integration tests live in `tests/oracle.rs`, `tests/reference_oracle.rs` and `tests/event_semantics_oracle.rs`. Three exercise all 16 planned/observed integer pairs from 0 through 3, separate cost views, and missing-plan semantics. Two use an independently implemented, deliberately simple valid-input evaluator over a frozen 16-event synthetic ledger covering multi-stage labor, overtime, materials, scrap, rework, partial/missing observation, invoice+credit, and build/configuration partitioning. One pins the ledger corpus row count, event-kind counts and unique event identities. The event-semantics oracle parses a separate frozen source TSV and hand-calculated expected-output TSV for delta sums, correction/reversal, 12→14 snapshots, and earlier-versus-later bitemporal cutoffs. The CI workflow pins both new TSV raw-byte SHA-256 digests. Hand-calculated outputs for the original ledger are documented in [tests/fixtures/README.md](tests/fixtures/README.md). Reference evaluators do not call production aggregation helpers; they do not yet independently validate every malformed-input rule, which remains a separate required layer.

The dedicated workflow checks formatting, the full current test suite, Clippy, release-mode scale characterization and deterministic output. Its measured synthetic numbers are reported in the PR body with the exact tested commit; they are not a shipyard-productivity benchmark. Never transfer results between commits or claim a gate passed unless it reports success on the exact head.

## Industry and scientific boundary

The 2026 review of shipyard production planning reports that hybrid discrete-event simulation plus optimization is comparatively operationally credible under bounded conditions, while publicly reproducible high-resolution shipyard datasets and longitudinal yard-wide KPI validation remain limited ([Bordbar et al., 2026](https://doi.org/10.3390/jmse14040396)). The purpose here is to close a narrow execution-to-economics traceability gap, not to claim the ledger itself improves productivity.

## Interoperability and credibility context

NIST's July 2026 Digital Twins Workshops summary (NISTIR 8620) identifies interoperability, verification/validation/uncertainty quantification (VVUQ), cybersecurity and workforce readiness as continuing manufacturing digital-twin challenges: https://doi.org/10.6028/NIST.IR.8620. NIST's ongoing Digital Twins for Advanced Manufacturing programme also emphasizes reference architectures, standards, data exchange, VVUQ, and lifecycle digital threads: https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing.
NIST's October 2026 report on uncertainty quantification discusses the need to harmonize modeling and metrology practices and distinguish variance propagation from distribution-based uncertainty treatment: https://doi.org/10.6028/NIST.AMS.100-85. In this ledger, deterministic accounting arithmetic must not be mistaken for calibrated measurement or model uncertainty.

This Rust kernel addresses only the narrow event identity, quantity reconciliation and accounting-view boundary. It does not yet implement a standard transport payload, ISO 23247 conformance, MTConnect adapter, cybersecurity threat model, or full VVUQ process. Those should be explicit adapter and assurance layers, not implied by the existence of a digital-twin-style event ledger.

## Benchmark targets (goals, not achieved results)

- 100% agreement with independent hand-calculated quantity/cost fixtures.
- 100% rejection of frozen duplicate, stage-mixing, missing-basis, unit and overflow negative controls.
- Zero absent quantity observations represented as zero.
- 100% of monetary outputs linked to a source reference and declared rate-card revision or direct financial source.
- Deterministic reports under event input permutation.
- The release-mode scale example measures median/p95 audit latency at 20, 200, 2,000 and 20,000 chained events (21 samples per size); it deliberately reverses event order to exercise long-lineage traversal. It does not measure peak memory, industrial throughput or end-to-end shipyard performance.
- Report planned quantity, observed quantity, variance, simulated quantity, rate-card estimate, direct financial actual, currency basis, and unallocated/unknown scope independently.

Every target remains unmeasured until execution and independent review. Synthetic events are never proof of real shipyard savings, manufacturing quality, physical feasibility or vessel certification.
