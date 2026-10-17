# Accessible Home Intervention Portfolio Benchmark Plan v0.1

**State:** contract and algorithm proposal with synthetic test data; no real project has been optimized or assessed.

## Why this direction

Home-modification research has historically focused heavily on fall prevention and physical safety. A 2025 review of 20 studies highlights gaps in personalization, smart-home technology, long-term adaptation, social participation and cost-effectiveness; it also notes limitations in the cost evidence. A 2025 scoping review of 38 articles on outcomes for adults aged 50+ and relatives found 16 outcomes but only four studies reporting the costs of installed modifications. A 2023 systematic review of 12 randomized trials in older adults found heterogeneous models and argued for more integrated intervention approaches. These are research signals—not evidence that this planner is effective, and not findings that can automatically generalize beyond the populations studied.

Sources:
- [Cha (2025), A Systematic Review of Home Modifications for Aging in Place in Older Adults](https://pubmed.ncbi.nlm.nih.gov/40218050/)
- [Home Modification Outcomes for Adults Aged 50 Years and Over and Their Relatives: A Scoping Review](https://pubmed.ncbi.nlm.nih.gov/40817791/)
- [Sheth & Cogle (2023), Home Modifications for Older Adults: A Systematic Review](https://pubmed.ncbi.nlm.nih.gov/36655622/)
- [WHO, Housing and health guidelines accessibility review](https://www.ncbi.nlm.nih.gov/books/NBK535292/)

## Paradigm shifts

1. **One-shot recommendation → staged intervention portfolio.** Compare sequences and prerequisite relationships, not isolated products.
2. **Weighted score → constraint gates plus Pareto frontier.** Hard constraints remain non-negotiable; cost, annual maintenance, lead time, disruption and preference alignment are displayed as distinct dimensions.
3. **Exact price fantasy → ranges, provenance and unknowns.** The planner refuses to silently treat missing quotes as zero, and a range crossing the budget cap remains uncertain.
4. **Cheapest upfront → lifecycle burden.** Track recurring maintenance separately from capital cost, and later expand to energy, consumables, replacement cycles, resident/carer time and repair latency when reliable scoped evidence exists.
5. **Ignore the housing system → permission and implementation gates.** A technically plausible intervention requiring unavailable landlord permission is not reported as budget-feasible.
6. **Plan equals outcome → stage-specific evidence.** Proposed, quoted, funded, installed, measured and professionally reviewed remain separate; a planned remediation is never counted as proof that a hard constraint is currently satisfied.
7. **Universal preference score → visible resident-specific trade-offs.** Preferences remain optimization dimensions, not gates that can override hard requirements. The same numeric disruption rubric is comparable only within the declared context.
8. **Opaque optimization → reproducible decision support.** Enumerate a small bounded space exactly, report why alternatives are excluded, preserve input hashes and publish mutation cases before scaling up.

## Initial software gates

| Gate | Proposed threshold | Evidence |
|---|---|---|
| Hard constraint coverage | 100% of frontier plans cover every unresolved hard constraint with at least one explicitly declared remediation option | Synthetic mutation corpus |
| Cost unknown handling | 100% of candidates requiring unknown cost remain outside the budget frontier | Unknown-cost negative controls |
| Budget uncertainty | 100% of ranges with low <= cap < high are separated from robustly within-budget candidates | Boundary mutations |
| Dependencies | 100% of cycles and missing dependency references fail closed | Graph mutations |
| Access gates | 100% of plans selecting a pending/denied gate are excluded from budget frontier | Permission mutations |
| Pareto correctness | 0 strictly dominated plans in the reported frontier | Pairwise independent frontier checker |
| Determinism | Identical bytes and pinned build produce an identical receipt | Exact-input digest + byte comparison |
| No false qualification | 100% of plan reports retain the synthetic-only limitation and report planned remedies as unverified | Receipt assertions |

## Scale and performance

The first evaluator enumerates all subsets for at most 20 interventions (maximum 1,048,576 subsets). This is deliberately auditable but exponential, so do not raise the cap without benchmarks.

Measure 5, 10, 15 and 20 interventions; vary hard-constraint coverage, dependency density, preference conflict, permission gates and proportion of unknown quotes. Use 30 timed repetitions after warm-up; report median/p95 runtime, peak RSS, hardware/OS, Rust compiler, build profile, input byte size/hash and frontier cardinality. The scale target remains open until baseline runs exist. Compare an independent exhaustive reference implementation before considering branch-and-bound or integer programming.

## Expansion path

- v0.1: synthetic, exact subset search and Pareto frontier.
- v0.2: staged funding schedules, quote freshness/expiry, implementation state transitions and sensitivity scenarios.
- v0.3: interface mapping to the typed task-network and constraint contracts; model resident-confirmed acceptable alternatives and professional review requirements.
- Research gate: current evidence search, disabled-resident co-design, domain review and cost provenance before collecting real inputs or claiming impact.

Any real implementation must be jurisdiction-aware and resident-directed. This is not a building-code interpretation, safety certificate, quote, funding guarantee or substitute for a qualified practitioner.
