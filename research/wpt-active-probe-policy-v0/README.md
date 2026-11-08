# WPT adaptive probe-policy v0

**Status:** exact robust decision-tree selection over a frozen synthetic action library. This is not a waveform generator, estimator, inverter controller, EM solver or safety interlock.

## Ownership

- #3512 provides qualified coupling/load estimator methods and their excitation characterization.
- #3640 owns primary-side self-sensing and the separation of coupling evidence from identity/authorization.
- #4115 provides finite pose hypotheses and upstream outcome/alias semantics.
- #4197 owns spatial field/configuration co-design.
- This crate chooses an adaptive order of prequalified probe actions; it does not produce their physical outcome maps or estimate pose from raw waveforms.

## Paradigm

Fixed multi-sine or sequential probing spends the same excitation budget independent of the remaining ambiguity. This policy chooses the next action based on the current outcome branch and stops as soon as one pose hypothesis remains. Its objective is exact and robust:

1. First compute the globally minimum feasible worst-case cumulative probe energy.
2. Then minimize worst-case cumulative latency subject to that globally minimal energy bound.
3. The action-count limit is a hard feasibility constraint; stable action ID resolves remaining ties.

No Bayesian prior is required. Each action maps every finite pose hypothesis to a **set** of possible quantized outcome classes produced upstream. For an observed class, the next hypothesis set is the union of every pose whose support contains that class. This handles bounded measurement ambiguity without assuming that a classifier returns one correct label. Every possible outcome branch must strictly reduce the current hypothesis set and ultimately terminate in one pose within the hard cumulative budgets. A branch that can leave all current hypotheses alive makes the action unusable at that node.

## Hard constraints

Each action is typed PASSIVE_OBSERVE or ACTIVE_EXCITATION. An active excitation must have positive incremental energy cost; passive observation may have zero incremental energy cost only when reusing already-running telemetry (any actual sensor draw is charged), and it always has positive duration. Each action is excluded before planning if it exceeds maximum ripple, falls below minimum ZVS margin, or exceeds maximum service disturbance. The complete policy must fit the declared worst-case energy, latency and action-count budgets. If no such decision tree exists, the evaluator abstains.

## Synthetic fixture

The parser represents each action row's outcome field as one comma-separated support set per pose; use `0|1,1|2,3` to mean pose 0 can yield bins 0 or 1, pose 1 can yield 1 or 2, and pose 2 can yield only 3. Singleton values remain ordinary integers. Duplicate bins and empty supports are rejected.

The fixture contains four pose hypotheses and eight invented actions, including one `PASSIVE_OBSERVE` action and seven `ACTIVE_EXCITATION` actions. A passive action may have zero incremental excitation-energy cost, but still consumes acquisition time; active excitation is required to carry positive energy cost. The test's passive telemetry sample is a synthetic zero-incremental-energy example, not a claim that physical sensing is free.

The CLI now prints the typed action library (PASSIVE_OBSERVE vs ACTIVE_EXCITATION), including incremental energy, duration, ripple, ZVS margin and service disturbance for every candidate. The admissible adaptive policy is:

- First apply `passive_telemetry_split` (0 synthetic incremental µJ, 2 synthetic µs).
- On outcome 0, apply `left_discriminator` (12 synthetic µJ, 4 synthetic µs).
- On outcome 1, apply `right_discriminator` (15 synthetic µJ, 5 synthetic µs).

Worst-case adaptive energy is 15 synthetic µJ and latency is 7 synthetic µs. The minimum-energy fixed sequence under the same three-action ceiling needs the passive sample plus both discriminators (27 synthetic µJ, 11 synthetic µs), exceeding both the case's 25 µJ and 10 µs budgets. The adaptive policy uses only the relevant active discriminator on each branch.

Three unique-pose actions are deliberately unsafe and must be excluded: ripple violation, insufficient ZVS margin and excessive service disturbance.

The test corpus also includes a set-valued noisy case where the first probe leaves an overlapping pair of pose hypotheses for one observation bin; a second probe resolves them. The parser rejects empty support sets and duplicate bins. This demonstrates robust bounded-support hypothesis propagation, not a probability distribution or calibrated stochastic noise model.

These values and output supports are entirely synthetic. Synthetic µJ/µs values are not measured physical quantities and do not imply a realizable excitation waveform.

## Exactness boundary

The recursive solver is exact for the supplied finite hypothesis/action set and the stated two-pass minimax objective. It is robust to the declared set-valued support relation: every possible quantized observation must lead to a strictly smaller hypothesis set before terminal resolution. It is bounded to 16 hypotheses and 16 candidate actions. It does not establish global continuous observability, physical outcome probabilities, waveform realizability, switching stress, thermal behavior or measurement calibration.

## Validation

The Rust test suite includes 19 adversarial/known-answer tests. In addition to targeted cases, the independent exhaustive tree enumerator constructs the feasible energy/latency Pareto set for small cases and verifies that the production DP returns the globally minimum-energy, then minimum-latency pair.

## Run

    cargo test --locked --manifest-path tools/wpt-active-probe-policy/Cargo.toml
    cargo run --locked --manifest-path tools/wpt-active-probe-policy/Cargo.toml -- research/wpt-active-probe-policy-v0/synthetic-actions.tsv

## Claim ceiling

A passing result proves only that the synthetic set-valued decision tree resolves the declared finite hypotheses within the fixture constraints. Digest references are syntax-checked, not externally content-verified. It does not validate upstream pose or estimator artifacts, establish a safe excitation sequence, authorize energization or flight, or demonstrate advantage over a physical state-of-the-art WPT system.
