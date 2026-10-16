# ACCESS-HOME-CONSTRAINTS-001 — typed resident constraints v0.3

**Status:** draft synthetic contract and evaluator extension. No real home or device is qualified by these artifacts.

## Paradigm shift

A list of features cannot show that a plan fits the person who will live with it. v0.2 made task alternatives and failure conditions explicit, but represented resident constraints as free text. v0.3 turns constraints into stable, typed claims and records how each candidate path relates to each claim.

Keep separate dimensions, not a universal accessibility score:
- **Path evidence:** declared evidence for required nodes and edges.
- **Hard constraints:** non-negotiable conditions defined by the resident.
- **Preferences:** desired qualities and trade-offs that should remain visible even when they do not hard-block a path.
- **Applicability:** the operating conditions under which a constraint is relevant.
- **Uncertainty:** gaps, conflicting findings and who needs to review them.

## Contract files

- `accessible-home-task-network-v0.3.schema.json`: typed contract for resident constraints and path-specific findings.
- `accessible-home-task-network-v0.3.example.json`: synthetic visitor-notification goal with network-dependent and local tactile paths.

## Typed constraint model

Each goal constraint has:
- stable `id`;
- `kind`: `hard` or `preference`;
- a resident-oriented statement;
- `applies_when`: the operating conditions to which it applies;
- source reference, review role and contextual note.

Every path contains exactly one `constraint_finding` per goal constraint. Each finding references the constraint ID and records a disposition (`satisfied_for_fixture`, `conflicted`, `unresolved`, or `not_applicable`), its evidence state, reference, review role and explanatory note. `not_applicable` is valid only where the path's condition is absent from the constraint's `applies_when`; conversely, an applicable constraint must not be marked `not_applicable`.

The semantic validator uses three independent assessments rather than one flattened score:
1. path task evidence (nodes and edges);
2. hard-constraint disposition;
3. preference conflict/unresolved counts.

## Proposed path semantics

- A required node or edge with a known barrier blocks the path.
- A hard constraint that is explicitly conflicted and supported by declared-scope verified evidence (or an explicit known-barrier state) blocks that path.
- A hard constraint that is unresolved, or whose claimed satisfaction lacks verified-for-declared-scope evidence, makes the path not-evaluable.
- A hard constraint is satisfied for the fixture only if the finding says so *and* evidence state is `verified_for_declared_scope`. The evaluator must not infer this from prose.
- A preference conflict never silently becomes a hard blocker. It is returned as a visible finding and counted in the path receipt. Unresolved preferences are also visible.
- A finding for an inapplicable constraint must say `not_applicable`; an applicable constraint must have a substantive disposition.
- A failure-mode candidate can be claimed only when an activated path is fixture-complete after task evidence and hard-constraint checks. Preference trade-offs remain in the report and must not be hidden in a UI rank.
- Evidence-complete means synthetic fixture consistency only. It is not field verification, construction approval or a safety/compliance determination.

## Synthetic example interpretation

The normal, network-dependent path remains `not_evaluable` because delivery is proposed and needs site measurement. It also records a declared conflict with the tactile-notification preference. The network-unavailable local path is `evidence_complete_for_fixture` and satisfies both hard constraints in the example. The preference conflict on the primary path is surfaced without being turned into a hard constraint. No actual alarm, notification device, power backup, route or home was tested.

## Benchmark ladder

### Deterministic contract and evaluator
- 100% of paths have exactly one constraint finding for each declared constraint.
- 100% of missing, duplicate or unknown constraint IDs and duplicate findings fail closed.
- 100% of out-of-condition `not_applicable` findings and in-condition `not_applicable` findings are rejected.
- 100% of known hard conflicts with sufficiently explicit evidence block the affected path.
- 100% of hard constraints that are unresolved or not adequately evidenced remain not-evaluable.
- 100% of preference conflicts and unresolved preferences survive into the receipt; zero preference conflicts are silently reclassified as hard blockers.
- Required evidence downgrades cannot improve a path's assessment or leave a stale failure-mode candidate claim intact.
- Same exact input bytes yield identical SHA-256 receipt and canonical report on a pinned build.
- Zero synthetic results are described as proof of real-home accessibility, safety, usability or statutory compliance.

### Human outcomes, only after compensated co-design
Co-design tasks, constraint wording, acceptable alternatives and thresholds with participants with intersecting access needs. Report outcomes separately rather than merging them into a universal score: task completion; unrecovered blockage; fallback availability under each tested condition; time and recovery time; errors/near-misses; resident-reported effort, autonomy, confidence and satisfaction; requested support; and conflicts between hard constraints/preferences. Include adverse results and uncertainty. No population-wide thresholds should be invented from synthetic fixtures.

### Longitudinal research
The accessibility systematic review included in WHO's 2018 *Housing and health guidelines* covered literature through April 2018. At that time, it described relatively few high-quality studies and called for stronger standardized and longitudinal evidence; it identified outcomes including injury rates, well-being, quality of life, mental health, dependency on external care services and social participation. Treat this as a research-gap signal from that review, not a statement that no newer evidence exists. Update the literature search before fieldwork. Future studies should measure participant-chosen outcomes over time where feasible and ethical, not only completion of a one-off route.

## Research context

- [ISO 21542:2021 — Accessibility and usability of the built environment](https://committee.iso.org/standard/71860.html). The official status page lists the edition as published and its review process as having reached close of review on 2026-09-03, with possible subsequent lifecycle stages shown; confirm the latest status and applicable local requirements before live use. Its scope includes access, circulation, egress and fire evacuation, plus recommendations for residential units.
- [WHO, Housing and health guidelines](https://www.who.int/publications/i/item/9789241550376), including accessibility for people with functional impairments and research priorities for standardized/longitudinal outcome measures.
- [South African National Strategic Framework on Universal Design and Access](https://www.gov.za/sites/default/files/gcis_document/202110/45328gen606.pdf), a local policy framework for universal design and access—not a project approval certificate.

## Integration and boundaries

This v0.3 contract extends the synthetic task network from PR #1705 and the semantic validator from PR #1711. It does not replace resident-led planning (#1687 / PR #1688) or the housing evidence/jurisdiction work (PR #1596). A schema-valid declaration does not prove that its statements are true.

Real deployment requires consented resident co-design, actual site measurements, current local regulations and qualified access/building/life-safety review. Do not collect unnecessary personal or diagnostic details to represent resident goals.
