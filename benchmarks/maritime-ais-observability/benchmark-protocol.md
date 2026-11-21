# Maritime AIS Observability — Benchmark Protocol v0.1

**Status:** synthetic state-observer reference. No operational AIS feed or external raw dataset is included.

## 1. Research question

When can a local AIS feed support a defensible inference about vessel service state, and when must the system abstain because of short dwell, message gaps, identity ambiguity, kinematic inconsistency or a changed geofence?

The benchmark deliberately separates:
1. raw AIS message;
2. source/receiver observation;
3. upstream geofence/zone classification;
4. dwell-confirmed FSM inference;
5. independently qualified port-call milestone.

This crate owns step 4 and the associated falsification tests. It does not own raw RF reception, trajectory interpolation, geospatial polygon construction, datum transformation or DCSA event mapping.

## 2. Input contract

Each synthetic fix has:
- MMSI and source-local message ID;
- source and receiver identity;
- sender event tick (integer simulation tick);
- receiver observation tick (a separate integer simulation clock);
- upstream zone label;
- geofence digest and spatial-reference identifier;
- speed in integer milli-knots;
- position-valid flag;
- raw navigation-status lexeme, if present.

MMSI is a broadcast vessel reference, not an authoritative globally unique port-call or ship identity. A separate, qualified vessel/visit identity mapping is required before call-level aggregation.

All fixes in one call trajectory must retain the same source, receiver, geofence digest and spatial reference. A changed identity rejects rather than silently mixing observations from different sensing configurations. Sender event ticks and receiver observation ticks are independent clock domains; no offset or synchronization is inferred, and the two values are never subtracted from one another.

## 3. State observer

The local vocabulary is OUTSIDE, ANCHORAGE, APPROACH, BERTH and DEPARTURE, plus INDETERMINATE for insufficient observation support. These are this benchmark's declared names, not a claim to reproduce any paper's unpublished internal FSM names.

Each zone has a dwell and speed envelope. A transition is emitted only after:
- at least two supporting fixes;
- uninterrupted fix gaps no larger than the declared maximum;
- total observed candidate dwell meets the configured threshold;
- both sender-event gaps and receiver-observation gaps remain within their separately declared limits;
- speed is within the zone envelope.

An inferred event carries:
- lower bound = first qualifying fix in the candidate zone;
- upper bound = last supporting fix;
- confirmation time = the time at which the dwell condition became satisfied;
- supporting fix count and FSM profile version.

These bounds reflect the finite sampling frequency. The true polygon crossing can happen between the last fix in the previous zone and the first candidate fix; this v0.1 reports the candidate-zone evidence interval, not a confidence interval around physical arrival. Every inferred transition also carries its receiver confirmation tick, while each observability-gap event reports sender-event and receiver-time gaps separately.

## 4. Initial-state and continuity rules

The default API starts in INDETERMINATE. If the receiver starts while a ship is already in the berth polygon, the first stable dwell must be labelled as zone reacquisition, not arrival.

A caller may specify OUTSIDE or another known initial state only with a non-empty evidence reference. A production caller must bind that reference to an independently qualified pre-observation baseline, not a manually assumed value. The reference ID is copied into the report for audit.

A gap longer than the maximum observation interval:
- emits an observability-gap event;
- invalidates the current state to INDETERMINATE;
- resets a pending dwell candidate;
- prevents any continuous duration claim across the gap.

The next stable zone is a reacquisition, not proof that the corresponding physical transition occurred inside the gap.

## 5. Replay, identity and kinematic constraints

- Exact duplicate message IDs with exact identical content are idempotent.
- Reusing a message ID with different content rejects.
- Out-of-order ticks reject; no silent sorting or interpolation occurs.
- MMSI must be nine digits in this synthetic profile.
- Receiver, source, geofence digest and spatial reference are immutable within one call trajectory.
- Invalid positions reset candidate dwell and make the current state indeterminate.
- A speed above the zone-specific envelope during candidate dwell cannot confirm a transition.
- A speed contradiction while the FSM claims a stable zone invalidates the state claim and emits a kinematic-conflict event.
- AIS navigation-status lexemes are retained as input observations and conflicts are counted; those lexemes never override independently classified zone/motion evidence.

## 6. Frozen synthetic cases

The Rust test suite covers:
- stable berth dwell producing a lower/upper/confirmation interval;
- brief berth transit not producing a berth event;
- a candidate dwell broken by a long observation gap;
- exact duplicate message replay;
- conflicting message-ID reuse;
- out-of-order sender timestamps and receiver timestamps;
- contradictory nav-status lexeme that cannot grant berth state;
- dwell sensitivity across one to four ticks on the same fix sequence;
- departure as an AIS geofence-exit inference, not physical lines-untied;
- speed inconsistency invalidating a stable berth claim;
- geofence, receiver and CRS identity changes rejecting;
- a long gap while berthed followed by reacquisition;
- invalid position resetting continuity;
- indeterminate initial state preventing fabricated arrival;
- non-indeterminate initial state requiring an evidence reference;
- invalid MMSI and invalid dwell profile.

## 7. Industry comparison

### CPPI methodology

The World Bank / S&P Global CPPI methodology note describes AIS/geofence validation and six call events: anchorage/pilot station, movement to berth, cargo start, cargo end, berth departure and port-limit exit. It reports a 95% matching rate on its own reference dataset. That is a programme-specific benchmark statistic, not a universal requirement or proof that every local receiver has 95% completeness.

### Edge-AIS FSM field study

Ding et al. (2026), *An edge-based AIS finite-state machine framework for real-time port ship service performance monitoring*, reports a sub-USD-300 RTL-SDR/Raspberry Pi receiver, five dwell-confirmed service states, 917 calls with κ=0.96 agreement against crew navigation status, 26 of 27 scheduled berthings detected, 0.4-hour median departure-time error, and approximately ±10% KPI sensitivity to dwell/gap settings. Its data are available on request. This project does not claim access or reproduction.

### U.S. BTS dwell methodology

BTS Port Performance Freight Statistics documentation describes a dataset-specific rule set:
- minimum two-hour dwell for a vessel-call observation;
- consecutive events less than two hours apart merged;
- an edge observation gap no larger than one hour for valid dwell.
Its method illustrates concrete threshold sensitivity and gap-handling policy; these are not universal physical truths or South African thresholds.

### Open AIS segmentation research

Belcore et al. (2025), *Open AIS Data to Assess Seaside Operations in Container Ports*, describes trajectory segmentation and anchoring/berthing classification tested across U.S. port infrastructure. It is an additional algorithmic comparator, not a gold-standard label source.

## 8. Event-detection scorecard

Ground truth must use an independently adjudicated source-local visit/call reference. MMSI is not used as a visit identity. Each truth label is a specific event type and event tick in the same declared sender event-time domain as the inferred interval.

For truth event (g=(call,kind,t)) and prediction (p=(call',kind',[l,u])), a match is eligible only when:
- `call == call'`;
- `kind == kind'`;
- distance from (t) to interval ([l,u]) is no greater than the pre-registered tolerance.

Interval distance is zero when the truth tick lies inside [l,u], otherwise the distance to the nearest endpoint. The scorer constructs all eligible pairs and performs maximum-cardinality, minimum-total-interval-distance one-to-one matching using a residual min-cost-flow solver. Equal-cost path selection is deterministic under input ordering. This maximizes TP count before optimizing timing distance and avoids the recall loss possible with a greedy nearest-pair rule.

- matched pair: true positive;
- unmatched prediction: false positive;
- unmatched ground-truth event: false negative.

Precision is TP/(TP+FP); recall is TP/(TP+FN). Undefined denominators remain NOT_DEFINED (None), not zero. The scorecard separately reports mean interval-distance error and mean width of matched event intervals. It reports metrics by event kind and overall.

Duplicate inferred events cannot increase recall: only one prediction may match a ground-truth event. A prediction of the wrong event type or from the wrong call reference cannot match. Reacquisition and observability-gap events are diagnostics, not service-event detections, and are not converted into arrival/departure labels.

## 9. Promotion gates

Before real-port validation:
1. Establish legal raw AIS access and receiver/antenna identity.
2. Freeze the port/anchorage/approach/berth geometry, edition, CRS and datum.
3. Acquire independent terminal/port call ground-truth labels, including unresolved/ambiguous events.
4. Report source coverage, message gap distributions and unmatched MMSIs.
5. Evaluate precision, recall, false transitions, unknown rate, transition-time interval width, event delay and parameter sensitivity.
6. Group holdouts by port, receiver, vessel, call and calendar period; never tune dwell/gap on held-out calls.
7. Keep AIS-inferred zone entry separate from DCSA/PCO actual event semantics unless a reviewed source mapping supports the equivalence.

## 10. Non-claims

Synthetic conformance does not validate real AIS telemetry, geofence geometry, receiver deployment, DGT event accuracy, CPPI equivalence, causal port performance or navigation/berth authority.

## References

- CPPI methodology: https://thedocs.worldbank.org/en/doc/aac122f6df85534428d66a7b9af4b7f6-0400012026/original/CPPI-Methodology-Note.pdf
- Ding et al. (2026): https://doi.org/10.1016/j.trip.2026.102342
- U.S. BTS PPFS methods: https://www.bts.gov/PPFS-Tech-Docs
- U.S. BTS methods handbook: https://www.bts.gov/sites/bts.dot.gov/files/docs/browse-statistical-products-and-data/port-performance/217686/ppfsp-handbook-methods.pdf
- Belcore et al. (2025): https://doi.org/10.3233/PMST250026
