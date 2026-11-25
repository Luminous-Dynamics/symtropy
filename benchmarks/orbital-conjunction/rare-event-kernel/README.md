# Symtropy Rare-Event Kernel

An isolated, dependency-light Rust package for mathematical primitives used by the orbital conjunction rare-event assurance programme.

This crate is a research kernel, not an operational collision-probability product. It deliberately does not implement orbit propagation, random sampling, candidate event-mode discovery, proposal fitting, flight operations. It includes a Bernoulli-specific sequence and a bounded-mean betting sequence for predictable adaptive importance-sampling contributions.

## Current primitives

- A fixed-stake bounded-mean betting confidence sequence for conditionally-mean-stationary observations, plus an adaptive defensive-importance-sampling adapter that binds event/measure/proposal/sample identities and checks observed normalized weights.
- An exact-identity before/after action-effect portfolio reconciler that reports new, absent, definitely elevated, definitely reduced and indeterminate events; binds the same protected-object/screening-horizon scope, surfaces catalog-revision changes and explicit evidence gaps, checks full-catalog/inventory declarations and requires coordination evidence when the secondary is maneuverable. A known above-threshold event remains visible even when other evidence is incomplete.
- Exact-rational three-way action-threshold classification for already-qualified risk intervals, with explicit inclusive/strict boundary semantics and abstention when the interval overlaps the threshold.
- Decision-gated refinement planning that proposes scheduler work only for indeterminate assessments and preserves unresolved candidate identities when budget or selectable work is unavailable.
- Strict positive-definite covariance validation using a Cholesky factor.
- Multivariate Gaussian log density without explicit matrix inversion.
- Gaussian half-space probability and log-probability, reduced to a one-dimensional normal CDF.
- Analytic tail-probability sweep from 1e-3 to 1e-9 across dimensions 1, 2, 3, 6 and 12.
- Stable log-domain standard-normal interval mass, including extreme positive and negative tails.
- Log mass for axis-aligned boxes under independent standard-normal coordinates.
- A Gaussian cube-tail union-bound primitive for probability bookkeeping.
- Outward-rounded interval enclosures for affine images of finite axis-aligned cells.
- A conservative norm lower bound and static collision-ball cell-exclusion predicate.
- Deterministic longest-axis cell subdivision with stable child split paths.
- Conservative standard-normal cell-mass upper bound using a cell-local density ceiling that tightens with distance from the nominal Gaussian mean.
- A Gaussian Chernoff / union-bound estimate for mass outside the finite whitened cube, plus a residual-mass ledger for unresolved cells.
- A budgeted affine-cell traversal that separately classifies cells wholly inside caller-declared chart boxes, cells proven event-free, and unresolved cells outside chart coverage.
- A disconnected-event-union verifier with component-specific chart assignments. A cell is accounted only when each event component is either covered by its own chart or proven event-free; one chart cannot suppress another mode, even when their input-space regions overlap.
- An interval-polynomial time-window classifier that proves no-contact / guaranteed-contact only by enclosing the entire relevant time interval, and otherwise returns an explicit ambiguous disposition.
- Dense-grid adversarial tests checking sampled affine images remain inside the interval enclosure and sampled norms remain above the reported lower bound.
- Streaming log-domain weighted event sums, contribution ESS, plug-in relative standard error, and a clearly labelled normal-approximation interval diagnostic.
- A beta(1,1)-mixture Bernoulli e-process and inverted time-uniform confidence sequence for independent bounded event indicators, with an exhaustive short-horizon optional-stopping coverage test.
- Full normalized Gaussian-mixture log density using log-sum-exp.
- Log importance weights and one-draw weighted event indicators.
- Strict Loewner covariance-floor validation, including the strictness test needed for a half-nominal covariance lower bound.
- A stable scalar Gaussian negative-part expectation E[max(-X, 0)] using pure-Rust erfc in the moderate regime and a log-domain Mills-ratio tail expansion for large positive means; it preserves log expectation when f64 underflows.
- A fixed-size conditional relative-motion covariance kernel that forms Q1 = B*A⁻¹ and Q2 = C - B*A⁻¹*Bᵀ, checks Schur-complement PSD without jitter/clipping, and evaluates directional conditional means, variances and Mahalanobis distances. NASA's non-positive-variance fallback and the mathematical zero-variance limit are separate explicit modes.
- A fixed-time Ncdot sphere integrator that evaluates every selected Lebedev node using the conditional Gaussian model, accumulates weighted contributions with log-sum-exp, preserves node and total underflow/overflow states, and reproduces NASA's strict 1e-3 significant-node threshold.
- A temporal Ncdot aggregator that trapezoid-integrates expected collision count in log space, computes the Ncdot-rate-weighted average significant-node count, tracks sampled peaks and plateaus, and refuses qualification when the caller's horizon declaration is incomplete, an epoch gap exceeds policy, or upstream samples are unqualified.
- Deterministic positive-weight Lebedev sphere rules with 6, 14, 26 and 5,810 nodes (algebraic precision 3, 5, 7 and 131), plus a validated nonnegative-sample integrator. The 5,810-point rule is generated from 144 symmetry-orbit records transcribed from NASA CARA Analysis Tools at immutable source commit `43f0160f94d76a91805d86521c3e8fa0d09ecd68` (November 2026 update).
- NASA-style Lebedev contribution diagnostics using the strict 1e-3-of-maximum integrand threshold, with significant-node count, significant solid-angle weight, significant integral share and largest-node share.
- A deterministic 6 → 14 → 26 point refinement ladder that compares successive integral estimates and keeps numerical stability separate from significant-node adequacy; its output is a refinement diagnostic, not a formal error bound.

## Run the isolated tests

From the repository root:

```sh
cargo generate-lockfile --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml
cargo fmt --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml --all-targets
cargo clippy --locked --manifest-path benchmarks/orbital-conjunction/rare-event-kernel/Cargo.toml --all-targets -- -D warnings
```

The package has its own Cargo workspace and does not require the full Symtropy/Bevy workspace.

The current authored test corpus contains **279 authored `#[test]` cases**:
30 in `lib.rs`, 16 in `action_effect_portfolio.rs`, 14 in `adaptive_importance_sampling.rs`, 9 in `decision_gate.rs`, 7 in `decision_refinement.rs`, 25 in `coverage.rs`, 13 in `residual.rs`, 10 in `sequential.rs`, 10 in `stats.rs`, 10 in `trajectory.rs`, 16 in `lebedev.rs`, 9 in `normal_positive_part.rs`, 11 in `conditional_velocity.rs`, 6 in `ncdot_sphere.rs`, 12 in `ncdot_temporal.rs`, 18 in `ncdot_refinement.rs`, 8 in `pop_col_energy.rs`, 10 in `pop_col_adjustment.rs`, and 17 in `pop_convergence.rs`, 13 in `space_time_partition.rs`, and 15 in `refinement_scheduler.rs`.

The package also includes:
- A deterministic NASA-style sampled Ncdot refinement controller for boundary extension, top-four sampled peaks, zero-rate MD2 fallback, dominant trapezoid contributions and significant-rate spacing. It does not certify between-sample peak completeness.
- A PeakOverlapPos center-of-linearity conditional-velocity and specific-energy kernel with explicit km, km/s, km³/s² and km²/s² units.
- A NASA-compatible CoL energy-change limiter using the 201-point [0,1] scan, source extrema endpoint/plateau behavior, bounded midpoint refinement and a replayable fraction-selection report.
- A typed PeakOverlapPos convergence controller preserving the source convergence values: strict 1.0, oscillation 0.5, relaxed 0.1–0.35 and not-converged 0.0. It preserves strict/relaxed thresholds, maximum-iteration ordering, CoL-adjustment gating and delayed two-cycle detection. It consumes caller-computed MD2 values and does not independently qualify their covariance metric.
- An exact synthetic space-time × sphere product-partition verifier. The first prototype tiles integer-second horizons with contiguous slabs, requires the exact north/south hemisphere measure, binds event/model/target digests, rejects missing/duplicated product cells, and recomputes rational π-scaled contributions independently.
- A deterministic uncertainty-per-cost refinement scheduler using exact rational contribution widths and estimated compute costs, stable identity tie-breaking, greedy budget selection, and a complete unresolved-candidate ledger. Scheduling does not mark work evaluated or remove residual uncertainty. Its heuristic is explicitly not optimal knapsack scheduling; finite-fixture regret is measured against a test-only exhaustive subset oracle.
- An exact-rational decision-sufficiency gate that classifies a supplied risk envelope as above threshold, below threshold, or indeterminate under an explicit inclusive or strict policy rule. It does not qualify the envelope or authorize action.
- A decision-gated planning state machine that stops scheduling once the interval is decisive, proposes work only while the interval is indeterminate, and explicitly abstains on budget exhaustion, absent candidates, or no selectable work. Proposed refinements remain unresolved and require an independent completion receipt.

## Important limitations

- The covariance floor helper checks only the matrix inequality. It does not prove all assumptions of the constrained-mixture finite-variance theorem.
- A finite proposal mixture does not prove event-mode completeness.
- The Gaussian half-space function is an analytic fixture, not a disk/ellipsoid collision integral or trajectory first-contact solver.
- The log-tail, box-mass and cube-tail routines are floating-point numerical kernels, not formally verified outward-rounded probability bounds; they cannot be used as an independent residual-mass certificate without additional enclosure work.
- The affine-cell module widens basic IEEE-754 operations by one adjacent representable value and fails closed on overflow. Its grid tests are useful negative controls, not a formal proof of the arithmetic implementation. It covers finite affine maps only; it is not a validated nonlinear ODE enclosure.
- The residual traversal bounds possible event mass only outside component-specific caller-declared chart boxes under a fixed standard-normal nominal model and static affine collision-ball event union, with chart-to-component assignments. A chart-covered region is not thereby proven to have accurate proposal density, correct likelihood weights, or complete event support.
- The polynomial trajectory classifier accepts coefficient intervals supplied by the caller. It does not construct or validate a Taylor remainder, establish the coefficient enclosures, solve nonlinear dynamics, or formally machine-verify its interval arithmetic. It is a fail-closed local component; ambiguous cases require high-fidelity evaluation.
- The Gaussian cell-mass bound is intentionally loose and the cube-tail bound uses a Chernoff polynomial lower bound. The traversal is preliminary numerical conservative bookkeeping, not a formally machine-verified certificate or a proof of the physical model.
- The cell splitter uses a deterministic longest-axis rule. Shared closed faces are covered by both children; they have zero measure only for a nondegenerate continuous Gaussian and still remain relevant to geometric tangency tests.
- The action-effect portfolio reconciler compares caller-supplied event inventories and probability envelopes only. It cannot validate that catalog screening was actually complete, prove event identities capture all post-action encounters, propagate action effects, quantify new-event consequences, or infer zero risk for an event absent from the declared horizon. Its below-threshold disposition is not maneuver authorization or flight-safety certification.
- The weighted indicator primitive does not construct confidence intervals or establish finite-sample coverage. The adaptive importance-sampling module uses fixed-stake bounded-mean test supermartingales and running-max inversion; its statistical guarantee requires a predictable proposal, a fixed target/event, the complete defensive-mixture density, and conditional-mean stationarity. The adapter checks observed weight bounds and evidence identities but cannot prove the sampler generated draws from the declared q_t or prove global mixture domination. Its f64 implementation is not a formally verified outward-rounded arithmetic kernel. It is not the empirical-Bernstein or Bayes-assisted confidence-sequence method used in current PLANS 2027 research.
- The Lebedev module includes the NASA CARA 5,810-point, degree-131 sphere rule and low-order 6/14/26 rules. It does not reproduce the 3D-Nc collision-rate integrand, PeakOverlapPos solver or adaptive temporal integration. Significant-node diagnostics and successive-rule differences are indicators, not formal quadrature error bounds; compare multiple orders and establish convergence separately.
- The proposal density must be the complete mixture density, including all defensive components.
- The space-time partition prototype is limited to a synthetic two-hemisphere surface split and caller-supplied constant rational integrand enclosures. It verifies partition completeness and contribution accounting only; it does not construct arbitrary sphere patches, enclose the physical Ncdot integrand, propagate trajectory/covariance intervals, or provide outward floating-point arithmetic or a qualified Nc bound.
- The decision-sufficiency gate and planning controller only classify caller-supplied bounds and propose work orders. They do not execute refinements, shrink intervals, validate returned child partitions or qualify physical estimates.
- The decision-sufficiency gate only classifies caller-supplied bounds. It does not prove their provenance, coverage, physical-model adequacy or calibration, and it does not select the policy threshold or authorize a maneuver.
- The refinement scheduler ranks by contribution-width-per-estimated-cost only. It does not estimate cost from runtime observations, model the distribution of refinement outcomes, calculate full decision-theoretic value of information, prove greedy optimality, or establish that a selected refinement will reduce uncertainty. Estimated cost is an input assumption, and every selected candidate remains unresolved until a separate validated refinement receipt is accepted. The test-only exhaustive oracle is restricted to at most 20 candidates and is a benchmark reference, not a production scheduler.


## Exact joint-action outcome catalogue

`joint_action_catalogue.rs` extends the single-action before/after reconciler with a closed finite coalition universe. Up to 16 candidate actions are enumerated into their statically selectable subsets (actionable and within the resource cap). Every such subset, including the empty no-action control, must have exactly one explicit outcome row.

A coalition row is one of three states:
- Qualified: exact action-set identity, full post-action event inventory, fixed risk semantics and catalog revision, full-catalog screening, a pinned consequence-model identity, a robust consequence-avoided lower-bound score and any required coordination receipt.
- Infeasible: a reason-bound statement that this exact coalition cannot be executed.
- Unresolved: missing or ambiguous evidence, which forces global abstention.

The oracle consumes a qualified outcome for the exact combination. It does not infer the result of A+B from the independently assessed results for A and B. Each qualified post-action event probability interval is checked against the inclusive threshold; any confirmed exceedance excludes that coalition, while threshold overlap, incomplete screening, catalog drift, or missing coordination prevents a global optimum claim. It selects by robust consequence avoided, then lower resource cost, then lexicographic action-set identity.

The tests include a non-additivity counterexample where each singleton is below threshold but the joint action creates a new above-threshold event, a higher-benefit safe synergy case, missing and unresolved rows, over-capacity/non-actionable exclusions, cross-operator coordination, consequence-model mismatch and deterministic tie-breaking.

### Claim boundary

- This is a bounded policy/reference oracle over caller-supplied coalition outcomes. It does not generate maneuver trajectories, propagate dynamics, screen the catalog, calculate collision probabilities or independently qualify consequence values.
- The robust consequence score is caller-supplied and bound to a model digest; the oracle does not validate that model or score producer.
- A missing or unresolved statically selectable coalition forces abstention. A complete exact optimum is claimed only over the declared finite action universe, resource constraints and supplied evidence.
- The 16-action bound is a tractability limit for exhaustive reference enumeration, not a production optimizer scale claim. No autonomous maneuver authority is created.


### Signed consequence deltas

The joint-action oracle now uses a signed integer lower bound for consequence avoided. This matters because a coalition may pass the hard collision-risk threshold while still increasing some separately modelled consequence (for example, manoeuvre cost, debris exposure or another lifecycle loss). Such a coalition must not be clamped to zero or confused with the no-action baseline.

The selector compares the signed lower bound only after the non-compensatory physical-safety and evidence gates have passed. If no-action is safe, its exact zero score dominates a negative-scored action, all else equal. If no-action is unsafe, it is excluded by the physical-safety gate and safe negative-scored actions remain comparable by their signed values. The selector chooses the greatest signed lower bound, then lower resource cost, then canonical action IDs.

This is a caller-supplied score in v0.1, not an independently recomputed consequence. Binding that delta to the full joint physical-world/loss-model scenario receipt and independent model-universe qualification is tracked separately in issue #7466.
