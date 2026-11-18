# Human multirate co-simulation kernel v0.1

**Evidence class:** synthetic mathematical software benchmark only.  
**Human-subject data:** none.  
**Parent:** [HUMAN-MULTIRATE-CO-SIM-001 / #5871](https://github.com/Luminous-Dynamics/symtropy/issues/5871).  
**Programme:** [HUMAN-IN-SILICO-MODEL-TOURNAMENT-001 / #5847](https://github.com/Luminous-Dynamics/symtropy/issues/5847).

## Purpose

This isolated, dependency-free Rust package benchmarks how different coupling/integration strategies behave on a two-compartment exchange system with a closed-form solution.

It is a narrow kernel for qualifying numerical behavior. It is **not** a physiological model, FMI importer, generic solver, clinical tool, or claim of human validity.

All capacities, conductances, states and times are arbitrary mathematical fixtures.

## Model and exact oracle

The system is

\[
C_a \dot{x}_a=-G(x_a-x_b), \qquad
C_b \dot{x}_b=G(x_a-x_b).
\]

Its capacity-weighted invariant is

\[
I=C_a x_a+C_b x_b.
\]

The relative state follows

\[
x_a(t)-x_b(t)=(x_a(0)-x_b(0))
\exp\left[-G\left(\frac{1}{C_a}+\frac{1}{C_b}\right)t\right].
\]

This yields an analytic known-answer oracle for every sampled time.

## Methods compared

| Method | Semantics | Expected failure/strength exposed |
|---|---|---|
| Parallel zero-order hold | Both subsystems advance from the same stale interface snapshot | Explicit coupling error and balance drift |
| Serial A then B | Second subsystem consumes the first subsystem's updated value | Order dependence and balance drift |
| Serial B then A | Reverse execution order | Order-dependence control |
| Conservative shared-flux Euler | One interface transfer is debited and credited exactly once | Conservation with conditional stability/positivity |
| Implicit midpoint reference | Monolithic reference integrator for this linear problem | Second-order accuracy and A-stability; not L-stable |
| Backward Euler reference | Monolithic reference integrator for this linear problem | First-order accuracy, L-stability and robust damping |

The two implicit entries are **not** generic FMI master algorithms. They are monolithic numerical reference methods for this known linear system. The adaptive shared-flux entry estimates local error by comparing one full Euler step with two half steps, accepts the two-half result, and rejects/reduces steps when the error estimate or positivity guard fails. Its absolute/relative tolerances apply to the local error estimate; global trajectory error is measured independently against the analytic oracle. A future FMI lane must probe each FMU's actual capabilities, including state save/restore, early return, clocks and intermediate updates.

## Scenarios

The command-line report executes:
- balanced capacities;
- asymmetric capacities;
- a stiff, highly asymmetric capacity/conductance regime.

Each is evaluated at multiple communication steps. The CSV keeps state error, RMS error, capacity-weighted balance residual, minimum state, positivity and the dimensionless explicit-Euler stability number separate.

For the shared-flux Euler method, the linear relative mode has amplification

\[
1-Gh\left(\frac{1}{C_a}+\frac{1}{C_b}\right).
\]

Its absolute-stability condition is that the magnitude of this factor is less than one. Conservation alone therefore does not guarantee stability or positivity.

## Run

From the repository root, execute:

    cargo fmt --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --all-targets --locked -- -D warnings
    cargo run --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --release

The executable emits a CSV scorecard to stdout.

## Qualification boundary

A passing test establishes behavior of this mathematical benchmark implementation. It does not establish:
- fidelity of a physiological model;
- validity of a model outside its source-defined domain;
- correctness of a third-party FMU;
- numerical superiority over preCICE, Modelica tools, mosaik or another industry simulator;
- any human health, performance or clinical conclusion.

Better-than-industry claims require matched models, same scenarios and budgets, source-pinned reference implementations, lower-tail robustness, uncertainty and compute accounting, plus independent reproduction.

## Graph-coupled network extension

The `network` module extends the two-node fixture to generic undirected compartment graphs. For capacities `C_i` and edge conductances `g_ij` it uses

\[
C_i \dot{x}_i = \sum_j g_{ij}(x_j-x_i), \qquad C\dot{x}=-Lx.
\]

The independent oracle diagonalizes the symmetric matrix \(C^{-1/2}LC^{-1/2}\) and evaluates its matrix exponential. It exposes multiple decay modes, connected-component zero modes, conservation of \(\sum_i C_i x_i\), and monotonic decay of \(\frac12\sum_i C_i x_i^2\).

The command-line scorecard now includes a heterogeneous three-node chain and four-node cycle. Candidate methods include simultaneous shared-edge Euler, conservative edge sweeps in both orders, implicit midpoint, and backward Euler. This is a small-graph mathematical oracle; it is not a scalable graph solver or a physiological model.

The graph extension is tracked by [HUMAN-NESTED-NETWORK-001 / #5944](https://github.com/Luminous-Dynamics/symtropy/issues/5944). All graph weights and states are arbitrary mathematical fixtures, not human-derived parameters.


### Adaptive graph-network controller

The graph module also provides adaptive simultaneous shared-flux subcycling. It:
- compares one full network step with two half steps;
- accepts the two-half-step state;
- rejects steps that exceed the local error indicator;
- rejects negative states when the initial state is nonnegative;
- rejects steps that increase the weighted quadratic energy beyond a floating-point slack;
- preserves the weighted total through paired edge fluxes;
- scores global trajectory error independently against the spectral oracle.

The configured local absolute/relative tolerance is an error-indicator target, not a global error guarantee. The report keeps accepted/rejected steps, global error, conservation, dissipation, positivity and step sizes distinct. This remains a small-graph synthetic mathematical reference.

### Event-aligned topology changes

The `simulate_network_topology_events` reference runs piecewise-constant graphs. Each topology event is an exact model-time boundary: the segment before the event uses the old edge set, the accepted endpoint state is carried unchanged into the new graph, and a new spectral oracle is built for the next segment. Events that are reordered, outside the horizon or not aligned with the fixed communication grid are rejected; v0.1 does not round event times or interpolate across topology changes.

The runner includes one three-node conductance-switch example. This qualifies event splitting and state continuity for a tiny synthetic graph, not a general asynchronous federation.

## Graph sensor-placement tournament

The `sensor_design` module composes the graph spectral oracle with a small exact sensor-subset search. It builds the finite-horizon information matrix using the full declared observation-noise covariance and the spectral transition matrix at each observation time. It reports observability rank, information eigenvalues, minimum/maximum eigenvalue, condition number, log determinant and trace-inverse only when full rank.

The exact selector compares E-optimal, D-optimal and A-optimal objectives over all budget-feasible subsets (up to 16 candidate sensors). A separate robust lane enumerates every single declared common-cause group loss and scores the remaining observable subspace. Unknown groups and independent sensor failures are not inferred.

This is a tiny known-truth mathematical reference, not a scalable sensor-placement solver and not a physical sensor-performance result. It is designed to export candidate-visible layouts and event histories to Symthaea #12728 while keeping evaluator truth sealed. The work is tracked in [HUMAN-NESTED-SENSOR-CO-DESIGN-001 / #5977](https://github.com/Luminous-Dynamics/symtropy/issues/5977).

## Active input design and model discrimination

The `network::SpectralOracle` now evaluates exact constant-forcing response for a frozen linear graph. It solves `C * dx/dt = -L*x + forcing` through the symmetric spectral basis, including zero modes without division by zero. Piecewise-constant forcing profiles are composed by carrying the exact state from one segment into the next.

The `experiment_design` module compares candidate graph structures under a shared bounded input and selected sensors. Its first reference score is covariance-aware pairwise Gaussian equal-covariance Gaussian KL separation:

`0.5 * sum_time[(mu_i - mu_j)^T * R^(-1) * (mu_i - mu_j)]`.

The tournament maximizes the minimum pairwise separation across candidate model pairs, with deterministic ties resolved by lower integrated squared input, lower peak amplitude and profile ID. It compares a passive zero-input profile with a bounded pulse on a three-node chain versus a chain with a bypass edge.

This is a deterministic known-covariance, fixed-candidate-parameter reference. It does not yet solve a continuous optimal-control problem, profile nuisance parameters, infer model probabilities, account for temporal noise correlation or claim that the candidate set contains the true system. It is not a physiological model. Work is tracked in [HUMAN-NESTED-EXCITATION-DESIGN-001 / #5982](https://github.com/Luminous-Dynamics/symtropy/issues/5982).

The score is the equal-covariance Gaussian KL divergence for fixed candidate parameters and observation covariance, summed over independent observation-time blocks. It is not the general nuisance-parameter-profiled T-optimal design objective. The next qualification layer must profile candidate-specific nuisance parameters, account for model discrepancy and compare against a Pontryagin/optimal-control reference before making a broader optimal-design claim.

### Finite pulse-catalogue search

The active-input runner now enumerates a deterministic finite grid over:
- injection node: 0, 1, 2;
- pulse amplitude: -1.0, -0.5, +0.5, +1.0;
- pulse duration: 0.25, 0.5, 1.0.

Every candidate gets a stable profile ID. Feasible profiles are scored; over-amplitude and over-energy candidates are retained as explicit rejection rows. On this frozen fixture, the finite grid contains 30 feasible profiles and 6 rejected profiles. The best worst-case pairwise Gaussian KL score is approximately 0.334498 for a unit-amplitude, 0.5-duration pulse at node 0 or node 2.

This is exact enumeration of a small, preregistered pulse family. It is not the continuous-control optimum and does not imply that the best discrete candidate is globally optimal. A future comparison should use the low-dimensional Pontryagin/optimal-control formulation from the cited model-discrimination literature and report the continuous-vs-grid optimality gap.

## Nuisance-profiled model-family discrimination

The `robust_discrimination` module groups fixed-parameter graph variants into declared structural model families and computes the least-favourable equal-covariance Gaussian KL separation over every cross-family variant pair. Within-family variant pairs are excluded from the structural-family objective and remain visible in the base experiment score.

The frozen negative control deliberately puts a zero-bypass-conductance variant inside the bypass family. This variant reproduces the chain model exactly. Therefore a fixed nominal chain-vs-positive-bypass pulse score can be about 0.334498 while the family-level worst-case separation is zero. Correct disposition: the candidate family sets overlap under their declared parameter bounds; do not claim structural identification.

This is an exact finite-grid nuisance-profile reference. It is not continuous profile likelihood: no likelihood optimizer, interval threshold, posterior, or Pontryagin control solver is implemented. The model-family map and every variant remain explicit and digested. Work is tracked in [HUMAN-NESTED-ROBUST-DISCRIMINATION-001 / #5995](https://github.com/Luminous-Dynamics/symtropy/issues/5995).

### Candidate-visible adequacy and out-of-model-set control

The robust-discrimination module now accepts an explicit candidate-visible observation trace and a preregistered maximum Gaussian residual threshold. It evaluates every finite-grid parameter variant, reports the best residual per family and returns one of four dispositions:
- IN_FAMILY_PREFERRED;
- PARAMETER_NOT_IDENTIFIED;
- IN_FAMILY_UNRESOLVED;
- OUT_OF_MODEL_SET.

The evaluator uses the exact declared sensor order, sample times and full covariance submatrix. A trace with a different sensor order, time schedule, shape or non-finite value rejects before scoring. OUT_OF_MODEL_SET means no registered finite-grid variant passes the declared residual threshold; it does not identify the missing physical cause.

The threshold is an explicit protocol input, not an automatically calibrated chi-square quantile or false-alarm guarantee. These controls are deterministic synthetic software checks only.

### Empirical adequacy-threshold calibration

The `adequacy_calibration` module generates additive zero-mean Gaussian observation noise using the full admitted channel covariance and independent time blocks. It selects an empirical nearest-rank residual threshold on a calibration PRNG seed, then evaluates it on a distinct validation seed. Hidden out-of-family graph truths are evaluated only on the validation stream. The Monte Carlo PRNG is deterministic and explicitly non-cryptographic.

The report includes counts, empirical rates and 95% Wilson intervals for:
- calibration/validation true-family coverage;
- false OUT_OF_MODEL_SET disposition on registered in-family truths;
- wrong-family preference and ambiguous outcomes;
- rejection of declared held-out out-of-family systems.

Registered reference fixture:
- candidate families: chain, bypass conductance 0.5 and bypass-family zero limit;
- selected pulse and node-2 sensor;
- calibration/validation seeds: 0xCA11_BA7E / 0xB01D_5EED;
- 128 replicates per candidate model, target calibration coverage 95%;
- covariance 1e-5 for the threshold-calibration sub-experiment;
- hidden held-out bypass conductance 0.75.

Independent reproduction of this fixed setup yielded 365/384 calibration in-family accepts (95.05%), 358/384 validation accepts (93.23%), 26/384 false OUT_OF_MODEL_SET (6.77%), and 128/128 rejection of the held-out bypass 0.75 (100%). The corresponding validation Wilson 95% intervals were approximately 90.26–95.34% for in-family coverage, 4.66–9.74% for false OUT_OF_MODEL_SET, and 97.09–100% for held-out rejection.

These are deterministic synthetic Monte Carlo results for one declared fixture, not a calibrated real-world OOD detector or a general guarantee. The Gaussian/noise covariance model is assumed correct. The test does not establish robustness to covariance misspecification, non-Gaussian noise, temporal correlation, hidden MNAR missingness or arbitrary omitted model structures.

## Observation-process robustness and full space-time covariance

The noise_robustness module adds a strict dense-covariance reference for
candidate-visible model-family adequacy. The covariance vector order is
time-major, then sensor-major. The generalized Gaussian residual is evaluated
as 0.5 times the squared norm of L^-1 r using a Cholesky factor of the admitted
covariance; the implementation does not explicitly invert the covariance.

The initial covariance constructors include:
- arbitrary finite, symmetric, strictly positive-definite dense matrices;
- AR(1) temporal correlation on equally spaced sample indices;
- exponential temporal correlation on strictly increasing irregular sample times;
- separable time-by-sensor covariance through a Kronecker product.

Covariance admission is fail-closed. Asymmetric, singular, indefinite,
non-finite, dimensionally inconsistent or oversized matrices are rejected.
There is no silent jitter, symmetrization, diagonal fallback, clipping or
pseudoinverse. Missing observations are handled by selecting the matching
principal submatrix; missing values are never replaced with zero.

The full-covariance adequacy evaluator scores the same candidate-visible trace
against every registered finite-grid model variant using the supplied dense
space-time covariance. Its versioned covariance and trace envelopes must agree
on caller-supplied covariance ID, process ID, sampling-policy ID, observation-
operator ID, availability-cutoff ID, ordered sensor IDs and exact sample times.
The covariance ID is present in both the trace and covariance envelope. These IDs are compared but
are not authenticated by this module; an upstream evidence layer must verify
their manifests and event receipts. The original independent-time implementation
remains the explicit iid baseline.

Run the independent AR(1) negative-control lane with:

    cargo run --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --release --bin noise_robustness

The frozen corpus uses four unit-marginal-variance observation times and
AR(1) rho values 0.2, 0.5 and 0.8, with 200,000 synthetic trials per rho.
Its base seed is 20261118 and each rho receives a distinct deterministic stream
seed. The threshold is the iid chi-square(4) 95th percentile. It reports
iid-assumption false rejection and full-covariance Mahalanobis false rejection
separately, with Wilson intervals. The target is to reproduce increasing
iid-assumption miscalibration as temporal dependence strengthens and recover
approximately nominal rejection under the correctly specified full covariance.
Exact observed rates are results of the Rust executable and are not claimed
until that exact branch head is run.

This first slice qualifies only Gaussian covariance handling and one AR(1)
counterexample. It does not yet implement Student-t or contaminated-Gaussian
likelihoods, heteroscedastic models, bias/drift state models, common-cause
dropout processes, MCAR/MAR/MNAR generators, covariance estimation error,
or a general dependent-data conformal guarantee. Unknown noise-family semantics
must remain NOT_ASSESSED / ABSTAIN. The benchmark is synthetic-only and does not
establish any physical sensor or human-subject validity.

### Full-covariance empirical threshold calibration

The `calibrate_model_family_adequacy_full_covariance` reference calibrator
uses the same admitted full time-by-sensor covariance as the candidate-visible
adequacy evaluator. It generates calibration traces with one deterministic
stream, fits a nearest-rank empirical residual threshold, and evaluates
registered candidate truths and held-out hidden out-of-family mathematical
truths with a distinct validation stream.

The report keeps true-family retention, false OUT_OF_MODEL_SET, wrong-family
preference, ambiguous disposition, held-out out-of-family rejection and false
acceptance separate. It also repeats the covariance/process/sampling/operator/
availability identity, ordered sensor IDs and observation schedule used for
calibration. The campaign is bounded to 100,000 total generated traces across
calibration, in-family validation and hidden-truth evaluation. These three
lanes use pairwise-distinct deterministic PRNG seeds.

This is an evaluator-side synthetic harness: it keeps hidden truth out of the
adequacy scorer's input and output, but the calibration function itself runs
in-process and is not a security/process-isolation boundary. The caller must
still put hidden truth and seeds behind the independent evaluator boundary
before using this as an agent-facing benchmark. An empty hidden-truth set or a
hidden truth whose dynamics exactly match a registered candidate is rejected;
this prevents an empty or non-OOD corpus from being reported as a successful
OOD campaign.

Threshold calibration is specific to the complete bound observation process.
Changing the covariance, process family, sample schedule, ordered sensors,
observation operator or availability cutoff requires a new calibration or an
explicitly verified compatibility rule. Unknown and unsupported observation
processes remain NOT_ASSESSED / ABSTAIN; they do not inherit an iid threshold.

### Structured AR(1) × sensor covariance (matrix-free exact path)

Alongside the dense correctness reference, `Ar1KroneckerCovariance` represents
the separable covariance `K_AR1(rho) kron R_sensor` without materializing the
full time-by-sensor matrix. It evaluates the exact Gaussian quadratic form
through AR(1) innovations:

    z_0 = e_0
    z_t = (e_t - rho * e_(t-1)) / sqrt(1 - rho^2), t > 0
    Q   = 0.5 * sum_t z_t^T R_sensor^-1 z_t

The sensor blocks are whitened through the sensor covariance Cholesky factor.
The same representation samples the exact joint Gaussian process through the
AR(1) recurrence. For T observation times and m sensors, scoring is
O(T*m^2) and storage is O(T*m + m^2); the dense reference requires an
O((T*m)^3) factorization and O((T*m)^2) storage. The structured form is only
admitted for its declared separable AR(1) model; non-separable covariance
continues to use the dense correctness reference or a future explicit factor.

The unit corpus compares structured Mahalanobis scores against a materialized
dense Kronecker covariance on small fixtures, and includes a 10,000-component
history that is intentionally too large for dense materialization. The
observation-process receipt now records `DENSE_SPD` or `AR1_KRONECKER` so
performance and calibration claims bind to the actual covariance algorithm.
No Rust performance result is claimed until exact-head execution and matched
benchmarks complete.

## Realized coverage under dependence and delayed feedback

The `coverage_inference` module is a separate method lane from covariance
scoring/calibration. It operates on an event-time-ordered sequence of binary
forecast-coverage outcomes, retaining forecast ID, issue time/step, event
time and outcome availability time/step.

It reports three distinct summaries:
- IID Wilson interval: an explicit independent-Bernoulli comparison, not the
  default uncertainty statement for a dependent sequence;
- Newey-West/Bartlett HAC long-run variance and asymptotic normal interval,
  with caller-specified bandwidth;
- non-circular moving-block bootstrap percentile interval, with explicit
  block length, replicate count and seed.

The HAC and block-bootstrap intervals are diagnostics under their stated
asymptotic/resampling assumptions. They are not distribution-free
finite-sample coverage guarantees. Block length/bandwidth sensitivity and
regime-stratified performance must be reported.

`DelayedAciSchedulerV1` is a bounded fixed-integer-delay recurrence reference.
It records the alpha used at forecast issue, keeps the label pending until
the exact due step, and then updates only the corresponding interleaved
delay lane using alpha_next = clip(alpha + gamma * (target_alpha - miss)).
It rejects premature, late, duplicate and out-of-order settlements; any
overdue pending label blocks new issuance. This is not a general asynchronous
feedback protocol or a claim of universal adaptive-conformal validity.

Run the frozen reference with:

    cargo run --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --release --bin coverage_inference

The runner uses a deterministic 120-outcome block-pattern sequence, HAC
bandwidth 8, 5,000 moving-block bootstrap replicates with seed 20261118,
and a separate fixed-delay ACI sequence with delay three. Exact executable
output is not claimed until the exact Rust branch head is run.

See Symtropy issue #6052 for the preregistered acceptance tests, including
AR(1), GARCH-like and Markov-switching future regimes, delay-to-memory
sensitivity, missing/late labels and historical decision-cutoff controls.

### Covariance identity binding refinement

The candidate-visible `ObservationProcessTraceV1` now repeats the exact
covariance ID as well as process, sampling, operator and availability-cutoff
IDs. The evaluator rejects a trace when any of these identities, ordered
sensor IDs or exact sample times differ from the admitted covariance object.
The identifiers remain caller-supplied references, not authenticated
signatures; evidence-manifest verification remains an upstream authority.

The evaluator also rejects an empty hidden out-of-family set and a hidden
truth whose mathematical dynamics exactly equal a registered candidate.
The equality check ignores human-readable scenario names and undirected edge
ordering, but preserves exact capacities, initial state, horizon and edge
conductances. This avoids reporting a meaningless OOD rejection rate.

## Continuous energy-ball optimal control

The `energy_optimal_control` module lifts experiment design above finite pulse
enumeration by optimizing real-valued forcing amplitudes in a declared
piecewise-constant temporal/actuator basis. For one fixed candidate-model pair,
the response difference is linear in the basis coefficients. Its covariance-
weighted Gram matrix is whitened by the integrated-energy metric; the top
eigenpair gives the exact global optimum over the energy ellipsoid.

The module replays the resulting waveform through the existing spectral
experiment evaluator. If the unconstrained top-eigenvector waveform exceeds
the peak-amplitude limit, the returned waveform is scaled into the feasible
set and marked `PeakConstraintActive`. Its achieved score is a feasible lower
bound, while the unconstrained energy-ball score remains an upper bound; no
global optimum is claimed for that peak-constrained case. A zero Gram matrix
returns `NoDiscriminatingControl`.

The exactness claim is limited to one fixed model pair, fixed sensors, fixed
observation schedule, common per-time sensor covariance, independent noise
blocks across observation times, a finite piecewise-constant basis and an
inactive peak constraint. It is not a Pontryagin solution for arbitrary
waveforms, not a nuisance-profiled T-optimal solver, and not a slew-constrained
or maximin multi-model optimizer.

Run the continuous-vs-grid comparison with:

    cargo run --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --release --bin energy_optimal_control

The runner compares an exact spectral energy-ball optimum over a 9-dimensional
basis (three actuators × three time segments) against a finite pulse grid under
the same chain-vs-bypass plant, sensors, covariance, horizon and energy budget.
The output reports exactness disposition, upper bound, replayed score, energy,
peak and per-segment coefficients. This is a benchmark comparison, not a
published SOTA result; the exact Rust output must be captured from a qualified
exact-head run.
