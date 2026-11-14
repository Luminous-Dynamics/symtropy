# ACCESS-HOME-TASK-BENCH-001 — synthetic task-chain qualification

**Status:** draft benchmark protocol. No real home has been evaluated by this tool.

## Paradigm

Evaluate a home around resident-selected activities and complete task chains, not only around a room checklist or the presence of adaptation products. Entering a home, locating a bathroom at night, preparing food, receiving a visitor notification, or operating a control during network outage depends on a sequence of capabilities and environmental conditions.

The harness evaluates declared evidence for steps in a synthetic task. It never simulates a physical body or certifies a layout. It distinguishes three states:

- **Blocked by a declared barrier:** a capability marked required has an explicit known barrier.
- **Not evaluable:** no required barrier is known, but required evidence is missing, proposed, needs measurement or awaits professional review.
- **Evidence complete for fixture:** every required capability in the synthetic fixture has been explicitly labelled verified for its declared scope. This describes the fixture only, not a real building.

A known barrier takes precedence over another missing item so unresolved evidence does not hide an explicit obstacle. Optional requirements remain visible but do not gate the declared task; the resident and task designer must decide what is truly required.

## Scenarios

The v0.1 fixture includes nine single-task scenarios:
1. Resident-chosen low-light route to a bathroom, with unknown obstruction clearance and proposed lighting.
2. Vision/mobility interaction where a preferred tactile cue is declared to obstruct the route.
3. A Deaf resident's sleeping life-safety notification, with qualified review and backup evidence unresolved.
4. Mobility-device bathroom transfer, with site measurement and transfer needs unresolved.
5. Deafblind visitor notification, with resident-chosen tactile/communication strategy unresolved.
6. Essential control when network access is lost, with a declared lack of manual fallback.
7. Fatigue-aware meal preparation, a positive synthetic-only control.
8. Sensory/cognitive evening routine, with proposed or unknown controls.
9. Combined vision/hearing communication and alert path, with redundant alert review unresolved.

These names are task prompts, not claims about every person in a disability group. Real journeys must be co-designed with the people who use them.

## Evidence contract

Every capability records a stable identifier, whether it is required by the declared task, one explicit evidence state, an evidence reference (synthetic references for this fixture), a reviewer and a note explaining the assumption or gap.

Unknown states cannot be treated as zero-risk or verified. A schema-valid JSON file is not proof that its input claims are true. On a real project, each verified-for-scope input needs a separately qualified evidence process, reviewer authority, site scope, time and source receipt. That is outside this crate's scope.

## Benchmarks and acceptance targets

These are proposed targets, not measured results.

### Deterministic fixture suite
- 100% of expected scenario/task assessments match the authored fixture expectations.
- 100% of known required barriers yield blocked_by_declared_barrier.
- 100% of required unknown, proposed, site-measurement or professional-review states yield not_evaluable, unless a required known barrier already blocks the task.
- Zero malformed, duplicate or missing-provenance fixtures get a successful report.
- Same exact input bytes yield identical receipt JSON and SHA-256 on a pinned build.
- Every finding retains step/capability identifiers, evidence state, reference and reviewer role.

### Human validation later
- Recruit and compensate disabled co-design participants with overlapping access needs; do not treat one person as representative of a group.
- Agree the participant's task outcome and acceptable alternative before testing.
- Test real task chains in a mock-up or actual home using participant-selected assistive technology, equipment and support arrangements.
- Report task completion and severe blockers by relevant cohort/task. Do not let aggregate success conceal that one participant is blocked.
- Include degraded modes such as network/power failure, missing alert channels, route obstruction, low-light/high-glare, fatigue or unavailable assistive technology only when appropriate and safe.
- Record consent, change requests and adverse results; no surveillance or emotion inference.
- Define contextual thresholds with participants and local professionals before a study. Do not create a universal accessibility score.

## Standards and research context

- **ISO 21542:2021** addresses built-environment access, circulation, egress, evacuation during fire, common spaces and residential recommendations. ISO's status record notes the systematic-review stage; check current applicability and jurisdiction-specific rules for live design. https://committee.iso.org/standard/71860.html
- **RNIB home adaptation guidance** describes context-dependent lighting, contrast, tactile markers and controls. These are possible interventions, not universal rules for every blind or low-vision resident. https://www.rnib.org.uk/living-with-sight-loss/independent-living/practical-adaptations/
- **RNIB assistive technology guidance** describes home technology that can improve access for some people; the benchmark therefore evaluates failure modes and manual alternatives rather than prohibiting automation. https://www.rnib.org.uk/living-with-sight-loss/assistive-aids-and-technology/everyday-tech/using-technology-at-home/
- **South African National Strategic Framework on Universal Design and Access** provides local policy context, not project approval. https://www.gov.za/ss/node/809572
- **W3C WCAG overview** describes digital content accessibility as a distinct standards domain. Passing a planner UI audit does not establish that a real home is accessible. https://www.w3.org/WAI/standards-guidelines/wcag/

## Relationship to Symtropy

- ACCESS-HOME-001 planner issue #1687: resident-owned preferences and design brief.
- Leptos planner PR #1688: interactive requirement capture.
- Housing evidence PR #1596: layout/evidence/jurisdiction contracts.

The intended flow is resident priorities → task scenarios → evidence-triage receipt → site measurements, resident trials and professional review. The harness does not own canonical building geometry or duplicate statutory/engineering authority.
