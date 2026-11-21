# GAUGE raw rigid-body trial contract v0.1

**Parent:** [BENCH-GAUGE-RIGID-001 #6446](https://github.com/Luminous-Dynamics/symtropy/issues/6446)  
**Child issue:** [#6580](https://github.com/Luminous-Dynamics/symtropy/issues/6580)  
**Source revision:** `f95bb06e7d11142f88ffd3b6b56c7258deb8fff6`  
**Scope:** structural parser and derived-view contract only. No raw physical-trial data is vendored.

## Observed raw files

Two representative files in the public GAUGE dataset were inspected:
- `data/rigid/bouncing ball/json/1.json` (12.3 kB);
- `data/rigid/newton's cradle/json/1.json` (416 kB, multiple body streams).

Both declare `FPS: 30`, `translation unit: mm`, and `euler order: ZYX`. Each body stream provides arrays:
- translation: `x`, `y`, `z`;
- quaternion components: `qx`, `qy`, `qz`, `qw`;
- Euler components: `ex`, `ey`, `ez`.

The Newton's-cradle sample contains multiple named body streams (including `ball_passive`). This confirms that the released benchmark includes machine-readable rigid-body pose trajectories, not only video.

## Parser contract

This v0.1 parser supports only the observed declaration profile:
- positive finite FPS;
- translation unit exactly `mm`;
- Euler order exactly `ZYX`;
- one or more named body streams;
- all ten pose channels present, finite and equal-length, with at least one sample.

It rejects missing channels, mismatched lengths, invalid scalar declarations, malformed JSON and zero-norm quaternion samples. It reports maximum quaternion norm deviation; it does not normalize or rewrite the source values.

The parser uses only the Rust standard library. It validates the entire JSON document, enforces a maximum nesting depth, rejects duplicate object keys and trailing payloads, and decodes JSON string escapes (including surrogate pairs). This avoids adding a third-party dependency to the isolated benchmark crate.

## Acquisition rate versus export rate

The GAUGE paper reports that 16 motion-capture cameras acquire rigid-body trajectories at 180 Hz, then each recording is truncated to the valid interval and downsampled to 30 FPS for the released JSON. Rigid-body and textile simulator states are also exported at 30 Hz from simulations running at 180 Hz.

The raw JSON's `FPS: 30` therefore means **export cadence**, not original acquisition rate. The downsampling filter, sample phase/anchor, timestamp mapping and uncertainty propagation are not established by the inspected raw records. A 30 Hz export has 33.3 ms sample spacing and 15 Hz Nyquist; it cannot alone establish short impact peaks or sub-frame contact timing. Interpolation cannot reconstruct that lost high-frequency information.

## Derived views

- Position conversion: raw millimetres × 0.001 = metres.
- Relative sample time: zero-based frame index / declared FPS.

These conversions are derived views. The source JSON remains authoritative and unchanged. The derived time is **relative frame-index time only**, not a sensor timestamp, absolute clock, or proven capture start time.

## Explicitly unresolved semantics

The inspected files do not establish:
- quaternion handedness, algebra, multiplication convention or whether the transform maps body-to-world or world-to-body;
- world/body frame IDs and coordinate handedness;
- Euler angle unit and active/passive interpretation;
- absolute timestamp/clock origin and synchronization;
- per-sample measurement covariance or uncertainty;
- complete all-family missing-frame, outlier and repeated-recording policy.

The report marks these as unresolved. Its `XYZW` label describes the component key order (`qx/qy/qz/qw`) only; it is not a quaternion algebra or frame declaration.

## Claim boundary

A structurally valid file is not necessarily calibrated, physically accurate, licensed for arbitrary reuse, or eligible for physical scoring. This parser does not validate all eight rigid task families, import raw data, reproduce GAUGE metrics, or qualify Symtropy's physics. Complete family/material/trial schema review, file hashes, rights review and uncertainty/split audit remain gates under #6446.
