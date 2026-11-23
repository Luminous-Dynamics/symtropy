# Maritime AIS Observability — Industry Comparator Register

**Research snapshot:** 21 November 2026.

## 1. CPPI 2025 methodology

The World Bank / S&P Global methodology note documents:
- carrier-reported timestamps from ten leading liner companies, representing approximately 75–80% of global container fleet capacity;
- AIS-derived vessel movement validated against port geofences;
- a reported 95% matching rate for the methodology's reference calls;
- six events: anchorage/pilot station arrival, movement to berthing place, cargo operations start, cargo operations end, berth departure and port-limit exit;
- vessel capacity, call-size and crane-deployment information for workload normalization.

This is the strongest industry-scale reference for event matching and port-call cohort validation. Its reported 95% match rate belongs to its dataset/method and must not be turned into a universal local receiver threshold.

Source: https://thedocs.worldbank.org/en/doc/aac122f6df85534428d66a7b9af4b7f6-0400012026/original/CPPI-Methodology-Note.pdf

## 2. Edge AIS field comparator (November 2026)

Ding et al., *Transportation Research Interdisciplinary Perspectives* 40 (2026), article 102342:
- https://doi.org/10.1016/j.trip.2026.102342
- Sub-USD-300 RTL-SDR / Raspberry Pi shore station.
- Five-state dwell-confirmed FSM, using port, berth and anchorage polygons plus vessel speed.
- Reported crew-navigation-status agreement κ=0.96 across 917 port calls.
- Detected 26/27 berthings in terminal schedules.
- Median berth departure-time error: 0.4 hours.
- KPI sensitivity: approximately ±10% under dwell/gap parameter changes.
- Paper says data are available on request.

This is a compelling cost/accessibility comparator, but it is a single-port deployment with small schedule-based berth validation (27 cases) and parameter sensitivity. We do not claim reproduction or data access. Our benchmark should match its event definitions where documented and challenge it with missing messages, source delay, polygon drift, receiver outage, duplicate/replayed fixes and independent call labels.

## 3. U.S. BTS Port Performance Freight Statistics

The BTS methodology provides an explicit operational comparator for AIS dwell-event filters:
- minimum two-hour dwell;
- merge repeated calls separated by less than two hours;
- maximum one-hour observation gap at the edge of a dwell event.
Its report notes that AIS receiver/terrain/weather interruptions, inaccurate coordinates and repeated call detections can bias dwell estimates. These thresholds are dataset-specific and should be swept, not treated as universal South African requirements.

Sources:
- https://www.bts.gov/PPFS-Tech-Docs
- https://www.bts.gov/sites/bts.dot.gov/files/docs/browse-statistical-products-and-data/port-performance/217686/ppfsp-handbook-methods.pdf

## 4. Open AIS trajectory segmentation

Belcore et al. (2025), *Open AIS Data to Assess Seaside Operations in Container Ports* (DOI 10.3233/PMST250026), uses public U.S. AIS data, trajectory preprocessing, segmentation and anchoring/berthing classification across multiple container-port settings. It is a reproducible method comparator, but port statistics are not pointwise ground truth.

Source: https://doi.org/10.3233/PMST250026

## 5. Metrics that matter

Compare all methods on the same receiver/time/feed and held-out calls:
- transition precision/recall and confusion by state/event type, scored by one-to-one source-local call/event matching;
- event interval-distance error and matched interval width, separately from confirmation latency;
- false berth-arrival and false port-exit rate;
- event-time median/P90/P95 interval width and absolute error;
- sender event-to-receiver observation latency distribution, without assuming clock synchronization;
- unknown/abstain rate and time spent INDETERMINATE;
- fix-gap and receiver-outage sensitivity;
- duplicate/replay/conflict rejection;
- dwell, gap and speed-threshold sensitivity;
- receiver/terminal/vessel-grouped holdout generalization;
- compute, hardware cost and power consumption.

A single accuracy, κ or matching percentage must never conceal false-positive berth calls, unmatched ships or event-time uncertainty.
