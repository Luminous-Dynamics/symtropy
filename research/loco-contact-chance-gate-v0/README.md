# LOCO Contact Chance Gate V0

Dependency-free Rust reference for calibrated marginal chance constraints on contact-aware locomotion actions.

Parent: [LOCO-CONTACT-SAFETY-001 / #4010](https://github.com/Luminous-Dynamics/symtropy/issues/4010).

V0 complements:
- [#4008](https://github.com/Luminous-Dynamics/symtropy/pull/4008): action-conditioned contact information and deterministic caller-supplied envelope filtering;
- [#4017](https://github.com/Luminous-Dynamics/symtropy/issues/4017): correlated multi-channel Gaussian contact fusion;
- [#4119](https://github.com/Luminous-Dynamics/symtropy/pull/4119): certificate evidence freshness/deadline gate;
- [#4058/#4061](https://github.com/Luminous-Dynamics/symtropy/issues/4058): finite robust physical viability/recoverability.

This crate adds a **probabilistic one-step risk layer**, not another physical contact solver or viability authority.

## Paradigm: calibrated uncertainty is an admission constraint

A high expected-information action is not automatically safe. Each candidate must first pass five predictive physical bounds:

1. energy upper bound;
2. support-margin lower bound;
3. peak normal force upper bound;
4. impact impulse upper bound;
5. actuator-load fraction upper bound.

Only candidates that pass all five marginal risk bounds can appear in the information-ranked eligible list. Rejected and abstained candidates remain visible in the full report, with explicit reasons. Information gain cannot increase the risk budget.

## Mathematical contract

For a declared family-wise risk budget alpha in (0, 0.5], V0 allocates alpha/5 to each one-sided marginal constraint using integer parts per billion (ppb). The allocated values sum exactly to the family-wise budget; any indivisible remainder is deterministically assigned to the first constraints.

For each constraint j:

    z_j = Phi^-1(1 - alpha_j)

For upper-bounded quantities:

    upper_j = mean_j + z_j * sqrt(variance_j)

For support margin S >= lower_limit:

    lower_support = mean_support - z_support * sqrt(variance_support)

Admit only if every conservative bound satisfies its declared physical limit.

Bonferroni's union bound gives:

    P(any of the five marginal violations) <= sum(alpha_j) = alpha

This does **not** require the five physical constraints to be mutually independent. It does require each supplied marginal tail probability to be valid for the declared model, calibration and operating domain. A covariance calibration or empirical interval-coverage result alone does not automatically prove Gaussian tail correctness.

The quantile uses a dependency-free, versioned Acklam rational approximation, with known standard-normal fixtures. It is not a hard-coded two-sigma multiplier.

## Provenance and abstention

Each candidate binds:
- exact action ID and supplied expected-information value;
- exactly one prediction for each of the five typed constraints;
- units (J, m, N, N*s and dimensionless ratio);
- model, operating-domain and calibration digests;
- prediction timestamp and validity deadline;
- a calibration receipt with evidence digest, verified flag, per-constraint independent validation-unit counts, per-constraint observed one-sided tail-exceedance counts, distribution-family declaration, Gaussian-tail-validation flag and validity interval.

Missing, stale, future, unverified, wrong-domain, wrong-model, wrong-calibration or unsupported-distribution evidence results in **Abstained**, not a numeric safety pass. Heavy-tailed or unvalidated Gaussian tails withdraw the Gaussian chance claim.

Total predictive variance is the quantity used in the bounds. Optional sensor-, process- and model-variance attribution is retained as audit provenance, including a covariance adjustment; the gate does not blindly sum components or discard correlation corrections. The upstream predictor owns the correct total variance calculation.

For each of the five constraints, V0 also requires direct held-out validation of the predicted one-sided bound using independent session/episode units. It computes the exact one-sided 95% Clopper–Pearson upper bound for zero observed tail events:

    p_upper = 1 - 0.05^(1/n)

The upper bound must be no greater than that constraint's Bonferroni allocation. For the 5% family-wise budget (1% per constraint), 299 independent zero-exceedance units are required; 298 are insufficient. V0 deliberately abstains when any tail event is observed or the independent sample is underpowered. This is conservative: a later version may implement the full Clopper–Pearson inversion for nonzero counts, but must not silently count repeated ticks from one rollout as independent units.

The direct exceedance check is distribution-free under the declared independent/exchangeable validation-unit assumption for the exact stratum and event definition. It validates the actual supplied quantile bound empirically; it does not prove all Gaussian modelling assumptions, performance under unobserved distribution shift, or an operational safety case. The calibration receipt flags/digests remain caller-supplied trust inputs; this crate checks consistency and freshness but does not cryptographically authenticate the source artifact.

## Implemented tests

Twenty deterministic fixtures cover:
- standard-normal upper quantiles at multiple tail levels, including 1e-8;
- exact integer Bonferroni allocation;
- safe action admission with all five bounds;
- isolated violation of each of the five constraints;
- high-information unsafe candidate filtered before a safe lower-information candidate is ranked;
- no independence assumption across the physical constraints;
- deterministic zero-variance limits;
- missing/unverified calibration receipts;
- heavy-tailed and Gaussian-tail-unvalidated abstention;
- stale and wrong-domain calibration;
- unit, action-identity and negative-variance rejection;
- monotonicity under stricter family-wise alpha;
- candidate and prediction permutation invariance;
- preservation of variance attribution without blind reaggregation;
- invalid risk budgets/physical limits;
- duplicate action rejection;
- empty prediction set abstention without panic;
- exact 95% zero-event validation sample threshold (298 underpowered, 299 admitted at 1% allocated risk);
- abstention on any observed calibration tail event;
- public quantile invalid-tail input rejection.

Synthetic fixtures validate the declared arithmetic and metadata gate only.

## Commands

    cargo fmt --manifest-path research/loco-contact-chance-gate-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-chance-gate-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-chance-gate-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: `.github/workflows/loco-contact-chance-gate.yml`.

## Competitive comparison

The comparison set should include:
- SafeMind (IEEE Access 2026): probabilistic CBFs, variance-aware barrier constraints and paper-reported 200 Hz quadruped deployment. Its claimed 3–10x fewer safety violations and 10–15% lower energy are study-specific, not directly comparable without matched morphology, terrain, sensors, control period and calibration protocol. https://doi.org/10.1109/ACCESS.2026.3679520
- STEP-CBF+ (Scientific Reports 2026): reduced-order foothold/posture screening with risk, support, actuator, friction and energy constraints. Its reported 4% mean falls, 77.3% stable-step fraction and 13.6 ms selection time are based on 25 paired randomized simulations per terrain; the authors state this is not a whole-body nonlinear invariance proof or hardware validation. https://doi.org/10.1038/s41598-026-72507-w
- Latent-regime MoE quadruped odometry (IEEE RA-L 2026): 14.2% velocity-RMSE reduction on ANYmal, 28.2% drift reduction on Unitree Go1 and >240 Hz on Jetson. This is an estimator baseline, not a chance-constrained safety method. https://doi.org/10.1109/LRA.2026.3728379
- Zhao et al. (Sensors 2026): continuous contact reliability with neural residual Kalman filtering; hardware experiments are indoor walking and turning. https://doi.org/10.3390/s26196041

The intended comparison is **matched** across contact estimator, predictor, calibration data, operating domain, five physical limits, candidate set, control period, hardware/simulator, random seeds, compute and energy boundaries. Compare family-wise violation, per-constraint coverage/calibration, false-safe/false-unsafe/abstention, task outcomes, energy, EIG, latency and deadline misses separately. Do not compare STEP-CBF+'s reduced-order MATLAB decision timing directly to Symtropy Rust or embedded-hardware timings.

## Claim ceiling

A passing synthetic test establishes only the arithmetic and fail-closed input/identity checks. The chance claim is conditional on calibrated Gaussian marginal tails for the exact operating stratum. It is a one-step family-wise risk screen—not a joint probability model, CBF forward-invariance theorem, robust viability certificate, physical robot safety guarantee or SOTA result. Simulator and hardware qualification remain separate.