# Time-aware active sensing under state evolution v0.1

**Status:** isolated offline exact horizon-two reference for NAV-TIME-AWARE-VOI-001 (#7282). It complements, rather than replaces, the temporal validity contract in #7139.

## Paradigm shift

The observation can be correct at capture and wrong as a representation of the current service state. The planner should first condition on the historical measurement, then propagate the posterior through the state-transition model for the capture-to-use interval.

For each action:
1. apply state-conditional observation likelihood at capture time;
2. update the capture-epoch state posterior;
3. propagate that posterior through the action-specific transition kernel;
4. assess the decision loss at the use epoch;
5. use that current belief to choose any second action.

The no-observation outcome follows the same sequence. State-dependent dropout can update the capture posterior before time evolution.

## Implemented contract

The Rust module navigation_time_aware_policy_tree.rs provides:
- binary service state (inside/outside);
- three capture-epoch outcomes (no observation, observed-zero, observed-one);
- state-conditional likelihood rows in integer PPM;
- action-specific two-state Markov transition from capture to use epoch;
- evidence age bound by declared action latency;
- nonzero commitments for outcome and transition models;
- exact capture-epoch and use-epoch posterior ratios;
- exact finite horizon-two branch-specific planner and greedy/static comparator;
- hard latency/energy/bandwidth ceilings, no action reuse, deterministic ordering, checked u128 arithmetic, abstention and non-authority marker.

The PPM scale is deliberate: two action stages each apply observation and transition factors, so the common two-stage risk denominator is \(10^{30}\), and bounded integer losses keep all risk numerators in u128.

The supplied transition kernel is not calibrated or authenticated by this module. The caller must bind it to the exact evidence-age interval, environment/model revision and collection evidence. The current v0.1 propagates the capture-epoch posterior to the use epoch; it does not add a second, separate delivery delay after that use epoch. Future work should split acquisition, physical sampling, transport/queue, and decision epochs explicitly.

## Synthetic counterexample

The test fixture gives one action a perfect capture-epoch measurement but a 50% inside-to-outside transition before its result is usable. A second action has 10% observation error but zero evidence age/identity transition.

- A static-state planner prefers the perfect capture measurement.
- The time-aware planner prefers the noisier, fresher evidence because it has lower terminal loss at the use epoch.
- The test also verifies that the first action's capture posterior may be certain while the use-epoch posterior is only 50/50.

This is a constructed Markov example, not a physical motion or sensor result.

## Tests authored

Eight tests cover:
- static-vs-time-aware action ranking reversal;
- capture posterior versus use posterior;
- state-dependent no-observation followed by propagation;
- zero-age identity transition;
- evidence age not exceeding action latency;
- malformed likelihoods/transitions/missing commitments;
- candidate order invariance, resource limits and no action reuse;
- explicit abstention.

Exact-head CI is the qualification gate.

## Comparator protocol

Freeze identical priors, capture-epoch likelihoods, action sets, transition kernels, resource budgets and loss matrix. Compare:
- static-state exact policy tree (#7250);
- stale last-measurement/no propagation;
- time-aware exact policy tree (this module);
- periodic/random sensing;
- delay-compensated kinematic extrapolation where motion data is available;
- privileged future-state oracle as evaluator-only upper bound.

Report capture-epoch residual and use-epoch loss separately. Also report evidence age, transition-kernel source/coverage, false-safe/false-degraded exposure, abstention, acquisition yield, resource use and regret to this exact oracle. A measurement-age correction that improves P50 while worsening false-safe exposure must not be treated as a win.

## Limitations

This v0.1 has a binary state, one Markov transition per action, fixed supplied transition kernels and a two-action horizon. It does not model continuous pose, arbitrary-duration transition powers, state/model regime changes, transport delay separate from the capture-to-use interval, non-Gaussian continuous observations, or physical integrity guarantees. It does not replace clock synchronization, time-basis validation, OOSM/fixed-lag replay, or attempt-ledger reconciliation.

## Research anchors

- Modaberi & Far (2026), delay-compensated UWB localization for dynamic agents, DOI 10.1016/j.eswa.2026.132248: https://doi.org/10.1016/j.eswa.2026.132248
- NAV-TIME-VALIDITY-001 #7139: https://github.com/Luminous-Dynamics/symtropy/issues/7139
- NAV-SEQUENTIAL-VOI-001 #6689: https://github.com/Luminous-Dynamics/symtropy/issues/6689
- ISO/IEC 21134:2026 integrated indoor localization and tracking benchmark framework: https://www.iso.org/standard/86651.html
