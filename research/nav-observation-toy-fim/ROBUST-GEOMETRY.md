# Sensitivity-aware geometry selection: prototype v0.1

Status: additive research slice with a targeted Rust test workflow. Parent: correlation-aware active-sensing PR #6494.

## Paradigm shift

Nominal information is not the same as deployable information. A sensor layout with the largest nominal Fisher determinant can be fragile to small bearing or calibration perturbations. The new selector:

1. computes the nominal exact-subset optimum for a chosen D/A/E criterion;
2. retains only candidates meeting a declared nominal-information floor, such as 90%;
3. evaluates each retained candidate over a frozen finite set of bearing and private-noise perturbation scenarios;
4. maximizes the minimum perturbed-to-nominal information ratio;
5. breaks ties by nominal quality, cost and deterministic IDs.

The perturbation set is part of the contract and must be frozen before evaluation. This is not a continuous worst-case certificate.

## Frozen known-answer example

Four candidate actions with cost budget two:
- Fragile pair: east/north at sigma 0.20 m, nominal determinant 625; adverse angular errors of plus/minus 0.35 rad.
- Stable pair: 45°/135° at sigma 0.205 m, nominal determinant about 566, retaining just over 90% of nominal maximum; angular perturbations of plus/minus 0.01 rad.

The nominal D-optimal selector chooses the fragile pair. A sensitivity-aware selector with a 90% retention floor should choose the stable pair, with worst-case retention above 0.99 under the two frozen scenarios. These are analytical/test expectations, not measured deployment performance.

## Direct comparator

The IEEE paper's three-sensor numerical example reports around 90% of maximum nominal information while reducing sensitivity and information variance under deployment perturbations. Reproduce its exact setup where available; then compare against this finite perturbation-envelope oracle with matched candidate locations, measurement modalities and perturbation distributions. Source: https://ieeexplore.ieee.org/abstract/document/11557156.

CAAF is an adjacent data-driven sensor-placement baseline. Its 2026 paper clusters correlated candidate locations before feature attribution to reduce redundant candidates; compare at identical candidate sets and prediction targets. Source: https://www.nature.com/articles/s44488-026-00011-1. Its licence is CC BY-NC-ND 4.0; use it as a citation/reference only unless rights for adapted material are separately resolved.

## Claim boundaries

- Only the named finite perturbation scenarios are covered.
- No distributional or adversarial guarantee exists for perturbations outside that envelope.
- Perturbation magnitudes must be calibrated or preregistered; they must not be tuned on the held-out result.
- FIM remains local linearized information, not posterior covariance, protection level or integrity evidence.
- The implementation consumes upstream common-mode/provenance facts and does not own source independence.
- Five tests are authored; the exact-head targeted workflow must pass before considering them executed evidence.
