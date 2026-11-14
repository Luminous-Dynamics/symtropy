# WPT exact temporal service-feasibility oracle v0

This crate provides a deterministic small-instance oracle for the service-feasibility lane in [#3559](https://github.com/Luminous-Dynamics/symtropy/issues/3559).

## Question answered

Given interval-indexed conservative energy-capacity bounds, can every enrolled receiver obtain its minimum battery-terminal energy obligation before its departure deadline?

The package constructs a time-expanded capacitated network:

1. The source feeds each segment/interval node up to its prequalified deliverable-energy bound.
2. Segment/interval nodes feed receiver/interval nodes through bounded transfer links.
3. Receiver/interval nodes feed each receiver's aggregate obligation node up to its battery-acceptance cap.
4. Each obligation node feeds the sink up to that receiver's minimum-service requirement.

A maximum flow equal to the sum of all minimum obligations is a constructive feasibility witness. A smaller maximum flow is an infeasibility witness; the residual minimum cut identifies the capacity boundary that prevents full service.

## Arithmetic and semantics

- All capacities are unsigned integer mWh. No floating-point optimization or tolerance is used.
- Time windows are half-open: `[arrival_interval, departure_interval)`.
- All capacities share a common **deliverable-energy** boundary. The upstream metrology / energy-accounting layer must convert source measurements and losses into conservative endpoint energy bounds. This crate does not apply an efficiency factor or infer missing loss terms.
- The submitted segment, transfer and receiver capacities must be **jointly realizable** for the simultaneous paths admitted by the graph. Individually conservative pairwise link maxima are insufficient when mutual coupling, cross-regulation, thermal interaction or switching constraints make those links incompatible when used together.
- A coupled constraint may be represented as an additional shared resource node only when it has a valid network-flow representation. General multiport, nonlinear or configuration-dependent constraints need an exact MILP/MIQP or other suitable oracle; this max-flow result must not be used as a substitute.
- Each receiver's obligation is a hard sink capacity. Max-flow may choose any feasible distribution; this is not a fairness objective and does not claim a unique schedule.
- The returned segment-slot, transfer-link and receiver-slot allocations expose the constructive flow witness. The min-cut is a certificate for the supplied network only. Neither result diagnoses physical root cause nor proves that the capacity inputs are calibrated.

## Frozen known-truth example

`scenario.synthetic-example.json` contains 3 vehicles, 2 intervals and 4 segment-time supply nodes. Total minimum service is 16 mWh. The exact oracle should return 16 mWh of max flow, zero shortfall and a 16 mWh minimum cut.

## Validation tests

The Rust test suite covers:
- exact feasible service for all three vehicles;
- shared segment-energy shortage and exact min-cut;
- transfer-link capacity binding;
- missing supply / unknown segment rejection;
- a late vehicle departure that cannot borrow energy from a later interval;
- duplicate identities and unknown properties;
- half-open service windows;
- zero-valued hard capacity bounds;
- integer overflow rejection.

## Claim ceiling

This is a **synthetic exact feasibility oracle** for supplied integer capacities. It does not estimate segment capacities, model continuous coil dynamics, establish robust feasibility under uncertainty, optimize service fairness or operating cost, prove converter/grid stability, qualify exposure/EMC, or authorize any energized test. Synthetic parity is not measured route energy service or field performance.
