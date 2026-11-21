# Maritime Backlog Ledger — Industry Comparator Register

**Research snapshot:** 21 November 2026. Published metrics below retain each source's population, boundaries and maturity; they are not assumed comparable with a synthetic fixture or South African local aggregate.

## 1. Measurement and interoperability standards

| Reference | Existing contribution | Use in this programme | Important boundary |
|---|---|---|---|
| [World Bank Container Terminal Performance Improvement Guide (2026)](https://documents.worldbank.org/en/publication/documents-reports/documentdetail/099103026152033883) | Process definitions for vessel, berth, crane, yard, gate, labour, maintenance and port-call measurement | Align event boundaries and stratify terminal versus port-level metrics | Some port-call metrics extend beyond the terminal operator's control |
| [World Bank / S&P Global CPPI 2025](https://www.worldbank.org/en/topic/transport/publication/cppi) | Global container-ship time-in-port benchmark | External port-call comparator after data/definition matching | Not rail, gate-to-door or queue stock accounting |
| [DCSA Port Call 2.0.0](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope) | Estimated/requested/planned/actual port-call timestamp exchange; typed service/facility context | Semantic admission and provider interoperability | Does not itself create a terminal inventory ledger or optimizer |
| [DCSA Industry Blueprint — Vessel Journey](https://reference.dcsa.org/content/standards/industry-blueprint/current/industry-blueprint-vessel-journey) | Timestamps, move forecasts and weekly berth-planning process | Place queue and backlog events in the wider vessel/terminal information flow | Move forecasts are not final cargo execution or binding commitments |

The World Bank's 2026 guide reports the 2025 CPPI dataset's overall arithmetic averages: 44.8 container moves per port hour, 56.3 per berth hour, crane intensity 3, gross STS productivity 23.5 moves per gross crane hour, 16.4 hours arrival time, 12.3 hours waiting and 4.1 hours steam-in. These are descriptive averages, not universal targets. The guide explicitly warns that port-level indicators should not be interpreted as terminal performance alone.

## 2. Current research algorithm comparators

| Publication | Algorithm / integrated problem | Reproduction target |
|---|---|---|
| Li & Hu, *Journal of Cleaner Production* (18 Aug 2026), [DOI 10.1016/j.jclepro.2026.149105](https://doi.org/10.1016/j.jclepro.2026.149105) | Integrated continuous berth, quay-crane, yard-crane, onshore power and multi-source energy dispatch; MILP reformulation; centralized, two-stage and ADMM coordination | Match berth, crane, OPS, storage, renewable and carbon boundaries; compare full centralized model against decomposed coordination on identical instances |
| *Computers & Industrial Engineering* (March 2026), [DOI 10.1016/j.cie.2026.111815](https://doi.org/10.1016/j.cie.2026.111815) | Deep-reinforcement-learning-enhanced branch-and-price for berth/quay-crane/yard assignment | Reproduce the same BQCYAP formulation, instances, commercial-solver baseline and compute budget |
| *Computers & Industrial Engineering* (August 2026), [DOI 10.1016/j.cie.2026.112092](https://doi.org/10.1016/j.cie.2026.112092) | Two-stage robust integrated berth/quay-crane/yard optimization with column-and-constraint generation | Match uncertainty sets, recourse, objective and stopping criteria before comparing |
| Published port queue/berth/cargo scheduling families | FCFS, berth-allocation heuristics, MILP/CP-SAT, robust and stochastic rolling-horizon models, NSGA-II / multi-objective evolutionary methods | Use only after the real input data, constraint set, objective vector and compute budget are matched |

The integrated energy paper reports a case-specific centralized optimization cost reduction up to 19.87% and notes energy cost above 59% of its modeled total. Those figures are not transferred to Durban and are not benchmark targets without reproducing its exact system and boundaries.

## 3. How Luminous should compete

Our initial advantage should not be a claim of beating a terminal optimizer at its own scheduling problem. It should be **cross-boundary integrity and outcome accounting**:

1. Preserve source time, revision, classification, facility, unit and observation cutoff.
2. Reconcile physical vessel and cargo stocks before reporting productivity.
3. Expose whether queue reduction is a true reduction, a diversion, a cancellation or a population-definition change.
4. Evaluate all plans with an independent stock-flow/constraint auditor.
5. Compose vessel, berth, yard, energy, gate, rail and inland service without assuming one operator owns all resources.
6. Preserve participant authority and source rights.
7. Measure delivered cargo and recovery—not only local throughput.

A fair SOTA programme compares:
- current operating policy / observed-history replay;
- FCFS and published terminal priority rules;
- exact enumeration on tiny instances;
- deterministic MILP or CP-SAT;
- robust/two-stage optimization;
- branch-and-price or equivalent decomposition for large integrated instances;
- RL-enhanced search and evolutionary multi-objective methods where source code/formulations are reproducible;
- Luminous evidence-qualified coordination.

All arms must share the same event cut-off, demand, equipment capability, resource calendars, disruption tape, objective boundaries and stopping/compute budget.

## 4. Proposed targets (research targets, not achieved results)

| Dimension | Advancement target |
|---|---|
| Ledger correctness | 100% frozen known-answer match; zero unexplained stock-flow residual |
| Small-instance optimization | Exact agreement with an independent enumerator |
| Evidence integrity | Zero accepted future-dated, conflicted, duplicate-effect, wrong-unit or wrong-terminal events |
| Queue performance | Lower vessel-hours in queue and p90/p95 waiting on matched held-out weeks |
| Inventory | Lower box-hours of inventory without missed load windows or hidden diversion/cancellation |
| Service | Higher share of cargo delivered inside declared door-to-door windows |
| Recovery | Lower time to restore backlog below a pre-registered threshold after paired disruptions |
| Fairness | Publish waiting/delivery distributions by operator, vessel/service/cargo class and inland destination |
| Energy / carbon | Lower energy and GHG per delivered service unit at matched metering boundaries |
| Compute | Publish runtime and quality-versus-time curves on declared hardware; no unqualified universal latency claim |

No numeric performance gain is claimed by the synthetic accounting tool. Real improvement claims require lawful event data and matched independent validation.

## 5. Existing Luminous owners to compose

- Symtropy #4007 — queue/backlog accounting and recovery programme.
- Symtropy #1930 / #1940 and #1955 / #1965 — PCO/DCSA event semantics.
- Symtropy #1858 / #1867 — JIT arrival and contingent recovery.
- Symtropy #3683 / #3705 — vessel-to-inland service-chain contract.
- Symtropy #4003 — Durban Gateway Terminal source admission plan.
- Symtropy #3243 / #1631 and #6473 / PR #6474 — South African freight/corridor sources and aggregate train scorecard.
- Mycelix #5974 / PR #5980 — cross-owner authority, handoff and golden-trace work.
- Sol-Atlas #654 and #380 — spatial evidence projection and ex-post port/corridor outcome evaluation.
- Symthaea #7444 / PR #7629 — vessel engineering and marine-freight evidence screening.

These are separate owner-native workstreams. This register is a comparison plan, not evidence that all components are already integrated or operational.
