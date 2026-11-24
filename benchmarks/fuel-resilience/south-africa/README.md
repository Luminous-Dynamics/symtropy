# South African Fuel and Energy Resilience Benchmark (SAFRB)

**Research cut:** 24 November 2026  
**Status:** public research programme; all PRs listed below remain drafts.  
**Implementation:** Rust 2024 standalone benchmark crates; pinned Rust 1.96.0 qualification workflows.  
**Operational posture:** read-only; no PLC/DCS/SIS writes, autonomous dispatch, provider commitments or physical allocation.

## Objective

SAFRB tests whether an evidence-qualified decision layer can improve industrial fuel and energy planning by distinguishing source claims from measurements and commitments, modelling grade-specific material and multi-carrier energy constraints, comparing plans under matched scenarios, independently accounting for useful service and tail risk, and identifying which next measurement could change a decision.

The aim is not to replace AVEVA PI, SLB, PyPSA, HiGHS, Gurobi, OR-Tools or grid-control software. The research hypothesis is that evidence reconciliation, capacity-rights separation, independent physical/service verification and decision-focused evidence acquisition can improve the inputs and assurance around established tools.

## Versioned implementation

v0.2–v0.7 have successful dedicated exact-head Rust workflows covering formatting, tests, Clippy and the known-answer executable. Test counts are unit tests. v0.2 also passed the repository-level CI workflow at its listed head. v0.8 is a new implementation increment; its exact-head qualification is pending and no pass is claimed.

| Version | Public draft PR | Increment | Tests | Exact-head result |
|---|---|---|---:|---|
| v0.2 | [#7180 — grade-aware fuel accounting](https://github.com/Luminous-Dynamics/symtropy/pull/7180) | Evidence-qualified grade-specific inventory, cargo milestones, mass balance, source and draft-policy audit. | 9 | PASS |
| v0.3 | [#7183 — min-cost-flow baseline](https://github.com/Luminous-Dynamics/symtropy/pull/7183) | Deterministic time-expanded single-grade logistics optimizer with route, storage, eligibility, deadline and priority constraints. | 9 | PASS |
| v0.4 | [#7185 — risk tournament](https://github.com/Luminous-Dynamics/symtropy/pull/7185) | Expected loss, VaR/CVaR, zone-level tail risk, matched comparisons and distributionally robust probability ambiguity. | 13 | PASS |
| v0.5 | [#7205 — fuel-to-useful-energy service](https://github.com/Luminous-Dynamics/symtropy/pull/7205) | Batch LHV and generator-efficiency intervals, nameplate/auxiliary limits, net Wh, service shortfall and EVSI fixture. | 14 | PASS |
| v0.6 | [#7215 — multi-carrier service portfolio](https://github.com/Luminous-Dynamics/symtropy/pull/7215) | Grid/shore/generator/renewable sources, battery SOC/power bounds, priority service, curtailment and exact bus-energy balance. | 14 | PASS |
| v0.7 | [#7243 — physical capacity vs access rights](https://github.com/Luminous-Dynamics/symtropy/pull/7243) | Installed/operable capacity, entitlement, provider offer, existing reservation/commitment/delivery and residual capacity. | 15 | PASS |
| v0.8 | [service continuity and restoration branch](https://github.com/Luminous-Dynamics/symtropy/tree/research/safrb-v0.8-service-restoration) | Time-bounded critical-service continuity, bounded shortfall, sustained restoration and right-censored recovery. | 20 | QUALIFICATION PENDING |

**Qualification boundary:** PASS establishes software behavior for the supplied synthetic fixtures and declared evidence contracts. It does not establish actual South African stock, IVP/MPP throughput, equipment performance, national resilience, economic impact, legal entitlements or field prediction accuracy.

## South African evidence findings and unresolved conflicts

### Demand and grade-specific accounting

The Department of Electricity and Energy 2024 market report presents rounded grade-specific diesel figures of approximately 12.0 billion litres D50 and 273 million litres D500, summing to approximately 12.273 billion litres. The same report's aggregate chart gives about 11.8 billion litres, also repeated in a Stats SA Mbalo brief. These figures are not silently reconciled.

A Q2 2026 trade report gives 3.3 billion litres of diesel imports and origin shares: India 27%, United States 21%, Oman 15%, Togo 9% and Netherlands 5%. The five published country-volume figures sum to 2.569 billion litres, while 77% of the rounded 3.3-billion-litre total is 2.541 billion litres: a 28-million-litre mismatch. SAFRB preserves source-reported shares and volumes separately.

The 2024 trade report has two diesel export figures in different sections (790 million vs 276 million litres). This remains an unresolved source conflict.

### National usable inventory is not established

The public sources reviewed do not establish a consolidated, independently verified national total of immediately usable South African diesel by grade, terminal/tank boundary, quality release and physical accessibility. SAFRB therefore does not publish national “days of diesel remaining” from incomplete or mixed categories.

The Minister stated in Parliament on 25 March 2026 that the Strategic Fuel Fund held eight million barrels of strategic fuel stock. The answer does not specify crude/refined split, diesel grade, measurement date, tank location, quality release, accessibility or withdrawal rate. This remains an attributed official statement, not an audited diesel inventory.

The Minister stated on 19 May 2026 that supply was secured and no near-term shortage was expected. The SARB's September 2026 analysis separately reports refining capacity had halved over the prior decade and imported refined products now supply more than half of domestic fuel demand. These statements describe different concepts and time horizons: current availability is not the same as structural import exposure.

### Draft Strategic Petroleum Stocks Policy

The July 2026 Gazette draft contains material internal conflicts:
- its executive summary states 90 state days and 14 private refined-product days;
- its detailed table states 60 state days and 21 private days, with a 70/30 crude/refined-product split;
- Section 7.1 changes annual petroleum demand from 27 billion litres in the introduction to 30 million litres in the disruption-cost section;
- the shortfall and sectoral/economic arithmetic in Section 7.1 does not reconcile.

The public-comment deadline was 9 August 2026. S&P Global reported on 15 October 2026 that consultation had concluded but Cabinet approval was still awaited. No final instrument was verified in the sources reviewed by this research cut. Draft values remain policy scenarios, not effective reserve requirements or evidence of actual stock.

### Island View Precinct rights and refinery restoration

Parliament's August 2026 Island View Precinct statement reports a 10% capacity-access allocation for new participants, around 13% operator capacity commitments and a later-phase 27.8% participation target. These are different access/contract/future-phase statements. Their denominators and time scopes are not established as identical, so they must not be added or treated as throughput.

CEF's September 2026 presentation describes Mossel Bay GTL restoration targets of approximately 18,000 b/d for phase 1 and 46,000 b/d for phase 2, with stated investments. The refinery has been offline since November 2020; these are future project targets, not current production. The source wording does not establish whether 46,000 b/d is incremental or total phase-2 output.

Detailed sources and claim ceilings:
- [Source register](v0.2/research/SOURCES.md)
- [Draft policy consistency audit](v0.2/research/POLICY_CONSISTENCY_AUDIT.md)
- [Capacity rights and restoration audit](v0.2/research/CAPACITY_RIGHTS_AND_RESTORATION.md)

## Cross-stack ownership

- **Sol-Atlas:** spatial/temporal context, source coverage, rights-aware assets, provenance and uncertainty.
- **Symtropy:** fuel/network/service constraints, deterministic simulation, optimization adapters, resource feasibility and independent replay.
- **Mycelix:** identity, recognition, access entitlement, provider offers, reservations, commitments, authority receipts and reconciliation.
- **Symthaea:** contradictions, competing hypotheses, active evidence acquisition, decision regret, bounded alternatives and abstention.
- **Authorized human/provider/operator:** retains operational and legal authority.

This is a logical ownership map, not a mandatory synchronous deployment. A map feature is not proof of a free berth; a signed statement is not proof of physical inventory; a reservation is not delivery; a forecast is not a measurement; a model output is not an authority receipt.

Related existing work was linked rather than duplicated: Mycelix #5762, #5920, #5932, #4574, #5968 and #4814; Sol-Atlas #942, #895, #654, #302, #332, #350, #293 and #315; Symthaea #12403, #12515, #13365 and #12173.

## Industry and benchmark strategy

The [v0.6 solver tournament protocol](v0.6/research/SOLVER_TOURNAMENT_PROTOCOL.md) preregisters matched comparisons against:
- PyPSA 1.4.0 for energy-system optimization and stochastic/CVaR planning;
- HiGHS 1.16.0 for open LP/MILP;
- Gurobi 14.0 for commercial optimization where licensed;
- OR-Tools for the single-commodity min-cost-flow baseline;
- simple greedy/do-nothing policies;
- Symthaea decision support and human planners;
- exact enumeration as an upper bound on tiny cases only.

Metrics remain separate: optimizer feasibility/objective/gap/runtime, physical mass/energy conservation, service shortfall, CVaR95, geographic service distribution, source eligibility, decision regret, EVSI, abstention and auditability. There is no composite leaderboard score.

The proposed future gate of at least 10% lower CVaR95 unserved critical-service energy than a matched deterministic baseline is aspirational and has not been achieved. No industry head-to-head, calibrated field prediction or independent physical plant validation has been completed.

## Immediate next validation gates

1. Obtain permissioned, time-aligned terminal/port/pipeline and stock evidence with measurement uncertainty, product grade, tank boundaries and quality/accessibility.
2. Reconcile vessel nomination/loading/departure/arrival/discharge events to terminal receipt, quality release and inland transfers.
3. Obtain source-pinned MPP/terminal engineering operating envelopes; do not infer hydraulic capacity from draft-policy prose.
4. Obtain Mycelix-verified provider entitlement/offer/commitment receipts and exact resource/period denominators.
5. Validate generator LHV, efficiency and auxiliary-load envelopes against independent laboratory or field measurements.
6. Build a matched PyPSA/HiGHS/Gurobi case with identical evidence, constraints, objective hierarchy and hardware budget; independently audit schedules.
7. Run a read-only shadow workflow with a terminal/operator partner; no operational control authority.

Until these gates are met, SAFRB remains a synthetic software benchmark and public-source audit, not an operational national fuel dashboard or forecast.
