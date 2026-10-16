# Accessible Home Benchmark Plan v0.1

**Status:** proposed research and engineering gates; no field results have been collected.  
**Scope:** design-time tooling and future resident-directed evaluation. This is not a building-code interpretation or home qualification process.

## Research basis and limits

The WHO's 2018 Housing and Health Guidelines recommends that an adequate proportion of housing be accessible to people with functional impairments and summarizes outcomes including injury, well-being, mental health, dependency on care services and social participation. Its underlying accessibility review searched literature only through April 2018 and explicitly called for standardized longitudinal research. [WHO Housing and health guidelines](https://www.who.int/publications/i/item/9789241550376); [systematic review chapter](https://www.ncbi.nlm.nih.gov/books/NBK535292/).

The field has moved beyond that review, but the evidence still has important gaps. A 2025 systematic review of 20 studies on home modifications for older adults reports fewer studies on personalized intervention, smart-home technology, long-term adaptability, social engagement and cost-effectiveness. Its population and narrative-synthesis approach mean that its results should not be generalized automatically to children or all disability groups. [Cha, 2025, PubMed](https://pubmed.ncbi.nlm.nih.gov/40218050/). A 2023 review of 12 randomized trials in older adults found that occupational-therapist-led modifications combined with clinical, physical-activity or behavioral components were among the more successful approaches. [Sheth & Cogle, 2023, PubMed](https://pubmed.ncbi.nlm.nih.gov/36655622/).

ISO 21542:2021 covers access, circulation, egress and evacuation, including residential-unit recommendations. The current status page shows a review process; verify the current edition, local adoption and applicable law with qualified professionals before a real project. [ISO 21542 official status page](https://www.iso.org/standard/71860.html).

These sources motivate the benchmark design; they do not establish that this repository's implementation is effective.

## Paradigm shifts

1. **Static checklist → temporal evidence graph.** Every claim names its scope, configuration revisions, source, reviewer role and review deadline. Changes invalidate only claims that depend on changed facets. No "verified forever" badge.
2. **One accessibility score → explicit constraints plus trade-offs.** Life-safety and resident-declared hard constraints do not trade against cost or convenience. Preferences remain visible when a candidate conflicts; they are not silently promoted to hard blockers or discarded.
3. **Independence as ideology → resident-defined agency.** A resident may choose assistance, automation, privacy, redundancy or human support. Do not reward a design simply because it removes a person's chosen support or maximizes tasks completed without help.
4. **Room geometry alone → socio-technical system.** Evaluate circulation and reach alongside sensory/cognitive access, control interfaces, power, connectivity, maintenance, support arrangements, emergency fallback and affordability.
5. **Happy-path demonstration → graceful degradation.** Verify behavior under power loss, network loss, device failure, stale measurements, missing support and conflicting information. A proposed fallback is not a demonstrated fallback.
6. **One-off audit → change and maintenance lifecycle.** Treat repairs, furniture moves, changed assistive devices, tenancy changes and changing needs as events that may trigger targeted revalidation.
7. **Data collection by default → privacy-preserving proof.** Keep exact plans and resident evidence local or in explicitly authorized restricted storage. Share the smallest claim needed for a decision; do not publish sensitive plan hashes as if they were anonymous.
8. **Compliance-only → lived outcomes.** Technical checks are necessary but insufficient. A home is not demonstrated to work for a resident until resident-directed tasks, use conditions and outcomes are evaluated with consent and appropriate safeguards.

## Benchmark tiers

### Tier 0 — Contract and evaluator correctness

These are software gates; use synthetic fixtures and mutation tests.

| Measure | Initial target | Pass rule |
|---|---:|---|
| Invalid event causality | 100% rejection | Wrong starting revision, future event or out-of-order event cannot emit a valid receipt |
| Stale evidence | 100% non-passing | A required claim whose bound revision changed or whose review expired cannot be counted as current |
| Unrelated invalidation | 0 | A claim depending only on an unchanged facet remains current |
| Barrier precedence | 100% | A current hard barrier blocks even when another required claim is stale |
| Stale barrier semantics | 100% fail-closed | A barrier tied only to the old configuration does not certify or automatically block the new configuration; current path becomes not evaluable |
| Contradiction handling | 100% explicit | Conflicting current claims with the same scope and assertion key are unresolved; distinct assertions do not conflict and timestamps alone do not decide truth |
| Replay determinism | 100% exact | Same bytes and pinned build yield the same result and receipt |
| Public fixture privacy | 0 real payloads | No resident identity, address, actual floorplan, personal notes or raw sensitive evidence in public examples |

The draft Rust evaluator includes a synthetic 1,000-claim / 1,000-path correctness stress test and a separate benchmark binary that generates an exact 10,000-claim / 1,000-event / 1,000-path workload, reports median/p95 timings and binds results to an input digest. Neither has been reported as passed; the benchmark records OS/architecture while compiler version, CPU details and peak RSS must be captured externally alongside the run.

### Tier 1 — Larger-scale replay and auditability

Run only after Tier 0 passes. Publish machine, OS, compiler, build mode, input generator version, input byte count, claim/event/path counts, wall-clock time, peak RSS, receipt hash and run status.

- **Proposed target:** 10,000 claims, 1,000 ordered change events and 1,000 paths replay in under 1 second and under 256 MiB peak RSS on a named reference machine. The benchmark binary times only evaluate_bytes, checks deterministic receipts across iterations, and reports minimum/median/p95/maximum time and input byte count/hash. Use /usr/bin/time -v or an equivalent for peak RSS, and archive rustc --version --verbose, CPU details and the exact release-build command.
- Repeat each run at least 30 times; report median and p95, not only the fastest run.
- Measure cold and warm runs separately; never compare unlike build modes.
- The target is a hypothesis until a baseline is actually measured. Do not claim it is met because a small fixture passes.

### Tier 2 — Physical and technical scenario validation

Use simulation and instrumented mock-ups before field use. Actual site decisions remain the responsibility of qualified professionals and the resident.

- Measure clearances, reach ranges, thresholds and turning/manoeuvre space with measurement uncertainty recorded.
- Test resident-selected tasks across relevant mobility, sensory and cognitive modalities; report success, assistance chosen, time, effort, errors and near-misses.
- Exercise normal operation and named failure modes: power loss, network outage, device failure, backup exhaustion, unavailable support and invalid/stale evidence.
- A life-safety or egress assumption must never be inferred from a synthetic graph or simulator alone; use jurisdiction-appropriate professional review and real site verification.

### Tier 3 — Resident-directed outcomes over time

Use an updated literature search and a co-designed protocol before recruiting anyone. Offer accessible consent, compensation, withdrawal and data deletion processes. Never withhold a necessary accommodation to create a control condition.

Collect a baseline and repeated follow-ups (for example, early post-implementation, then 1, 3 and 6 months; an optional 12-month follow-up should be justified by burden and study aims). These are candidate time points, not validated universal intervals.

- Resident-chosen task completion and the ability to decline/respond/override.
- Resident-reported autonomy, control, effort, satisfaction and fit to changing needs.
- Participation in personally meaningful activities, rather than only basic activities of daily living.
- Falls/injuries and near-misses where ethically and practically measurable; explicitly handle missing reports and exposure time.
- Support/caregiver burden, but do not assume less support is inherently better.
- Unintended effects and adverse events, including increased surveillance, alert fatigue, stigma or loss of privacy.

Report outcomes by task and context before any aggregate; do not compress them into one score that lets convenience compensate for a hard safety or resident-agency failure.

### Tier 4 — Cost, adaptability and access equity

Measure total cost, not purchase price alone: installation, maintenance, consumables, energy/network requirements, staff/caregiver time, repair time and replacement cycles. Track the time from a resident-identified barrier to a funded, completed and revalidated change. Study affordability and housing-tenure constraints without requiring publication of diagnosis or exact address. Use minimal, consented subgroup data and suppress small cells to reduce re-identification risk.

### Tier 5 — Privacy, consent and operational trust

- Default to local processing for sensitive plans and evidence.
- Make field-level disclosure understandable and revocable; log access without copying raw resident data into logs.
- Demonstrate that withdrawal stops future sharing and that deletion reaches the declared copies, subject to disclosed legal/operational retention duties.
- Test shared-device, backup, support-worker and accidental-publication threats.
- Distinguish integrity (bytes unchanged) from authenticity, correctness, consent and anonymity.

## Evidence and release gates

- **Gate A — Synthetic contract:** schema/reference validation, mutation tests, deterministic receipt, no real payloads.
- **Gate B — Independent review:** accessibility domain expert, privacy/security reviewer and disabled resident/co-design group review the claims and failure modes.
- **Gate C — Mock-up:** test usability and failure paths without claiming physical-site qualification.
- **Gate D — Site assessment:** field measurements and relevant professional review, with resident-authorized evidence handling.
- **Gate E — Outcome evaluation:** preregister primary outcomes and analysis, compensate participants, report adverse effects and uncertainty, and publish an accessible summary.
- **Gate F — Maintenance:** trigger scoped reassessment when a dependent configuration or a resident's chosen requirements change.

A queued CI run is not a pass. A passing evaluator is not proof of truth. A verified configuration is not a lifetime guarantee. Public benchmark reports must identify which gate was tested and which remains open.
