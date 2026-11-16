# GeoBio reference kernel v0.2

Standalone, dependency-free Rust reference code for the GeoBio Foundry research programme.

## Run locally

From the Symtropy repository root:

```sh
cargo test --manifest-path research/geobio-foundry/kernel/Cargo.toml
```

The package has its own Cargo workspace and is intentionally not a member of Symtropy's production workspace. No production crate depends on this code.

## Implemented

- Typed process, material-flow, elemental-composition, origin and evidence records.
- Explicit kg and J units in field names.
- Input, product and waste flow validation.
- Composition completeness checks: complete compositions must reconcile to total flow mass; incomplete compositions cannot claim that unlisted elements are zero.
- Elemental mass-balance residuals.
- Uncertainty propagation for independent measurements and explicitly named fully correlated groups.
- Process-level status: within tolerance, outside tolerance or unquantified.
- Independent-witness requirement for independently reproduced and qualified claims.
- Resource intensity per kg of declared primary product, with first-order uncertainty propagation for feedstock-origin ratios and gross/recovered energy kept separate.
- Numerical overflow checks, negative/non-finite input rejection, complete-composition reconciliation and origin-intensity ratio overflow rejection.
- Single-species ideal-gas pressure/known-mass-transfer reference module with explicit limitations and observation comparison.
- Non-uniform one-dimensional subterranean profile summary with explicit unobserved intervals, coverage fraction, length-weighted width/height and standard uncertainty, observed extrema and profile variability.
- Typed cross-field Subterra site-evidence validator rejecting logically inconsistent connectivity/access/survey/engineering/preservation states. Non-Earth sampling/biological activity requires a named authorization-case reference; Earth records may mark planetary protection not applicable but still require a basis reference for an authorized activity.
- Evidence-preserving cave network graph: candidate/geophysically supported links remain separate from directly observed topology; robot routes use only directly observed edges with demonstrated traversability; directed edges allow inbound and retreat routes to differ.

## Important limitations

- Correlation groups assume perfect correlation within a group and independence between groups. A general covariance matrix is not implemented.
- Ratio uncertainty uses first-order derivatives; this is not a full covariance estimator and does not model non-linear uncertainty or model discrepancy.
- Detection-limit/censored measurements, elemental ontology mapping, isotope effects, process heat/exergy closure, JSON deserialization and lifecycle cost are not implemented.
- The profile kernel uses a piecewise-constant representative cross-section per interval. It is not a 3D surface reconstruction, connectivity model, roof-stability predictor, geomechanics solver or safe-clearance certification.
- The kernel does not model a live biological process, geomechanics, pressure vessels, life support or any physical actuator.
- A synthetic fixture and unit tests do not establish laboratory, field, orbital or flight performance.

## Fixture

`../fixtures/water-electrolysis-reference.json` is a synthetic record for schema validation and stoichiometric illustration only. Its energy figure is deliberately illustrative and must not be cited as an engineering performance datum.
