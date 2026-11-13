# DAS Industry and Field Baselines — Matched-Parameter Addendum v0.3

**Snapshot:** 2026-11-13  
**Programme:** SUF #4910 / DAS #4972 / source interchange #5004  
**Status:** public manufacturer literature and peer-reviewed paper abstracts; not a head-to-head test

## 1. Current product facts that change the comparison

### AP Sensing DAS N56 / 3P Vector
AP Sensing announced its N56-Series on 29 September 2026. It describes simultaneous monitoring of two independent fibre routes with up to 100 km per fibre (up to 200 km total in one instrument) and 3P Vector technology. AP's technical page explains the pulse-rate/Nyquist constraint and that optical loss, fibre type and backscatter profile affect reach and signal quality.

**Implication:** compare route count, mode, pulse rate, fibre attenuation and usable detection exposure—not just "200 km". Dual independent fibre routes may provide redundancy only when power, timebase, trench, chambers, backhaul and owner/rights failure domains are also considered.

### OptaSense QuantX
The current public product page describes 100 km quantitative data on standard single-mode fibre with 50,000 channels at 2 m pitch in the 100 km configuration; 1 m output channel pitch is available in other configurations, gauge length is adjustable, and high-rate raw acquisition can create a large data stream. The vendor page reports 3.2 Gbit/s in one acquisition context and H5 raw data for post-analysis.

**Implication:** this is a useful high-volume/raw-acquisition and quantitative-phase comparator. The 100 km/2 m mode must not be conflated with a shorter-range/1 m pitch profile or with independently measured spatial resolution.

### FEBUS A1
FEBUS publishes 70 km standard range and 100 km long-range option, two simultaneous synchronous channels, channel spacing down to 0.2 m, gauge length tunable from 1 m, and acquisition frequency up to 400 kHz. The product page says sensing is typically every 2 m over several tens of kilometres.

**Implication:** 0.2 m is a channel-spacing capability, not automatically a 0.2 m spatial resolution or event-location uncertainty. The actual datasheet/configuration and a two-point resolution test are required.

### Silixa iDAS / iDAS-MG
The current iDAS page reports up to 40,000 channels and channel spacing down to 25 cm, plus continuous/triggered acquisition, GPS/PTP synchronization and high-throughput data transfer. The 2024 iDAS datasheet reports 45 km range, 10 m gauge length, 25 cm–32 m sample spacing, 10 m spatial resolution and 400 Hz–100 kHz sampling. iDAS-MG offers selectable gauge lengths of 3, 5, 10 and 30 m, with range/noise tradeoffs.

**Implication:** this is a particularly clear example of why pitch and true resolution must be separate fields: 25 cm sample spacing is reported alongside a 10 m spatial resolution in the datasheet.

### OptaSense ODHF
ODHF differentiates quantitative phase mode up to 10 km from intensity-only mode up to 50 km, with 1.3–35 m spatial resolution depending on configuration and 10 m output channel pitch in the long-range mode.

**Implication:** quantitative and intensity-only modes are not interchangeable. Compare within the same observable, event class and mode.

## 2. Direct industry comparison: where Luminous should compete

| Dimension | Interrogator/product vendors | Luminous target |
|---|---|---|
| Optical source and interrogation | High-performance lasers, pulse schemes, fading reduction, range and bandwidth | Remain vendor-neutral initially; do not compete on optics before measured evidence and capital |
| Raw acquisition | Vendor-specific high-rate capture and viewers; some products expose H5/high-volume data | Preserve byte-exact source, export configuration, transform lineage and reproducible replay |
| Event detection | Vendor and integrator algorithms tuned to product/application | Independent benchmark with event-conditioned POD, misses, false alarms, uncertainty and explicit blind length |
| Fibre geolocation | Route calibration and mapping may be project/service specific | Separate optical range → fibre chainage → route station → projected geography with revision/digest bindings |
| Utility asset truth | Utility GIS/as-built systems own canonical geometry | Never replace owner truth; qualify provenance, conflicts, route attribution and freshness |
| Interoperability | Product suites, data viewers and project APIs | Typed source envelopes, OGC SensorThings observation interface and MUDDI-aligned spatial evidence |
| Investigation | Alarm, analytics, visualisation, client engineering support | Decision-value-aware investigation plan and independent evidence bundle |
| Lifecycle economics | Vendor/project service model | Cost per qualified observable km-year and per independently accepted actionable event |
| Security/authority | Product and customer-specific security controls | Query-level owner authorization, critical-geometry protection and hard read-only operational boundary |

Luminous should **complement and qualify** existing interrogators before contemplating an interrogator product.

## 3. Best field evidence baseline for buried utility fibre

Kamaruzaman et al. (2026), *Field-validated distributed acoustic sensing for real-time fault detection and localization in medium-voltage underground cables using operational SCADA communication Fibres*, Results in Optics 24 (March 2026), DOI 10.1016/j.rio.2026.101093, is a highly relevant task baseline.

The paper describes:
- a 5.4 km operational medium-voltage underground cable corridor;
- controlled laboratory tests at 16 kV and field testing using existing operational SCADA communications fibre;
- direct burial peak strain rate 1082.4 µε/s and SNR 22.27 dB in lab;
- conduit placement SNR 13.54 dB in the reported lab conditions;
- field event localization around 2.8 km on a calibrated route, with 9.52 µε/s peak strain rate and 19.57 dB SNR;
- three-second temporal periodicity matched to the excitation sequence;
- independent ground microphone inspection and physical excavation confirmation.

**Correct comparison:** this is a field-validated preliminary screening/localization case on a specific MV cable, fibre route, installation and event signature. It does not provide a general event-class POD denominator or false-alarm rate per fibre-km-day, so it cannot be treated as universal fault-classification performance.

**Luminous opportunity:** reproduce/extend this protocol with multiple event classes, no-event exposure, blind/degraded intervals, site-held-out splits, independent truth and explicit confidence intervals—while keeping utility protection/SCADA control outside the gateway.

## 4. Research state-of-the-art: what the latest results actually measure

- Cohen et al. (2026) buried fibre geolocalization: traffic-induced quasi-static strain maps plus video/GPS trajectories, sub-metre route geometry on the studied field/simulation conditions. It measures route reconstruction against tap-test truth, not fault POD.
- Ma et al. (2026) semantic-topological trajectory reconstruction: reported 89.13% F1 under low-SNR DAS conditions. This is vehicle trajectory association/reconstruction, not absolute cable geolocation.
- Zhuo et al. (2026) frequency-decoupled noisy urban DAS trajectories: reports mIoU 0.6017 and Dice 0.5268, plus recall gain over U-Net++ under the authors' field task. These are segmentation metrics.
- SVD-NLM (2026) fading-noise reduction: reports PSNR 31.751 dB and SSIM 0.9612. These are denoising descriptors, not critical-event preservation guarantees.
- Li et al. (2026) infrastructure DAS review: covers sensing mechanisms, deployment strategies, preprocessing, event localization/identification and engineering use; use it to stratify physical coupling and installation profiles.

The current public research does not yield one comparable "DAS SOTA" number. SUF must maintain separate leaderboards for:
1. vehicle trajectory segmentation and association;
2. traffic trajectory reconstruction;
3. relative fibre path reconstruction;
4. absolute route geolocation;
5. event detection and classification;
6. fibre-relative event chainage;
7. utility asset attribution;
8. data lifecycle/replay and decision value.

## 5. Reproducible vendor bake-off protocol

A fair pilot/bake-off uses the same physical fibre route and independently surveyed test points where multiple interrogators can be attached safely and legitimately. For each interrogator, freeze and publish:
- model, firmware, optical topology, acquisition mode and settings;
- gauge length, channel pitch, declared resolution test, pulse rate, temporal sampling, optical power/budget and fibre loss;
- source-native export and rights/retention constraints;
- clock/timebase and data gaps;
- event classes, magnitude strata, coupling profile and independent truth;
- raw data volume, bytes/compute/energy per km-day;
- event POD/false-alarm metrics, localization errors and uncertainty intervals;
- operator review burden, maintenance and total lifecycle cost.

Where one interrogator cannot export raw data or configuration, mark that comparison dimension unavailable rather than pretending parity. Use independent third-party event injection/survey/verification and no cross-vendor training leakage.

## 6. Research targets (not achieved results)

- Reproduce the Kamaruzaman 5.4 km field protocol or an equivalent authorized study before claiming superiority on MV cable diagnostics.
- Measure POD and false alarms using actual eligible event and no-event exposure denominators, not event demonstrations alone.
- Demonstrate at least 95% held-out route-position interval coverage on each declared fibre/coupling stratum, with interval widths and sample count.
- Demonstrate no high-confidence absolute route coordinate in the absence of an independent datum/control tie.
- Improve investigation time, survey cost or accepted-event economics at matched miss/false-dispatch constraints.
- Keep a Pareto result when costs and risk loss lack a defensible common unit.

## 7. References

- AP Sensing N56-Series news (29 September 2026): https://www.apsensing.com/en/news/product-launch-das-n56-series
- AP Sensing DAS/3P Vector technical page: https://www.apsensing.com/en/technology-and-products/distributed-acoustic-sensing
- OptaSense QuantX: https://www.optasense.com/technology/quantx/
- OptaSense ODHF: https://www.optasense.com/technology/odhf/
- FEBUS A1: https://www.febus-optics.com/en/page/a1
- FEBUS DAS parameter guide: https://www.febus-optics.com/en/page/distributedacousticsensing
- Silixa iDAS: https://silixa.com/technology/idas-intelligent-distributed-acoustic-sensor/
- Silixa 2024 iDAS datasheet: https://silixa.com/wp-content/uploads/iDAS_Datasheet_2024.pdf
- Silixa iDAS-MG: https://silixa.com/technology/idas-mg/
- Kamaruzaman et al. (2026): https://doi.org/10.1016/j.rio.2026.101093
- Cohen et al. (2026): https://doi.org/10.1109/TGRS.2026.3729504
- Ma et al. (2026): https://doi.org/10.1016/j.eswa.2026.131886
- Zhuo et al. (2026): https://doi.org/10.1016/j.iopt.2026.100055
- SVD-NLM DAS denoising (2026): https://doi.org/10.1109/JSEN.2026.3658349

## 8. Claim ceiling

All device parameters are manufacturer-published and configuration-dependent. All paper results belong to their authors' datasets and protocols. This document is neither an independent head-to-head test nor evidence of Luminous superiority, deployment compatibility or paid customer outcomes.
