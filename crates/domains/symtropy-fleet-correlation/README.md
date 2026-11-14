# Symtropy Fleet Correlation

Empirical cross-site dependence analysis for synchronized data-centre PCC or other declared electrical-boundary traces.

## Paradigm

Do not treat a fleet as independent sites until the measured joint time series supports that approximation. Cross-site covariance and lagged dependence can amplify aggregate variability, ramps and threshold exceedance even when every site has the same marginal power distribution.

## v0.1 capabilities

- exact shared start timestamp, interval duration, sample count and alignment-model checks;
- explicit site-level clock uncertainty and maximum pairwise clock uncertainty gate;
- per-site mean/variance and full fleet aggregate variance;
- independent-site variance sum versus observed portfolio variance, with covariance-excess and variance-identity receipt;
- pairwise contemporaneous correlation and strongest positive/negative lag correlation over a caller-bounded interval window;
- portfolio peak, minimum, declared threshold exceedance fraction and absolute ramp distribution;
- fail-closed duplicate IDs, non-finite readings, incompatible axes, unbounded lag overlap and clock errors.

The positive-lag convention is corr(A[t], B[t + lag]); positive lag means B's pattern follows A. Signed power is preserved. This is trace analysis, not causal attribution, a calibrated probability model, reserve sizing or a grid stability study.

## Run

```sh
cargo test -p symtropy-fleet-correlation
cargo clippy -p symtropy-fleet-correlation --all-targets -- -D warnings
```

All current tests use synthetic known-truth vectors. No measured fleet dataset or field claim is included.
