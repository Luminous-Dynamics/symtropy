# DAS Commercial Specifications — Matched-Parameter Update v0.2

**Snapshot:** 2026-11-13  
**Owner:** SUF #4910 / DAS #4972  
**Status:** manufacturer public product literature; not independent qualification

## 1. Why the benchmark needs parameter-level comparison

Interrogator marketing frequently presents sensing range, output channel count/pitch, gauge length, pulse/repetition rate, spatial resolution and bandwidth together. They are different parameters. A customer cannot infer the actual detection/ localization performance of an installation from any one of them.

The Luminous comparator should freeze the exact setting and fibre profile used in each result, not compare the maximum number quoted for a product family.

## 2. Current public vendor specifications

| Vendor / product | Public manufacturer statement | Important comparison caveat |
|---|---|---|
| AP Sensing 3P Vector DAS | Manufacturer describes reduced fading and stable signal; up to 2 × 100 km measurement range; discusses pulse-rate/Nyquist limits and typical event frequency bands | 2 × 100 km is a vendor claim and does not establish POD throughout the link, spatial resolution or false-alarm performance. Actual frequency limits depend on pulse rate and fibre length. |
| OptaSense ODHF | Vendor describes quantitative mode up to 10 km and intensity-only mode up to 50 km; maximum ping rate 50 kHz; spatial resolution 1.3–35 m and 10 m output channel pitch in the long-range intensity mode | Quantitative and intensity-only modes are different measurement products. Channel pitch and actual spatial resolution are not equivalent. |
| OptaSense QuantX | Vendor describes up to 100 km quantitative data, 50,000 channels, 1–2 m output channel pitch depending on mode, and adjustable gauge lengths | Maximum range and channel pitch do not independently specify sensitivity, resolution or event-detection performance over the full link. Gauge length is a separate spatial averaging parameter. |
| FEBUS A1 | Vendor states 70 km range (100 km long-range option), two synchronous channels, channel spacing down to 0.2 m, gauge length tunable from 1 m, acquisition frequency up to 400 kHz | Fine channel spacing is not a claim of 0.2 m spatial resolution or guaranteed detection at maximum range. Settings, optical budget, gauge length and environment must be frozen. |
| Silixa iDAS | Vendor states selectable channel spacing down to 25 cm, up to 40,000 measurement channels per connected fibre and pulse rates up to 100 kHz | 25 cm channel spacing is sampling density, not necessarily spatial resolution. Fibre mode, pulse rate and gauge length need to match the tested use case. |

## 3. Vendor-neutral parameter dictionary

Every acquired/exported dataset should declare the following separately, or explicitly mark the field unavailable/withheld:

- **Sensing range:** furthest optical distance for which the instrument supports a declared acquisition mode under the stated link budget/profile.
- **Output channel pitch:** spatial interval between adjacent output measurement channels.
- **Gauge length:** fibre length over which the phase/strain estimate is formed or averaged.
- **Spatial resolution:** ability to distinguish two nearby physical perturbations under a specified measurement configuration and acceptance test.
- **Pulse/repetition rate:** optical interrogation frequency; influences temporal sampling and frequency bandwidth together with range and instrument architecture.
- **Sample rate / temporal sample interval:** actual time sampling of exported data.
- **Dynamic range / noise floor / sensitivity:** measurement response under a declared mode, fibre and environmental condition.
- **Fibre/coupling profile:** cable construction, installation, depth, direct burial/conduit, splice count, slack, fibre type, optical budget and host/target geometry.
- **Detection capability:** event class/magnitude, POD, critical miss rate, false alarms per observable fibre-km-day and independent ground truth.

Channel pitch must never be used as a proxy for spatial resolution. Gauge length must not be substituted for channel pitch. Maximum sensing range must not be treated as qualified usable length.

## 4. Revised Luminous positioning

Luminous should not try to win on nominal sensing range or smallest channel pitch. The platform should:
1. ingest vendor-native output and exact model/configuration metadata;
2. preserve source byte identity and configuration;
3. normalize only declared observation semantics;
4. measure event-conditioned POD and blind/degraded coverage on matched fibre profiles;
5. bind fibre chainage to route geography with a separate calibration receipt;
6. compare vendors at matched event class, magnitude, optical budget and route profile;
7. publish per-class coverage and uncertainty without promoting marketing specifications into measured results.

## 5. Pilot negotiation checklist

Request from each vendor or integrator:
- exact interrogator model/revision, firmware and operating mode;
- gauge length, channel pitch, sample rate, pulse rate, decimation and data type;
- range-vs-fibre-loss and sensitivity curves where available;
- native waveform/export samples, metadata schema and rights to independent replay;
- calibration and clock synchronization description;
- source-side filtering, compression and event-detection logic;
- API/export protocol, retention/storage rate and full acquisition-rate limits;
- a matched test plan with independent reference events and no-event exposure.

A vendor may legitimately decline raw export or disclose only derived outputs; record that integration boundary and do not claim equivalent raw-data replay.

## 6. References

- AP Sensing DAS: https://www.apsensing.com/en/technology-and-products/distributed-acoustic-sensing
- OptaSense ODHF: https://www.optasense.com/technology/odhf/
- OptaSense QuantX: https://www.optasense.com/technology/quantx/
- FEBUS A1: https://www.febus-optics.com/en/page/a1
- FEBUS DAS parameter guide: https://www.febus-optics.com/en/page/distributedacousticsensing
- Silixa iDAS: https://silixa.com/technology/idas-intelligent-distributed-acoustic-sensor/

## 7. Claim ceiling

This is a public specification inventory. All numbers are vendor-reported, often mode- and configuration-dependent. No independent head-to-head test, matched field trial or Luminous performance superiority is claimed.
