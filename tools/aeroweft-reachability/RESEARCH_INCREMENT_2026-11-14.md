# AeroWeft reachability research increment — 14 November 2026

This note binds the reachability workspace to the current multi-track benchmark programme.

## Independent analytic-extrema integration oracle

The test at tests/independent_analytic_extrema_oracle.rs checks the continuous occupancy hull for two contiguous constant-velocity slices on all three axes. It derives endpoint extrema analytically from the declared interval bounds rather than relying only on a sampled grid. The fixture includes positive, negative and sign-changing velocity intervals.

This qualifies the declared bounded constant-velocity slice arithmetic only. It does not validate aircraft equations, bank-angle dynamics, airspeed, wind evolution, a physical aircraft, or any real operating envelope.

## Additional aviation benchmark tracks

- Track H: infrastructure recovery portfolios and common-cause exposure (#5224).
- Track I: reciprocal recoverability and option externalities (#5207).
- Track J: endogenous demand, queueing and service-quality feedback (#5248).
- Track K: finite-buffer airside blocking and queue spillback (#5261).
- Track L: PNT/clock evidence to hard aircraft occupancy bounds (#5270).
- Track M: cooperative/non-cooperative surveillance observability (#5275).

## 2026 research references now in the benchmark suite

- Lyu & Feng (2026), MNL demand + multi-server queueing + vertiport/vertipad location/capacity planning, DOI 10.1016/j.tra.2026.105027.
- Qin et al. (2026), bilevel vertiport capacity/commuter behaviour, DOI 10.1016/j.tre.2026.105103.
- Gottschalg et al. (2026), risk-averse two-stage site/fleet recourse under demand uncertainty, DOI 10.1016/j.tra.2026.104957.
- Dubai integrated site/fleet/routing/recharging MIP, DOI 10.1016/j.trip.2026.101945.
- Salt Lake City parcel-level siting, DOI 10.1016/j.cities.2026.107255.
- EUROCONTROL GUID-203 (5 November 2026), GNSS degradation preparedness/detection/coordination/response/recovery.
- Albaidhani (24 October 2026), measurement-limited civil-aviation GNSS spoofing benchmark, DOI 10.1016/j.nexres.2026.102604.
- 2025 ADS-B spoofing TDoA multilateration with receiver-network redundancy, DOI 10.1016/j.trpro.2025.10.010.
- ASTM F3442-25 and FAA cooperative/non-cooperative DAA evaluation principles.
- EUROCAE ED-267 OSED for very-low-level DAA and NASA EO/IR surveillance range research.

These are established baselines. AeroWeft must distinguish literature-level feature comparison, a transparent local implementation of a model class, and a faithful reproduction with exact source/data/parameters/solver pinned.

## South African regulatory source semantics

Sol-Atlas now explicitly keeps BVLOS (visual line-of-sight rule) distinct from BRLOS (radio-link scope). A conditional Part 101 B-VLOS approval path is not a blanket BVLOS permission and does not imply BRLOS authorization. Private-operation restrictions introduced in 2026 remain scoped to private operations.

## Verification status

The dedicated Cargo workflow is the authority for exact-head format, Clippy and test results. Authored source/tests are not a pass. Pending, queued, cancelled, skipped or absent workflow status is not a pass.
