# MILUV DS-TWR source audit and adapter contract

**Status:** source-code audit complete for the pinned public devkit revisions; dataset payload not admitted; no raw MILUV measurements parsed or scored by this document.

**Parent:** Symtropy DS-TWR interval baseline [PR #6793](https://github.com/Luminous-Dynamics/symtropy/pull/6793), head `7ce8904b1252967e4ee951096fb32ba2d797ceb5`.

## 1. Result

The timestamp-to-duration mapping and clock groups can now be traced directly to the MILUV paper and the authors' public reference implementation. The adapter no longer needs to guess the six-event DS-TWR mapping.

There is also a separate benchmark-integrity finding: the pinned upstream preprocessing chain filters rows using ground-truth-derived range bias before exporting its CSVs. See [Symtropy issue #6813](https://github.com/Luminous-Dynamics/symtropy/issues/6813). Therefore, a processed CSV controller view cannot be considered source-complete merely because evaluator columns are removed.

This audit supports an implementation plan; it does **not** validate the actual payload bytes, counter edge cases in downloaded MILUV sessions, antenna calibration, NLOS bounds, or physical range accuracy.

## 2. Audited source revisions

| Source | Revision | Relevant code |
|---|---|---|
| [MILUV devkit](https://github.com/decargroup/miluv) | `399ad46f4e6e88c3c98b40aae33fe8e2c6719cff` | [`preprocess/process_uwb.py`](https://github.com/decargroup/miluv/blob/399ad46f4e6e88c3c98b40aae33fe8e2c6719cff/preprocess/process_uwb.py) |
| [UWB calibration submodule](https://github.com/decargroup/uwb_calibration) | `c1a0354` | [`computecorrectedrange.py`](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/computecorrectedrange.py), [`utils.py`](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/utils.py), [`machine.py`](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/machine.py), [`postprocess.py`](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/postprocess.py) |
| MILUV paper | DOI `10.1177/02783649251405898` | [Paper](https://doi.org/10.1177/02783649251405898) |

The MILUV repository's preprocessing configuration sets `max_ts_value = 2**32`, `ts_to_ns = 1e9 * (1.0 / 499.2e6 / 128.0)`, and `ds_twr = True`. The pinned calibration implementation documents timestamps as uint32 values and applies this conversion before experiment-level clock unwrapping.

## 3. Source-verified DS-TWR event mapping

The paper defines the six timestamps in the two devices' local clocks. The reference implementation confirms these event assignments:

| Paper event | CSV field | Radio / clock domain | Physical event |
|---|---|---|---|
| (T^1) | `tx1` | Initiator (i) | First transmission |
| (R^1) | `rx1` | Responder (j) | First reception |
| (T^2) | `tx2` | Responder (j) | First response transmission |
| (R^2) | `rx2` | Initiator (i) | First response reception |
| (T^3) | `tx3` | Responder (j) | Second response transmission |
| (R^3) | `rx3` | Initiator (i) | Second response reception |

Therefore the paper equation

`y = c/2 * ((R2 - T1) - ((R3 - R2)/(T3 - T2)) * (T2 - R1))`

maps to the four duration inputs already used by [`ds_twr_range_interval`](src/ds_twr_integrity.rs):

| Kernel term | Verified construction | Clock domain |
|---|---|---|
| (A=R^2-T^1) | `rx2 - tx1` | Initiator (i) |
| (B=R^3-R^2) | `rx3 - rx2` | Initiator (i) |
| (C=T^3-T^2) | `tx3 - tx2` | Responder (j) |
| (D=T^2-R^1) | `tx2 - rx1` | Responder (j) |

This is not a six-timestamp subtraction in one global clock. (A) and (B) are formed on the initiator clock; (C) and (D) are formed on the responder clock. The rate ratio (B/C) is what relates the two local time scales.

## 4. Counter width, tick units and rollover

The authors' reference configuration specifies:

- Timestamp counter modulus: (2^{32}) ticks.
- Device tick rate: (499.2,	ext{MHz}	imes128 = 63.8976,	ext{GHz}).
- One tick: approximately (15.6500400641) picoseconds.
- Tick-to-nanosecond factor: approximately (0.0156500400641) ns/tick.
- One 32-bit wrap period: approximately (67.2164102564) ms.

The two clock streams must be unwrapped independently:

- Initiator (i): `tx1 → rx2 → rx3`.
- Responder (j): `rx1 → tx2 → tx3`.

The pinned `ComputeCorrectedRange._unwrap_ts` implementation performs a local three-event unwrap separately for those two sequences. The full `PostProcess._unwrap_all_clocks` implementation is more contextual: it groups timestamp observations by tag/clock stream, orders them using ROS time, and uses continuity checks to repair wraps across the experiment.

**Adapter requirement:** preserve raw uint32 values and clock identity in the raw-ingest layer. Unwrap each local oscillator independently using a versioned algorithm and experiment context. A one-row adapter must not guess the number of wraps from a negative delta if the observation gap could span more than one counter period.

Do not run this conversion or rollover logic a second time on already processed `*_raw` CSV fields; their semantics are explained next.

## 5. Important naming trap: processed `*_raw` fields are not raw device ticks

In `RosMachine`, UWB timestamp fields are multiplied by the configured DW1000 tick-to-nanosecond factor. The `PostProcess` stage then calls `_unwrap_all_clocks` before the MILUV `process_uwb.py` export code copies timestamp columns into `tx1_raw`, `rx1_raw`, and their peers.

The export script copies these fields before applying its antenna-delay and received-power calibration transforms. Consequently, the published processed `*_raw` suffix means **pre-calibration timestamp values**, not the original uint32 tick counters. These timestamp values have already passed unit conversion and clock unwrapping.

There are therefore two distinct supported ingestion modes, which must never be conflated:

### Mode A — original recorded messages / raw device ticks

- Retain original integer ticks, source message IDs, radio IDs, per-radio clock identity, ROS acquisition time and message ordering.
- Apply the exact device tick factor and independent per-clock unwrap once.
- Keep the unwrap algorithm/version, wrap count and any ambiguity in the receipt.
- Construct (A,B,C,D) only after the clock streams are independently qualified.

A bounded **per-transaction** raw-tick adapter is implemented in [follow-up PR #6815](https://github.com/Luminous-Dynamics/symtropy/pull/6815). It unwraps the initiator sequence [tx1, rx2, rx3] and responder sequence [rx1, tx2, tx3] independently, requires a caller-supplied maximum inter-event gap strictly smaller than one uint32 wrap period, and returns raw/unwrapped events, rollover counts and duration inputs.

This adapter is not a substitute for full experiment-stream clock reconstruction. The caller must justify the sub-wrap gap contract and establish that neither radio reset/rebooted during the exchange; a raw counter decrease alone cannot distinguish a reset from a wrap.

### Mode B — authors' processed MILUV CSV

- Treat `tx*_raw` and `rx*_raw` as already converted and experiment-unwrapped nanosecond values, subject to confirmation against the exact dataset payload and schema version.
- Convert the resulting duration differences from nanoseconds to seconds for the current interval kernel.
- Do not apply another uint32 unwrap or tick conversion.
- Preserve the source export revision and a clear `processed_unwrapped_ns` representation marker in the data receipt.

Mode B is implemented by the typed `MiluvProcessedUnwrappedTimestampsNs` input and `DsTwrDurationInput::from_miluv_processed_unwrapped_ns` in [follow-up PR #6815](https://github.com/Luminous-Dynamics/symtropy/pull/6815). The constructor maps event pairs to A/B/C/D and sums endpoint absolute-error bounds for each duration. It does not apply a second unit conversion or unwrap. Mode B is useful for reproducing processed CSV values; it cannot repair rows already removed by truth-conditioned preprocessing.

## 6. Propagation-speed constant: expose the baseline convention

The pinned reference implementation uses (299{,}702{,}547) m/s in both `computecorrectedrange.py` and `utils.py`. The current Symtropy interval kernel uses (299{,}792{,}458) m/s, the vacuum constant.

These must not be silently mixed in a reproduction. Their relative difference is approximately 0.03%, or about 9 mm at a nominal 30 m range.

Implemented in [follow-up PR #6815](https://github.com/Luminous-Dynamics/symtropy/pull/6815): `propagation_speed_mps` is now an explicit validated kernel input, with named MILUV-reference and vacuum constants. The caller must preserve the selected value with its evidence receipt; the kernel does not yet serialize a complete measurement receipt.

Still required:

- Include propagation-speed convention in every serialized range receipt.
- Bound propagation-speed uncertainty when making a physical claim rather than reproducing the reference convention.
- Retain a separate vacuum/physical-model mode only when the experiment explicitly justifies it.
- Do not infer a universal RF propagation velocity from either constant alone; state the medium and calibration convention.

The MILUV reference numeric constant is a reproduction parameter. The actual physical propagation model, environment and associated uncertainty remain part of the experiment specification.

## 7. Calibration is a separate transformation

The reference code constructs the DS-TWR durations, then applies tag-specific antenna-delay corrections to (A) and (D), and separately applies a received-power-dependent correction. The range result is therefore not just the raw timestamp equation.

Keep these as separately attributable transformations:

1. Raw timestamp events and clock-domain reconstruction.
2. Clock-ratio / DS-TWR interval propagation.
3. Antenna-delay correction with calibration artifact identity and uncertainty.
4. Received-power correction with model version, domain and uncertainty.
5. NLOS/multipath uncertainty and any positive-bias assumption.
6. Final range interval and acceptance/abstention decision.

Do not collapse antenna delay, clock uncertainty, power bias and NLOS excess path into one unexplained standard deviation. Hard bounded systematics and calibrated statistical residuals need separate composition rules. The current positive-only NLOS-bias helper is valid only where nonnegative excess-path bias is independently justified.

## 8. Controller/evaluator isolation: row-selection leakage

The pinned `PostProcess._process_data` calls `_store_distance_data()` before filtering rows by `abs(bias) < 4`. `_store_distance_data` obtains `gt_range` from motion-capture poses and calculates `bias = range - gt_range`. The resulting filtered `data.df` is what the MILUV export script later writes.

This means the existence of a processed CSV row is already conditioned on evaluator truth. A controller field allowlist, label-stripped table or BLAKE3 digest over the surviving rows cannot establish source completeness.

**Required benchmark boundary:**

- Source-complete controller observations must be taken before any truth-derived row filter, preferably from the original recorded messages or an author-provided unfiltered export.
- Evaluator truth may annotate/exclude records in a separate view; it must not change controller-visible row IDs, row count, ordering or values.
- All invalid, missing and excluded transactions must remain traceable by event identity with an evaluator-side reason.
- Negative controls must mutate `gt_range`, `bias`, Vicon pose and truth-derived thresholds while requiring the controller event stream and digest to remain byte-identical.

Tracked in [issue #6813](https://github.com/Luminous-Dynamics/symtropy/issues/6813). The currently processed MILUV CSV must not be promoted as a source-complete controller benchmark until this is resolved.

## 9. Measurement availability and missingness are part of the benchmark

The paper describes CIR and first-path / received-power information as measurement-side evidence, but the complete set of these features is not necessarily available for every event. CIR capture also reduces the measurement rate. Passive receptions are not a universal independent witness: receiver availability, shared propagation paths, common radio clocks, power supply, antenna placement and obstruction can create shared failure causes.

The benchmark must therefore record, per event:

- Which radio actually observed the signal.
- Whether CIR, first-path power, received power and clock-skew fields are present and valid.
- Whether the receiver is active or passive.
- Acquisition rate and missing-observation reason.
- Shared channel/obstruction and clock/power dependency hypotheses.
- Whether a claimed second witness contributes independent failure information.

Do not score a method only on the subset where its preferred sensor happens to be available. Report the complete-cohort service result, feature-availability strata and acquisition-induced rate/latency/energy costs.

## 10. Required implementation and verification sequence

1. Both processed/unwrapped-nanosecond and bounded per-transaction raw-tick adapters are implemented in #6815. Full experiment-stream continuity, radio reset/boot-epoch detection and any multi-wrap ambiguity still require separate evidence.
2. Add source-verified constructors for the four duration terms and explicit initiator/responder clock groups.
3. Initial independent per-clock rollover and dual-clock wrap tests are implemented in #6815. Still add stream-level tests for radio resets, long gaps, multiple possible wraps and reconciliation against ROS acquisition time.
4. Add a known-answer test that compares the Rust equation with the authors' formula using the same (A,B,C,D), speed constant and units.
5. Explicit propagation speed and named MILUV-reference / vacuum constants are implemented in #6815. Receipt serialization and bounded physical-speed uncertainty remain open.
6. Add calibration receipt interfaces without allowing the estimator to access evaluator-only truth.
7. Resolve issue #6813 before source-complete dataset admission; controller event IDs and row count must be invariant to truth mutation.
8. Freeze exact upstream code revisions, payload hashes, schema, experiment-level splits and evaluation oracle before reporting any MILUV score.
9. Only then reproduce the published UWB-IMU EKF and the raw DS-TWR / calibration pipeline, including all documented failures and excluded cases.

## 11. Required source references

- [MILUV paper and protocol equation](https://doi.org/10.1177/02783649251405898)
- [MILUV preprocessing configuration and export path](https://github.com/decargroup/miluv/blob/399ad46f4e6e88c3c98b40aae33fe8e2c6719cff/preprocess/process_uwb.py)
- [Pinned UWB calibration range reconstruction](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/computecorrectedrange.py)
- [Pinned UWB calibration utility equation](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/utils.py)
- [Pinned timestamp conversion and clock definitions](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/machine.py)
- [Pinned experiment-level clock unwrapping and truth-derived row filter](https://github.com/decargroup/uwb_calibration/blob/c1a0354/pyuwbcalib/postprocess.py)
- [Symtropy interval-valued DS-TWR baseline](https://github.com/Luminous-Dynamics/symtropy/pull/6793)
- [Symtropy truth-conditioned row-selection issue](https://github.com/Luminous-Dynamics/symtropy/issues/6813)

## Claim boundary

This is a source-code mapping and data-lineage audit. It is not a reproduced MILUV experiment, calibrated range-error model, physical benchmark result, NLOS classifier, statistical integrity guarantee or safety certification. All numerical tick and wrap values above are derived from the publisher's documented configuration constants. Physical calibration, raw payload admission and independent evaluation remain outstanding.
