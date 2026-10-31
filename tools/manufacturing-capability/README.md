# Time-windowed manufacturing capability reference

Standalone Rust descriptive diagnostic comparing pooled within-subgroup capability, overall process performance and worst-window capability across ordered production windows.

## Scope and interpretation

The first version intentionally does not implement a formal control-chart/SPC system or claim ISO 22514-2:2026 conformance. Cp/Cpk/Pp/Ppk are descriptive indices computed under an explicitly **unverified Normal-shape assumption**. They are not evidence that the process is stable or statistically controlled.

The key distinctions:
- **Pooled within-subgroup Cp/Cpk** uses pooled within-window variance. It can hide temporal mean shift.
- **Overall Pp/Ppk** uses total observed sample variation. It reflects between-window shifts but does not identify the time of the change.
- **Worst-window Cpk** reports the minimum descriptive Cpk among the actual time windows.
- **Guarded mean margin** subtracts an externally supplied expanded uncertainty from the mean's distance to each specification limit. It is a mean/measurement diagnostic, not a full predictive distribution for future parts.
- **NO_SHIFT_FLAG_DETECTED** means the frozen threshold rules did not flag mean span or variance ratio. It does not mean the process is stable.

## Synthetic fixtures

Each window contains eight deterministic samples:

    mean + scale * [-1.5,-1.0,-0.5,-0.2,0.2,0.5,1.0,1.5]

Four campaign families:
- stable control: all means 0.5, all scales 0.05;
- linear drift: means 0.20, 0.45, 0.70, 0.95; scales 0.05;
- variance shift: mean 0.5; scales 0.02, 0.02, 0.10, 0.18;
- combined drift and variance shift.

A mean-span flag occurs when window mean span exceeds 10% of specification width. A variance-shift flag occurs when the largest window sample variance exceeds the first-window sample variance by a factor greater than 3. These are deterministic research thresholds, not universal SPC limits.

## Evidence contract

Each row binds part, lot, operation, station, instrument, calibration, characteristic, unit, timestamp and source digest. Calibration is required to be valid at each measurement time. The campaign also consumes a qualified MSA state, specification evidence and uncertainty-budget evidence. Incomplete evidence rejects the capability computation rather than generating optimistic indices.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-capability/windowed_capability.rs -o /tmp/windowed-capability-tests
    /tmp/windowed-capability-tests

    rustc --edition=2024 -D warnings tools/manufacturing-capability/windowed_capability.rs -o /tmp/windowed-capability
    /tmp/windowed-capability --verify
    /tmp/windowed-capability --csv

## Competitive benchmark ladder

First compare the same frozen subgroup data with an independently implemented statistical package, holding the subgroup definition and capability formula constant. Then test false-alarm and detection performance under synthetic stable, drift, variance and combined regimes. Finally validate on complete held-out lots/campaigns with an approved measurement system.

The next stage should add EWMA/Page-Hinkley or an explicitly chosen control chart, calibrated against stable-process false-alarm rates and a declared detection-delay objective. Do not promote this threshold flagger to a process controller.

## Claim ceiling

This is a synthetic time-windowed descriptive capability benchmark. It does not establish a process is stable, capable or in statistical control; does not certify parts or lots; does not implement ISO 22514-2 in full; and does not authorize process adjustments or product release.
