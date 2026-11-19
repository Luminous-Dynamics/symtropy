# Forecast Information and Noise Field Verification

A separate executable for Symtropy #6220, implementing the 2026 forecast information/noise decomposition on fixed, normalized cell-area verification support.

## Command

```sh
cargo run --manifest-path tools/marine-forecast-scorecard/Cargo.toml --bin forecast-information-noise -- audit-information-noise tools/marine-forecast-scorecard/fixtures/synthetic-information-noise-0001.json
```

## Computation

For forecast (f_i), verifying field (y_i), climatology (c_i), and normalized area weight (w_i), first form forecast and verifying anomalies, then remove their weighted regional means. The engine reports:

- SDAF and SDAV: forecast and verifying field activity
- ACC: area-weighted anomaly correlation
- FI: signed forecast information (p / SDAV^2)
- IE: absolute information error (|1-FI| SDAV)
- NE: orthogonal noise error
- Debiased area-weighted RMSE / STDE and its decomposition residual

It checks (STDE^2 = IE^2 + NE^2) within a fixed numerical tolerance. FI is signed and not clipped to [0,1]. Degenerate verifying-field variance is invalid, not converted into a score of zero. Field/cycle results are preserved separately; aggregating FI first can conceal per-cycle overshoot and undershoot.

## Smoothing negative control

The synthetic fixture has one matched raw/damped pair. Damping suppresses orthogonal forecast noise enough to reduce RMSE, but also reduces FI and increases IE. The validator demonstrates this as a benchmark warning flag. It is a known-answer software case, not a claim about a real forecast model.

## Evidence constraints

- First availability must be after issue time and no later than decision cutoff; forecast valid and verifying times must match and follow cutoff.
- Verification support, source digest, climatology digest, grid, transform-independent identity and normalized area weights are frozen in the record.
- A raw/damped comparison must have identical variable, lead, fold, episode, voyage, weather event, verifying source, support, climatology, cell identity, weights, truth and climatology values.
- Use area weighting for field verification; any route-exposure weighting is a separate decision-value metric.
- Point buoy checks remain spatially limited; they are not a continuous route-wide truth field.
- The tool calculates field diagnostics only. It does not decode GRIB/NetCDF, interpolate fields, validate spatial representativeness, authenticate publisher provenance or authorize navigation.
