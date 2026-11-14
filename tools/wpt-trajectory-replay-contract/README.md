# WPT synchronized trajectory replay contract v0

Tracks the physical evidence bridge in issue #3726 (asset health) and issue #3861 (corridor co-design).

## Paradigm shift: one canonical trajectory, two physical projections

A vehicle's measured pose and motion are the common input to two separate model lanes:

- Electromagnetic response: receiver-to-transmitter gap, lateral alignment, coupling and useful vehicle-side power.
- Pavement response: left/right wheel paths, local temperature, heat flux, tensile strain and pad-interface condition.

The two lanes must not use independently assembled lane-position or time series. Every projection references the exact same trajectory ID and sample ID, uses the declared corridor coordinate frame, and carries a coordinate-transform SHA-256 digest. This validator checks evidence lineage; it does not calculate coordinate transformations or infer either physical response.

## Episode contract

Each canonical pose contains nanosecond timestamp, x/y/z, yaw/pitch/roll, speed, axle load and left/right wheel-path positions. Every pose has exactly one electromagnetic projection and one pavement projection. Duplicate IDs, missing poses, time reversal, frame mismatch, bad transform digests, clock misalignment beyond the declared uncertainty, invalid physical domains and unsupported evidence are rejected.

## Bounded validation

Maximum 512 samples per episode. The v0 contract is synthetic-only and requires a SYNTHETIC_ONLY flag. Unknown properties fail closed. The projection coverage check is one-to-one within each lane; both lanes must cover the entire canonical sample set.

## Known-truth fixture

The synthetic fixture contains three monotonically timestamped poses with lateral movement from -120 mm to +20 mm. All six projections point to those same three poses. It is a lineage and data-completeness control, not an electromagnetic coupling calibration or pavement damage model.

## Industry comparator

Fraunhofer IISB's robot-assisted IPT bench provides a repeatable laboratory misalignment regime with seven axes, a 13.5 m track, 170 kg load capacity at the farthest point and up to 2.4 m/s motion. Purdue–INDOT's Class-8 roadway demonstration is a separate regime reaching up to 65 mph. These speeds and configurations must not be blended into a single benchmark. The purpose of this contract is to make trajectory inputs reproducible across both test types.

## Validation

Run cargo test --locked -p wpt-trajectory-replay-contract.

Eight Rust tests cover shared sample coverage, timestamp monotonicity, duplicate sample IDs, unknown sample references, coordinate-frame agreement, projection completeness, trajectory/transform binding, clock-offset correction and synthetic-only admission.

## Claim ceiling

This crate validates trajectory identity, sample ordering, frame identity, transformation receipt syntax and one-to-one projection coverage for bounded synthetic episodes only. It does not validate coupling, pavement mechanics, thermal response, conversion efficiency, EMC, safety, lifetime or field performance.
