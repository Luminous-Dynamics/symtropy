# Coupon manufacturing record templates

These header-only CSV files are data-entry templates, not measurements or a qualified manufacturing record.

- coupon_build_traveler_template.csv: one row per specimen/build attempt, including geometry digest, material lot, process version, start/end, material, failures/rework, labour, direct cost and raw log reference.
- coupon_measurement_template.csv: one row per measured characteristic, including nominal and measured value, units, expanded uncertainty, instrument/calibration identity, acceptance limits and raw evidence.

## Data-entry rules

- Leave unknown fields blank and mark the record or dependent conclusion as unresolved; never populate unknown cost, uncertainty, calibration or measurement values with zero.
- Use one stable experiment ID, specimen ID and geometry digest throughout. STL has no units; retain the linked manifest and use the unit declaration in it.
- Calculate hashes over the exact retained artifact bytes. A file path or operator-entered digest is a reference, not a verified hash, until recomputed.
- Freeze acceptance limits before inspecting results. Do not back-fit tolerances to the manufactured specimen.
- Keep all failed builds, rework, repairs, exclusions and aborted measurements. They are part of yield and reproducibility.
- Use calibrated instruments with traceable records and declare the uncertainty method/coverage. A displayed precision is not measurement accuracy.
- The record status does not grant physical-test authorization. The sample remains non-flight-critical and facility suitability must be assessed separately.
