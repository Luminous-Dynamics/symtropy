# Exact inter-pass geometry measurement oracle v0.1

Tracking: [SHIPYARD-GEOMETRY-MEASUREMENT-ORACLE-001 #4895](https://github.com/Luminous-Dynamics/symtropy/issues/4895)

## Why this is a separate qualification layer

The inter-pass planner (#4649) can check that a scan is registered and has a declared uncertainty, but it should not be the authority that validates the measurement from which its residual-area state was derived. This oracle independently answers whether a pair of synthetic 2D profiles is sufficiently identified, registered, supported and uncertainty-bounded to produce a geometry comparison.

It reuses—not replaces—the adjacent owners:
- #4872 and #4770: scan registration, calibration, coordinate-frame and transform assurance.
- #4947: multi-rate event-time and temporal correspondence.
- #4643: rights-aware BAM scan and source admission.
- #4649: downstream pass recommendation.
- #4614: physical model VVUQ.
- #4633: fixture/restraint/release and final free-state distortion.

## SOTA and standards positioning (13 November 2026)

The closest adaptive planning comparator is Yu et al. (published 15 September 2026): 4,434 beads across 270 T-joint specimens, inter-pass 3D profiling and residual cross-sectional volume reallocation; the paper reports nine additional validation specimens with 100% UT compliance in its stated structural CJP application. This is an adaptive planner and quality outcome comparator, not a directly matched metrology oracle or a directly comparable BAM 1G T-GMAW case. [Source](https://doi.org/10.1007/s00170-026-19085-3).

Wang et al. (October 2026) report online welding-path RMSE ≤0.1334 mm and torch-posture RMSE ≤0.8064° for online path/posture planning over unknown spatial geometry. These are relevant trajectory targets, but not profile-area, metrology-coverage or uncertainty-bound metrics. [Source](https://doi.org/10.1016/j.rcim.2026.103315).

ISO/TR 23247-101:2026 was published in May 2026 and describes a manufacturing digital-twin use case for robotic multilayer/multipass GMAW. ISO/AWI 23247-8 was approved and registered on 23 October 2026 for manufacturing digital-twin VVUQ, but remains under development. This oracle aligns with their concepts; no standards conformance claim is made:
- https://www.iso.org/standard/93278.html
- https://www.iso.org/standard/94858.html

A closed-loop robotic-welding survey indexed under DOI 10.1016/j.rcim.2026.103375 currently lists a February 2027 issue date. It is not used as a published-as-of-13-Nov-2026 benchmark source in this protocol.

## V0.1 measurement contract

The Rust crate is a nested, dependency-free workspace. It accepts **synthetic-only** profile pairs. Both profiles must share exact WID, joint revision, transition identity, coordinate frame and identical x stations. Each profile is tied to a lowercase SHA-256 identifier; the registration receipt must bind both scan hashes and the target frame.

The registration receipt is checked for:
- its own SHA-256 identifier;
- exact reference and observed scan-digest binding;
- source and target frame identity;
- calibration and algorithm revision identity;
- maximum residual within both the receipt-declared and request-declared residual limits.

The profile pair must:
- have at least two strictly increasing stations;
- span the region of interest exactly (no extrapolation);
- be captured in the correct temporal order and be fresh at decision time;
- provide non-zero, bounded point uncertainty wherever a z measurement exists;
- meet minimum covered-length permille;
- keep the longest contiguous unsupported gap within the declared limit.

A missing z sample is unsupported. A missing/zero uncertainty for a present z measurement is a hard abstention. No gap is interpolated, bridged or silently converted to zero.

## Deterministic mathematics

Coordinates are integer micrometres. Let d_i = z_observed_i - z_reference_i. Every area is reported as **twice integer µm²** so half-square-micrometre results remain exactly representable without floating point.

For a same-sign segment of width Δx, the doubled positive or negative lobe is Δx × (|d_0| + |d_1|). For an opposite-sign segment, the linearly interpolated zero crossing splits it into two triangles. The doubled positive lobe is:

A⁺₂ = Δx × p² / (p + n)

and the doubled negative lobe is:

A⁻₂ = Δx × n² / (p + n)

Here p and n denote the magnitudes at the positive and negative endpoints, respectively. Each rational lobe is rounded to the nearest integer µm²×2, ties upward. Signed net area is added minus removed; absolute change is added plus removed. Mean absolute deviation is absolute doubled area divided by twice the supported profile length, rounded to nearest integer micrometre.

At each supported station, the conservative vertical uncertainty radius is reference uncertainty + observed uncertainty + maximum registration residual. Its piecewise-linear integral is returned as a separate doubled-area upper bound. It is not folded into the measured signed geometry change.

All arithmetic uses checked integer operations. Overflow causes abstention, never wraparound or saturation into a plausible measurement.

## Frozen corpus and tests

The JSON corpus defines:
- 4 positive known-answer profiles: identical, constant positive offset, sign crossing and constant negative offset;
- 1 explicit long-internal-gap known-answer rejection;
- 25 additional named negative controls;
- 30 total cases (4 positive / 26 negative).

The Rust library has 32 unit/mutation tests. A separate JavaScript replay should reproduce the four numeric positive controls and the negative-gap outcome. All quantities are synthetic; they are not scanner calibration or physical weld observations.

## Benchmark targets

| Metric | Software target | Later physical target |
|---|---:|---:|
| Positive known-answer agreement | 100% | Independent calibrated reference |
| Frozen invalid-control rejection | 100% | Independent fault-injection/measurement campaign |
| Gap handling | No unsupported-gap interpolation | Report coverage and max gap per scan |
| Geometry area | Exact fixed-point known answers | Bias/error versus traceable reference artifacts |
| Uncertainty | Exact conservative bound on synthetic cases | Empirical coverage, interval width and stratified calibration |
| Provenance | Zero scan/receipt cross-binding | Hash and calibration custody chain |
| Decision safety | Abstain outside stated limits | Abstention risk/coverage under physical use |
| Runtime | Deterministic O(n) profile traversal | 10³–10⁶ profile-point stress tests on declared hardware |

Do not compare a 2D cross-section error directly with the 0.1334 mm trajectory RMSE reported by Wang et al. A future matched comparison must use the same profile region, datum, sample spacing, uncertainty method, instrument and acceptance threshold.

## Claim ceiling

A PASS establishes only deterministic qualification and comparison of synthetic registered 2D profiles. It does not establish laser-scanner calibration, real 3D registration, BAM raw-data admission, physical bead geometry, fusion/penetration, weld quality, NDT acceptance, residual stress, thermal history, WPS/PQR/WPQT compliance or fully cooled/restraint-released distortion.
