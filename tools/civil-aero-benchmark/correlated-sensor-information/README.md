# AER-01 correlated sensor information oracle v0.1

**Tracking:** [AERO-SENSOR-INFO-001 / #4258](https://github.com/Luminous-Dynamics/symtropy/issues/4258)  
**Parent:** [#4242 structural observability co-design](https://github.com/Luminous-Dynamics/symtropy/issues/4242)  
**Related implementation:** [#4244 synthetic structural observability oracle](https://github.com/Luminous-Dynamics/symtropy/pull/4244)

## Purpose

This standalone Rust crate and independent Python oracle extend the two-target sensor-placement benchmark to include:
- admitted per-channel observation gains and operator identity;
- full positive-definite cross-channel noise covariance;
- D-optimal and E-optimal placement under the full covariance;
- worst-case D-optimal and E-optimal placement under individual sensor dropout and shared common-cause group loss;
- explicit missing operator, stale calibration and invalid covariance rejection.

It is a synthetic, real-valued, quasi-static Fisher-information benchmark only. It is not a structural solver, real aircraft sensor layout, SHM classifier, damage-detection method, inspection POD, airworthiness finding or certification tool.

## Mathematical model

For a declared local two-parameter state:

\`y = A H \\theta + \\epsilon,\qquad \\epsilon\\sim\\mathcal{N}(0,R)\`

\`F = (AH)^T R^{-1}(AH)\`

- \`H\`: synthetic structural sensitivity rows.
- \`A\`: per-channel real scalar observation gain with an explicit admitted operator digest.
- \`R\`: full channel-noise covariance, ordered by the exact sensor registry.
- \`F\`: local Fisher information for the declared parameter scaling.

The covariance is not silently diagonalized. A non-symmetric, singular or indefinite covariance rejects. Each selected covariance principal submatrix is factored with Cholesky and solved by forward/back substitution; the implementation does not explicitly form R inverse, add diagonal jitter, or use a pseudoinverse to manufacture observability. Rank deficiency remains explicit.

The v0.1 model uses an explicitly quasi-static real observation gain. Dynamic frequency response, phase, bandwidth and sensor transfer functions remain outside this crate and are owned by #4236/#3071. A dynamic operator cannot be promoted through this real static gain.

## Frozen synthetic result

Five synthetic candidate sensors are given. S1 and S2 share a declared power group and have noise correlation 0.95; S3 and S4 share an interrogator group and have noise correlation 0.20; S5 is independent. Every channel has a synthetic, digest-bound real observation gain and calibration marker.

The frozen result demonstrates an optimizer-ranking change:

| Selection | Diagonal-noise baseline | Full correlated covariance |
|---|---|---|
| Two-sensor E-optimal | S1 + S2; minimum eigenvalue 2.25 | S3 + S4; minimum eigenvalue 1.6666667 |
| Two-sensor D-optimal | Not the primary diagonal comparison | S1 + S2; determinant 51.9230769 |
| Four-sensor robust D-optimal | Not evaluated | S1 + S2 + S3 + S4; worst-case log determinant 1.4271164 |
| Four-sensor robust E-optimal | Not evaluated | S1 + S2 + S3 + S4; worst-case minimum eigenvalue 1.1538462 |

The four-sensor robust D-optimal and E-optimal results both select S1 + S2 + S3 + S4 in this specific fixture. Their objective functions differ and are independently optimized: worst-case log determinant for D-optimality, and worst-case minimum eigenvalue for E-optimality. They survive six declared scenarios:
- dropping S1, S2, S3 or S4 individually;
- losing the shared POWER_A group (S1+S2);
- losing the shared INTERROGATOR_B group (S3+S4).

The weakest remaining information is approximately 1.1538462 and the worst-case log determinant is approximately 1.4271164. A scenario that leaves fewer than two independent target directions receives zero robust score and makes robust D-optimality infeasible. The result is specific to this small fixture and its declared parameter scaling.

## Verification design

The Rust executable:
1. validates sensor IDs/order, finite sensitivities, positive scalar gains, operator SHA-256 syntax, calibration status, and exact registry/covariance order;
2. checks covariance symmetry and strict positive definiteness through Cholesky;
3. forms principal covariance submatrices, factors each by Cholesky and solves the required systems without explicitly forming an inverse or adding jitter;
4. computes \`(AH)^T R^{-1}(AH)\`, eigenvalues, rank and determinant;
5. enumerates nominal D-opt/E-opt and robust maximin D-opt/E-opt under all declared single-sensor and common-cause dropout scenarios;
6. emits \`aircraft_feasibility=NOT_ASSESSED\`, \`dynamic_observation_status=NOT_ASSESSED\`, and \`physical_calibration_status=NOT_ASSESSED\`.

The separate Python standard-library verifier independently repeats Cholesky factorization, triangular solves, Fisher-information calculations and exhaustive search, checks all four frozen selection answers, and optionally compares the Rust JSON output. It does not import the Rust implementation.


## Joint six-channel covariance and failure-domain regression

A second frozen fixture, data/instrument-correlated-failure-v0.1.json, extends the same operator-aware sensor record and covariance contract to six channels, four selected channels, 15 exact candidate subsets and seven failure scenarios per subset's selected set (four individual selected-channel dropouts and the common-cause groups represented in that selection).

The six synthetic channels retain:
- per-channel sensitivity and positive static operator gain;
- explicit ADMITTED operator status and SHA-256-shaped operator identity;
- VALID calibration status;
- declared common-cause group membership;
- synthetic deployment cost, mass and power;
- a full positive-definite covariance matrix ordered by the channel registry.

Its known answers are:
- nominal full-covariance D-optimum: S_A + S_C + S_E + S_F;
- nominal full-covariance E-optimum: S_A + S_B + S_E + S_F;
- diagonal-noise E-optimum: S_A + S_B + S_C + S_D, minimum eigenvalue 10.0;
- robust maximin D- and E-optimum: S_A + S_C + S_E + S_F;
- robust worst-case log determinant: approximately 3.22529639267;
- robust worst-case minimum eigenvalue: approximately 2.90608817764.

The nominal E-optimum is deliberately non-robust: losing common-cause group PWR_Z removes S_E and S_F, leaving S_A and S_B with only one independent target direction. The robust maximin selection retains full rank across all selected sensor dropouts and selected common-cause group failures. This makes the second fixture a useful cross-check that the sensor-operator identity, calibration and cost fields are preserved in the larger covariance/failure test, rather than testing an isolated Fisher matrix.

The executable emits a JSON bundle containing both the original five-channel baseline and the six-channel regression. The independent standard-library Python implementation recomputes both fixtures from their exact bytes, and CI retains both source fixtures and the combined result. All second-fixture values are synthetic; no physical sensor placement or aircraft claim follows.



## Minimum-cost robust design under an information floor

The fixtures also declare a minimum acceptable worst-case E-optimal information value. A separate exact enumeration ranges over every deployment size that can survive the declared single-sensor dropout budget and searches for the **lowest deployment-cost design that remains full-rank and meets the information floor in every individual-sensor and common-cause failure scenario**.

The comparison is intentionally different from maximin D/E:
- maximin D/E asks how much information a fixed-size sensor set can maximize;
- minimum-cost robust design asks how little instrumentation can meet a predeclared information requirement.

For the five-channel baseline, the floor is lambda_min >= 1.0. The minimum-cost robust set is S1 + S2 + S3 + S4: deployment cost 4.0 units, 50 g, 500 mW, worst-case lambda_min approximately 1.1538462.

For the six-channel joint fixture, the floor is lambda_min >= 1.5. The minimum-cost robust set is S_A + S_C + S_E: deployment cost 4.2 units, 48 g, 12 mW, worst-case lambda_min approximately 1.5036392. The four-channel maximin E-optimal set S_A + S_C + S_E + S_F has a higher worst-case value (approximately 2.9060882) but costs 6.2 units, weighs 73 g and draws 19 mW. Under the declared synthetic costs, the three-channel result saves approximately 32.3% of deployment cost while still clearing the stated information floor under every scenario.

This is not a universal optimum or aircraft design recommendation. The threshold is expressed in the fixture's normalized parameter coordinates; parameter scaling and the choice of acceptable information floor are part of the engineering requirement and must be justified from the actual use case. If no subset meets the threshold, the selector returns no design rather than quietly weakening the floor.


## Hostile controls

The Rust tests and Python verifier reject:
- asymmetric covariance;
- singular/non-positive-definite covariance;
- missing/unadmitted observation operator (no identity fallback);
- invalid operator digest;
- expired calibration;
- attempted admission of a dynamic transfer function into the real quasi-static oracle;
- deletion of the correlation terms while retaining the frozen ranking claim.

The independent replay confirms every reported robust scenario retains rank two and checks the worst-case D-optimal log determinant as well as E-optimal minimum eigenvalue.

## Industry and benchmark positioning

The approach extends familiar sensor-placement baselines—uniform/engineering judgement, effective independence, QR-pivot, D-optimal Fisher information, E-optimal worst-direction information and robust placement—by requiring the same *admitted measurement operator and full covariance* for each competitor.

This is not a claim of algorithmic superiority over NASA/industry SHM practice or SAE ARP6461A. The next comparisons should use equal candidate sets, sensor constraints, state parameterization, operator assumptions, covariance estimates, failure scenarios and search budgets. Model ranking, calibration uncertainty, covariance-estimation uncertainty, sensor dropout and common-cause failures should be reported separately.

## Run locally

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/correlated-sensor-information/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civil-aero-benchmark/correlated-sensor-information/Cargo.toml
cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/correlated-sensor-information/Cargo.toml > /tmp/correlated-sensor-information.json
python3 tools/civil-aero-benchmark/correlated-sensor-information/verify_independent.py --rust-output /tmp/correlated-sensor-information.json
~~~

## Claim ceiling

This work verifies arithmetic and fail-closed behavior for one synthetic two-parameter quasi-static model. It does not validate instrument calibration, real covariance estimation, physical aircraft state observability, damage detection, NDE probability of detection, fatigue, structural integrity, flutter, airworthiness or permission to install/test/fly.
