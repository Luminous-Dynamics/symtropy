# Magnetic field-map comparison reference

A zero-dependency Rust research crate for deterministic, exact-grid comparison of
predicted and reference magnetic vector-field maps.

## Run locally

    cargo test --manifest-path research/magnetics/field-map-metrics/Cargo.toml
    cargo fmt --manifest-path research/magnetics/field-map-metrics/Cargo.toml -- --check

The crate declares its own workspace so it does not join the production Symtropy
workspace or modify its lockfile.

## Contract

Each map has a coordinate-frame ID, a frozen grid ID, and a non-empty ordered
list of unique sample IDs, positions in metres, and B vectors in tesla. The
comparator rejects mismatched frame/grid/order/IDs/positions and non-finite
inputs. It never interpolates or silently drops samples.

Metrics are intentionally independent:
- component MAE, vector MAE and vector RMSE;
- reference RMS and normalized vector RMSE (undefined when reference RMS is zero);
- maximum and nearest-rank p95 vector error, including the worst sample ID;
- pointwise absolute-tolerance coverage and all-points-within-tolerance;
- direction-angle and reversal metrics above an explicit field-magnitude floor.

The tolerance result applies only to the supplied grid. It is not a solver,
material, magnet, motor, actuator or product qualification.

## Important limitations

Metrics are unweighted over the supplied samples. Freeze the same grid and
sampling definition for every comparison. A nonuniform grid must not be
misrepresented as a volume-weighted error. Surface and volume maps are distinct
benchmark strata.

This version does not estimate measurement or prediction uncertainty, perform
covariance-aware coverage, interpolate maps, or calculate force/torque. It does
not solve magnetostatics and does not establish physical accuracy. Uncertainty
coverage needs an explicit covariance and calibration contract in a later
increment; it must not be inferred or fabricated from a residual score.

Synthetic known answers validate arithmetic only. They are not physical field
measurements.
