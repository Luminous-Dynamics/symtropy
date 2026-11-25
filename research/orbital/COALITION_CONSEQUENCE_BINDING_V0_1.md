# Source-Bound Coalition Consequence Binding v0.1

**Status:** synthetic research adapter; exact-head CI pending. **Claim ceiling:** finite structural checks only. No physical qualification or maneuver authority.

This layer composes the #7161 joint-world/loss bridge and #7220 independent loss-model-universe checker. It enumerates every resource-feasible portfolio (up to 16 actions), recomputes canonical portfolio IDs, requires an explicit direct loss for every portfolio in every admitted joint scenario, and binds each portfolio to a post-action full-catalog risk snapshot.

It deliberately rejects additive coalition scoring: in general, L(A+B,s) is not L(A,s)+L(B,s). It reports the signed lower bound min_s[L(no-action,s)-L(A,s)] and preserves negative deltas. Nominal mean/CVaR require a separate source-pinned probability distribution; Wasserstein DRO-CVaR remains unavailable until an ambiguity-set receipt is verified.

The frozen fixture has no-action, A, B and A+B across four synthetic loss-model scenarios. The minimum avoided-consequence values are A=+5, B=-3, A+B=-10. These are arithmetic controls, not observed orbital performance.

The research target is to compare against CARA analytic and Monte Carlo Pc, high-fidelity brute-force Monte Carlo, orbital importance sampling (DOI 10.1016/j.asr.2024.12.074), cross-entropy Gaussian/GMM (DOI 10.2514/1.G009204), and safe-margin screening (DOI 10.2514/1.G008922). All accuracy, coverage and compute targets are prospective. Benchmark at matched dynamics, covariance, event geometry, horizon, hardware and stopping rule.

The adapter does not authenticate issuers, prove model-universe completeness, calibrate loss models, recompute Pc/propagation, validate full-catalog screening itself, or grant authority. See [PR #7501](https://github.com/Luminous-Dynamics/symtropy/pull/7501) for implementation and exact claim boundaries.
