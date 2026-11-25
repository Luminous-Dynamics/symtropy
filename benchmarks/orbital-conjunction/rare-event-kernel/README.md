# Symtropy Rare-Event Kernel

An isolated, dependency-light Rust package for mathematical primitives used by the orbital conjunction rare-event assurance programme.

This crate is a research kernel, not an operational collision-probability product. It deliberately does not implement orbit propagation, random sampling, candidate event-mode discovery, proposal fitting, flight operations. It includes a Bernoulli-specific sequence and a bounded-mean betting sequence for predictable adaptive importance-sampling contributions.

## Current primitives

- A fixed-stake bounded-mean betting confidence sequence for conditionally-mean-stationary observations, plus an adaptive defensive-importance-sampling adapter that binds event/measure/proposal/sample identities and checks observed normalized weights.
- An end-to-end matched adaptive-IS receipt comparison for fixed-stake, PrPl-EB and Hedged. Every method receives the exact same draw receipts, verifies the same event/measure/proposal/weight identities, and is queried after every draw; the harness compares interval width, threshold decision time, repeated update-plus-summary latency and storage profile.
- A constant-state, O(1)-per-observation predictable plug-in empirical-Bernstein (PrPl-EB) statistical sequence, with variance-adaptive predictable stakes and running intersection; the adaptive-IS receipt adapter validates identities and retains an exact O(n) duplicate-sample ledger.
- A hedged-capital bounded-mean confidence sequence using two direction-specific log-capital processes, parameter-dependent stake caps, a theorem-backed interval sublevel set, a log-tailed caller-configurable candidate grid and conservative adjacent-grid outer bracketing. Update cost and state are O(G) in grid size.
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

The current authored test corpus contains **287 authored `#[test]` cases**:
30 in `lib.rs`, 14 in `adaptive_importance_sampling.rs`, 9 in `decision_gate.rs`, 7 in `decision_refinement.rs`, 25 in `coverage.rs`, 13 in `residual.rs`, 10 in `sequential.rs`, 10 in `stats.rs`, 10 in `trajectory.rs`, 16 in `lebedev.rs`, 9 in `normal_positive_part.rs`, 11 in `conditional_velocity.rs`, 6 in `ncdot_sphere.rs`, 12 in `ncdot_temporal.rs`, 18 in `ncdot_refinement.rs`, 8 in `pop_col_energy.rs`, 10 in `pop_col_adjustment.rs`, and 17 in `pop_convergence.rs`, 13 in `space_time_partition.rs`, and 15 in `refinement_scheduler.rs`.

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
- The hedged-capital numerical state is O(G) in the caller-selected parameter-grid size; its adaptive-IS wrapper adds O(n) exact sample-ID storage for duplicate rejection; its interval is an outer bracket of grid-accepted means using the theorem's interval property, and no accepted grid point causes abstention to [0,1]. It does not claim exact continuous root isolation. PrPl-EB's numeric sequence state is O(1) per update; its adaptive-IS adapter deliberately retains O(n) exact sample identities for duplicate rejection. PrPl-EB is a bounded-mean confidence sequence, not a validation of the target density, event-mode completeness, proposal approximation, or orbital dynamics. Its adaptive-IS application requires predictable proposal choice, actual draws from the complete declared mixture density, and fixed target/event conditional mean. The implementation uses floating-point arithmetic with one-ULP endpoint nudges, not formally verified directed rounding.
- The weighted indicator primitive does not construct confidence intervals or establish finite-sample coverage. The adaptive importance-sampling module uses fixed-stake bounded-mean test supermartingales and running-max inversion; its statistical guarantee requires a predictable proposal, a fixed target/event, the complete defensive-mixture density, and conditional-mean stationarity. The adapter checks observed weight bounds and evidence identities but cannot prove the sampler generated draws from the declared q_t or prove global mixture domination. Its f64 implementation is not a formally verified outward-rounded arithmetic kernel. It is not the empirical-Bernstein or Bayes-assisted confidence-sequence method used in current PLANS 2027 research.
- The Lebedev module includes the NASA CARA 5,810-point, degree-131 sphere rule and low-order 6/14/26 rules. It does not reproduce the 3D-Nc collision-rate integrand, PeakOverlapPos solver or adaptive temporal integration. Significant-node diagnostics and successive-rule differences are indicators, not formal quadrature error bounds; compare multiple orders and establish convergence separately.
- The proposal density must be the complete mixture density, including all defensive components.
- The space-time partition prototype is limited to a synthetic two-hemisphere surface split and caller-supplied constant rational integrand enclosures. It verifies partition completeness and contribution accounting only; it does not construct arbitrary sphere patches, enclose the physical Ncdot integrand, propagate trajectory/covariance intervals, or provide outward floating-point arithmetic or a qualified Nc bound.
- The decision-sufficiency gate and planning controller only classify caller-supplied bounds and propose work orders. They do not execute refinements, shrink intervals, validate returned child partitions or qualify physical estimates.
- The decision-sufficiency gate only classifies caller-supplied bounds. It does not prove their provenance, coverage, physical-model adequacy or calibration, and it does not select the policy threshold or authorize a maneuver.
- The refinement scheduler ranks by contribution-width-per-estimated-cost only. It does not estimate cost from runtime observations, model the distribution of refinement outcomes, calculate full decision-theoretic value of information, prove greedy optimality, or establish that a selected refinement will reduce uncertainty. Estimated cost is an input assumption, and every selected candidate remains unresolved until a separate validated refinement receipt is accepted. The test-only exhaustive oracle is restricted to at most 20 candidates and is a benchmark reference, not a production scheduler.


## Finite adaptive proposal execution conformance

`proposal_execution_conformance.rs` is a finite-support reference sampler/verifier that addresses a gap left by density-only draw receipts: a receipt can report plausible target and proposal densities without demonstrating that the sample was generated by the declared proposal.

The reference model uses four finite atoms and integer probability masses with ticket space (S=256). A branch ticket selects the defensive target component with probability (alpha), otherwise the residual component; a separate atom ticket is inverse-CDF mapped through the selected component. The verifier then recomputes, with integer arithmetic:

[
q_t(i)=alpha p(i)+(1-alpha)r_t(i)
]

[
Y_t(i)=alpha,mathbf{1}_E(i)rac{p(i)}{q_t(i)}in[0,1].
]

The residual proposal has two declared versions. The verifier selects the next version from the **previously accepted event outcome**, and rejects a residual or complete-mixture proposal identity inconsistent with that history. The residual proposal digest and complete mixture digest are distinct fields: only the latter is transferred to the confidence-sequence adapter as the identity of q_t. It also rejects sample replay, sequence gaps, identity changes, out-of-range tickets, and atom identities inconsistent with the inverse-CDF transform. Failed receipts do not mutate verifier state.

A verified draw can be converted directly into the existing `AdaptiveImportanceDraw` type. The conversion reconstructs the target categorical density and complete defensive-mixture density from the exact integer mass ledgers, maps zero target mass to negative infinity, and carries the complete-proposal identity, event/measure/component identities and fixed mixture weight. The resulting draw can be passed unchanged to the existing fixed-stake, PrPl-EB or Hedged receipt adapters. Tests exercise all three adapter integrations and check the fixed-stake normalized contribution against the exact finite ratio.

The exhaustive single-draw fixture enumerates all (256^2=65{,}536) branch/atom ticket pairs. It checks exact integer frequencies against the defensive mixture and verifies the weighted event mean identity (mathbb{E}[Y_t]=alpha P_p(E)). A second test confirms the adaptive residual proposal changes only after a prior event outcome has been accepted.

### Claim boundary

- This is a finite categorical execution-conformance model, not a production orbital sampler or a continuous-distribution sampler.
- The verifier proves the mapping from the **supplied** tickets to the declared mixture draw; it does not prove that the ticket source is uniform, independent, unpredictable, or free from manipulation. RNG quality, entropy custody and domain separation need separate qualification.
- The sampler, policy, RNG, seed commitment and stream identities are pinned as supplied identifiers; this module does not cryptographically attest their binaries or prove those identifiers describe the executed implementation.
- The finite-support calculation does not establish global defensive-mixture domination for a continuous orbital target measure, proposal support completeness, event-mode completeness, covariance realism, orbit-model adequacy or physical collision risk.
- The exact ticket enumeration is a finite reference test, not a field coverage result or SOTA claim. The conversion into `AdaptiveImportanceDraw` closes the finite-support adapter boundary only; production Gaussian/GMM sampling, proposal-density evaluation, RNG quality, continuous-domain domination and event-mode completeness still require independent qualification.
