# SAFRB temporal assurance and adversarial qualification

**Work item:** [SAFRB-TEMPORAL-ASSURANCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/7336)  
**Parent implementation:** [SAFRB v0.8 PR #7294](https://github.com/Luminous-Dynamics/symtropy/pull/7294)  
**Research cut:** 24 November 2026  
**Status:** deterministic test implementation; field and electrical-domain validation remain separate

## 1. Why example tests are not enough

The v0.8 evaluator is a temporal state machine. Its most consequential errors would be semantic, not arithmetic syntax errors: prematurely declaring restoration, losing repeated outage episodes, confusing uncertain service with confirmed loss, or changing reported durations when the input sampling convention changes.

This tranche adds a dependency-free, independent reference trace over the evaluator's three-valued window alphabet:
- confirmed shortfall;
- uncertain against the critical floor;
- guaranteed at or above the critical floor.

For every sequence length 1–6, all (3^n) possible sequences are evaluated at three distinct sustained-restoration thresholds. That is 1,092 sequences and 3,276 evaluator/oracle comparisons. In addition, the widening invariant enumerates 5,151 ordered delivery intervals, and the improvement invariant checks four monotone delivery shifts for each interval (20,604 comparisons). The oracle consumes only the discrete state alphabet; it does not consume the production assessment objects or `EpisodeBuilder` implementation.

## 2. Metamorphic invariants

| Invariant | Required behavior |
|---|---|
| Interval widening | Less precise delivery bounds must not improve a continuity disposition. |
| Delivery improvement | Raising both delivered lower and upper bounds must not worsen the disposition or increase shortfall bounds. |
| Time-origin translation | Adding a constant to every timestamp must preserve all durations, classifications and energy results. |
| Equivalent partitioning | Splitting a constant-rate interval into equivalent adjacent subwindows must preserve aggregate energy shortfall, guaranteed time and restoration episode timing. |
| Recovery threshold | A streak exactly equal to the configured duration may confirm at its observed end; one second short remains censored. |
| Coarse sampling | Confirmation is timestamped at the end of the sample that proves the threshold, never interpolated earlier without finer evidence. |
| Repeated shocks | A later outage after a confirmed recovery creates a distinct episode. |

## 3. Industry comparison: report different dimensions together

IEEE 1366-2022 remains the active distribution reliability-index guide. EIA publishes SAIDI, SAIFI and CAIDI and explains their customer-weighted meanings. They remain essential comparators where customer interruption records and valid denominators exist.

SAFRB should report a complementary vector:
- customer-weighted SAIDI / SAIFI / CAIDI where applicable;
- energy not supplied (EENS/AENS/EUE) at an explicitly named boundary;
- critical-floor shortfall lower/upper bounds;
- guaranteed service-hours and uncertain service-hours;
- time to first recovery candidate and time to confirmed sustained recovery;
- right-censored episodes and restoration failures;
- worst-cohort, p95 and p99 outcomes;
- evidence freshness, missing/failed observations and common-cause groups.

Do not infer customer-weighted reliability indices from aggregate Wh. Do not collapse customer interruption time, energy shortfall and service quality into one composite score.

## 4. SOTA and research boundary

Mandatory external comparisons already captured by Power Fabric #6320 should remain distinct:
- LARA (EACL Findings 2026): constrained LLM planning against feeder restoration baselines.
- FLARE (arXiv:2610.23251): degraded-observation feeder restoration with an external physical guard.
- 2026 distributionally robust IEEE-118 whole-process restoration: black-start, network reconfiguration, flexible loads and independent AC-OPF checks.
- 2026 benchmark-model review: dynamic restoration phenomena, including EMT/RMS study pathways.

A separate 2026 review of renewable bottom-up black-start emphasizes that installed capacity is not equivalent to deliverable inverter headroom. Restoration admissibility also depends on active/apparent-power limits, low-inertia frequency dynamics, cold-load pickup, protection, and imperfect situational awareness.

The SAFRB v0.8 evaluator does **not** model those electrical dynamics. Its current measured quantity is Wh delivered at one named boundary. The certificate is an energy-floor result only; it is not proof of instantaneous kW adequacy, voltage/frequency compliance, waveform quality, protection coordination, AC feasibility or black-start safety.

## 5. Paradigm-shift agenda

### A. Move from a scalar service score to a non-compensatory service vector

For an electricity service, future qualification should preserve at least:
`(E_{Wh}, P_{kW}, V_{pu}, f_{Hz}, Q_{var}, uptime, quality, evidence)`

For water, the equivalent vector would contain delivered volume, pressure, quality and continuity. Telecom would require availability, throughput, latency and quality. Do not combine unlike units; every mandatory predicate must pass independently.

### B. Certifiable restoration envelope

A planner should propose a sequence. Independent engineering validators should determine whether each transition remains inside a qualified envelope:
- topology and source reachability;
- active and apparent inverter headroom;
- minimum and maximum voltage;
- frequency and rate-of-change-of-frequency limits;
- cold-load pickup and transient response;
- protection selectivity and fault clearing;
- synchronization and island-merge limits;
- service-floor and beneficiary constraints.

A planner's score or successful action response must never self-certify a physical transition.

### C. Active observability as a constrained resource

Integrate Symthaea EVSI only as a decision-value and measurement-request proposal. Count acquisition cost, result latency, source freshness, shared crews/tools and source dependence. A sensing request is not a result; a result is not authority; authority is not physical feasibility.

### D. Preserve compensation versus mechanism restoration

A temporary battery or generator may maintain the service while the original grid/feed/plant remains unavailable. Report both:
1. functional service continuity; and
2. restoration of the original mechanism or a qualified permanent replacement.

### E. Common-cause and repeated-shock resilience

Add compound supplier, berth, port, pipeline, grid, communications and weather shocks with explicit dependency groups. Measure repeated-shock viability, not just a single recovery episode.

## 6. Proposed competitive targets

These are preregistered research hypotheses, not achieved results:

| Dimension | Target |
|---|---|
| Temporal oracle | 100% agreement over the bounded exhaustive state corpus |
| Interval honesty | Zero continuity upgrades when evidence bounds widen |
| Restoration timing | Zero confirmation before a full observed sustained-recovery duration |
| Clock/partition invariance | Exact equality for the tested transformations |
| Physical feasibility | Zero accepted transitions that fail an independent domain validator |
| Service tails | At least 10% lower CVaR95 critical-service shortfall than the strongest matched baseline, at equal cost and identical hard constraints |
| Recovery | At least 10% lower p95 time-to-sustained-recovery without worsening worst-cohort service floors |
| Measurement efficiency | Lower decision regret per unit of sensing time/cost, with all denied, stale, timed-out and failed requests in the denominator |
| Operational usability | Report p50/p95 end-to-end latency, evidence completeness, false rejects, overrides and verifier cost |

No claim of superiority should be made until matched datasets, exact model versions, independent validators, paired scenario seeds and complete failure accounting are in place.

## 7. Sources

- IEEE SA, IEEE 1366-2022, active distribution reliability-index guide: https://standards.ieee.org/ieee/1366/7243/
- U.S. EIA, Electric Power Annual Table 11.1, SAIDI/SAIFI/CAIDI definitions and reporting: https://www.eia.gov/electricity/Annual/table.php?t=epa_11_01.html
- LARA, Findings of EACL 2026: https://aclanthology.org/2026.findings-eacl.321/
- FLARE, arXiv:2610.23251 (16 October 2026): https://arxiv.org/abs/2610.23251
- Lin & Xu, Distributionally Robust Whole-Process Restoration..., Applied Sciences 16(21), 10584 (2026): https://www.mdpi.com/2076-3417/16/21/10584
- Review, Power distribution system blackstart restoration using renewable energy, Nature Communications (2026): https://www.nature.com/articles/s41467-026-76692-0
- Review, A benchmark model for power system restoration studies, Renewable and Sustainable Energy Reviews 232 (May 2026): https://www.sciencedirect.com/science/article/pii/S1364032126000869
