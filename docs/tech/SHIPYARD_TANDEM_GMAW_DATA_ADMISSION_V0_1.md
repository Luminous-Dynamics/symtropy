# BAM Tandem-GMAW dataset admission v0.1

Tracking: [SHIPYARD-TANDEM-GMAW-DATA-ADMISSION-001 #4643](https://github.com/Luminous-Dynamics/symtropy/issues/4643)

## Source identity

- Zenodo DOI: https://doi.org/10.5281/zenodo.21824586
- Official record: https://zenodo.org/records/21824586
- Version 2.0.0; published 7 August 2026; modified 10 August 2026.
- Publisher: BAM (Bundesanstalt für Materialforschung und -prüfung).
- Five workpieces (WID615, WID617, WID630, WID631, WID632), six bead/layer stages per workpiece.
- Record reports an archive total of 4.5 GB.
- The record advertises Dataset Open but its License field is blank and its Rights section says Copyright (C) 2026 The Authors.

**No raw download, vendoring, mirroring, public derivative release or model training is authorized by this manifest.** The rights state is RIGHTS_PENDING until the source owner/data curator supplies explicit terms in writing. Do not infer reuse rights from record openness or the ability to download a file.

## Dataset topology summarized without copying the raw data

The source record describes S355J2+N base material, ER70S-6 wire, 30 mm plate, approximate 50-degree groove, minimum seam length 300 mm, and a Fronius TWIN arrangement with two TPS 500i power sources.

Five workpieces / six layers each = 30 metadata-level layer slots. L1 is a single-wire root pass; L2-L6 are tandem passes.

| WID | L2-L6 tandem mode |
|---|---|
| WID615 | PMC + CMT |
| WID617 | PMC + PMC |
| WID630 | CMT + CMT |
| WID631 | PMC + PMC |
| WID632 | PMC + CMT |

The source record describes robot motion, voltage/current, leading/trailing source recordings, pre-pass 3D surface scans, and HDR video by layer. The final L6 archive additionally describes a post-weld surface scan. The video capture arrangements/settings may differ.

The public experiment index describes columns for WID/layer/track, configuration, travel speed, tandem process mode, leading/trailing modes, commanded wire-feed values, weave amplitude/frequency, t85, robot pose and file references. This v0.1 metadata manifest does **not** copy the CSV values or raw file contents.

### t85 handling

The source CSV's unit row labels t85 in seconds, but this intake has not established the field's exact semantic definition or provenance. Values are therefore marked NOT_INGESTED / UNRESOLVED and null in the manifest. Blank source values are never changed to zero; a source unit alone does not prove that the field is a measured metallurgical t8/5 value.

## Research contract

- Each observed pass is a multi-stream event: robot motion, voltage/current, power-source recordings, pre-pass scan and (where available) video.
- The source-native timebase for each stream must be preserved. Two synchronized welding sources do not prove the robot, process, scan and video clocks share a common timebase.
- Layer-level and sensor-level observations are nested within WID. The independent workpiece count is five, not 30 layer rows or millions of samples.
- Train/validation/test splits are grouped by complete WID. No layer/window/frame from one WID may cross partitions.
- The five-WID mode assignment is not a randomized experiment. The modes are confounded with workpiece identity and other process settings. Do not claim causal superiority for PMC or CMT from this dataset.
- The source describes pre-pass scans at each layer and a post-weld scan on final L6. Do not infer a post-pass scan for each intermediate layer or a final fixture-released shape without supporting evidence.
- Process data and surface scans do not establish weld acceptance, NDT capability, residual stress, thermal history, fatigue performance or class approval.

## Existing owners

- #4433 owns generic pass/workpiece/equipment/process-trace identity.
- #4614 owns physical model VVUQ and measured-source admission.
- #4633 owns final free-state weld sequence/fixture/release co-design.
- #3331 owns component and as-built lineage.
- #3784 owns generic calibration and experiment design.

This issue owns only this source's rights/admission profile, layer/artifact mapping, timebase and missingness contract, and WID-level replay/report.

## Initial validation protocol

1. Resolve reuse and derivative/training permissions with BAM/data curators/authors.
2. Pin record version, source retrieval time and official file inventory. The source-reported MD5 values are download-integrity hints, not signer authenticity or licence grants.
3. If authorized, pilot only one WID and L1-L2 before considering a full 4.5 GB intake.
4. Validate NetCDF/HDF5 variables, units, sampling rates, timebases, missingness, calibration, dropouts and leading/trailing stream correspondence.
5. Validate scan coordinate frames, registration, resolution, uncertainty and relation to robot/world coordinates.
6. Bind every derived feature to exact source bytes, transformation code/version, input/output digests and units.
7. Group all splits by WID and report per-WID outcomes. With only five clusters, inference remains exploratory.

## Frozen controls

The 31-control contract corpus includes one positive metadata-only baseline and 30 negative cases for rights promotion, version/source substitution, integrity drift, wrong layer topology, L1/L2-L6 semantic confusion, missing trailing source, absent scans, t85 imputation/overinterpretation, timebase assumptions, video comparability, scan roles/datum, WID leakage, pseudoreplication, causal mode claims, process-to-quality promotion and unauthorized raw data persistence.

## Claim ceiling

PASS means only that the publicly described record, its 5×6 topology, artifact-role contract and rights/missingness/split boundaries are faithfully represented. It is not a parse of the raw binary files, a physical experiment result, a weld-process qualification, a weld-quality/NDT finding, thermal or residual-stress validation, causal process comparison, released-state distortion measurement or permission to use the raw data.
