# LBE-BENCH-004: Longitudinal Disagreement Debt and Institutional Memory v0.1

**Status:** research and evaluation protocol  
**Tracking:** [issue #3123](https://github.com/Luminous-Dynamics/symtropy/issues/3123)  
**Related:** [LBE-BENCH-003 issue #3110](https://github.com/Luminous-Dynamics/symtropy/issues/3110), [event-sourced kernel PR #3091](https://github.com/Luminous-Dynamics/symtropy/pull/3091)  
**Principle:** unresolved is not the same as neglected; closure is not the same as learning; institutional memory must survive a change of leadership.

## 1. Research purpose

LBE-BENCH-003 tests whether a process can preserve distinct positions, distinguish evidence from values, respond to hazards and make reviewable decisions. BENCH-004 asks what happens after the meeting: do positions remain accessible, are commitments revisited, who carries the cost of delay, and can a decision be reopened when the world changes?

The research object is the *trajectory of an unresolved or provisionally handled issue*, not the personality or psychological state of any participant.

## 2. Working construct: disagreement debt

Disagreement debt is a portfolio of unresolved or deferred issues that continues to impose material or procedural costs.

It is **not automatically negative**. A principled hold-open decision can be safer and more legitimate than premature closure. Deferral becomes debt when it causes accumulated harm, missed review, hidden ownership, cost shifting, lost voice, repeated re-litigation without learning, or no funded/accessible path to resolution.

Represent debt as a vector, not a scalar:

| Dimension | What to record |
|---|---|
| Age | Creation time, last substantive review, time overdue |
| Voice | Affected groups, access to contribute, withdrawals and reasons |
| Material incidence | Service reliability, scarcity, finances, ecological load, physical capacity |
| Care/facilitation burden | Hours, payment, who performs work, concentration across groups |
| Accountability | Owner, decision authority, next review, appeal route, expiry |
| Evidence | New evidence, quality, scope, update and rebuttal history |
| Reversibility | Stop trigger, rollback route, recovery time, remedy access |
| Recurrence | Reopened issues, repeated cycles and changed context |
| Distribution | Who benefits from deferral, who pays, which future groups inherit costs |

Never add these dimensions into a universal "wisdom", "love", "quality" or "conflict" number. Report each dimension and uncertainty separately.

## 3. Multi-horizon protocol

A single workshop or vote cannot reveal all institutional dynamics. Compare at least three horizons:

- **Micro:** moments to weeks. Speaking access, agenda control, immediate task-to-status escalation, retaliation, and response to new evidence.
- **Meso:** months to several years. Repeated review cycles, policy implementation, safeguard regression, staff turnover, cost transfer and delayed service outcomes.
- **Macro:** institutional generations and longer trajectories. Path dependence, institutional memory, accumulated unpaid care, ecological consequences and reactivation of old tensions.

The 2026 process-temporal review of organizational paradox research groups processes along short (0–4 years), medium (5–24 years) and long (25+ years) horizons, and sequential, cyclical and entangled styles. Those categories are useful analytical lenses, not mandatory durations for every pilot. Synthetic benchmarks should simulate all scales; field studies must choose follow-up periods appropriate to observable outcomes and funding.

## 4. Issue lifecycle and event lineage

An issue may move through:

1. submitted / position registered;
2. inquiry and evidence gathering;
3. held open, provisionally prioritized, integrated, sequenced, contextualized, experimentally tested or empirically resolved;
4. implementation and monitoring;
5. review due, renewed, amended, reversed, closed or reopened.

A reopened issue never erases its earlier closure or finding. It adds a new event referencing the earlier record and explains what changed: new evidence, changed context, affected group, or previously inaccessible voice.

Minimum event envelope:
- stable sequence number;
- host-authenticated actor identifier;
- timestamp and declared time basis;
- event type and canonical payload reference;
- predecessor receipt and current content-addressed/signed receipt;
- reason, scope and evidence reference where applicable.

The kernel's sequence/time/link/replay checks are not cryptographic verification. The host must authenticate actors, verify signatures and canonical digests, bind the genesis position set, enforce authorization, protect privacy and store events in an independently protected append-only system.

## 5. Benchmark arms and matched design

Compare:
1. majority vote;
2. facilitated consensus;
3. Delphi-like anonymous iterative elicitation;
4. adversarial collaboration with jointly agreed fair tests;
5. participatory platform baseline with proposal, amendment, voting and accountability functions;
6. LBE plurality-preserving, event-sourced procedure.

Match scenario, population, information, resources, external shocks, facilitator hours, time budget and decision authority where possible. Record every mismatch. Do not compare the most resourced LBE arm to an under-resourced baseline and claim an architecture win.

At least two independent evaluators should code process outcomes. Use participant surveys, interviews, observation, policy/document review and quantitative outcome traces. Preserve disagreements between evaluators and pre-register adjudication.

## 6. Scenario suite

| ID | Shock or failure | Required observation |
|---|---|---|
| DD4-01 | Leadership turnover attempts to delete dissent | Minority positions, corrections and source lineage survive |
| DD4-02 | New leadership changes agenda, resource and information control | Power map changes; previous commitments remain attributable |
| DD4-03 | Legitimate hold-open decision becomes overdue | Review is triggered without forcing an unjustified closure |
| DD4-04 | Refuted hazard receives genuinely new evidence | New finding is appended; earlier refutation remains visible |
| DD4-05 | Unpaid mediation/care work concentrates on a subgroup | Burden is measured, named, compensated or remedied |
| DD4-06 | Short-term rationing damages long-term service reliability | Future costs and affected groups are visible |
| DD4-07 | External shock reactivates a previously settled paradox | Issue reopens with new context and preserved past rationale |
| DD4-08 | Connectivity loss or exit makes appeal inaccessible | Offline/accessibility route is tested; safeguard regression blocks action |
| DD4-09 | Emergency exception remains after expiry | It ceases or receives independently justified renewal |
| DD4-10 | Formerly correct policy becomes wrong after changed conditions | Evidence update, review and repair occur without rewriting history |

## 7. Required metrics

### Process durability
- percentage of positions surviving leadership turnover;
- event completeness and deterministic replay parity;
- missing, duplicate, reordered, backdated or broken-receipt event rejection;
- overdue review frequency and age distribution;
- safeguard regression detection and time-to-restoration;
- hazard report-to-review, review-to-remedy and reopening latency.

### Epistemic performance
- update after decisive evidence;
- calibrated probability quality for claims with resolvable ground truth;
- false closure and false reopening rates;
- distinction between empirical, normative and structural issue types;
- decision reversals that were warranted versus arbitrary.

### Material and distributional outcomes
- service continuity, quality and capacity;
- allocation and scarcity incidence by affected group;
- care, facilitation and participation hours, paid/unpaid status and concentration;
- environmental and long-horizon resource costs;
- time to remedy, remedy adequacy and repeat harm;
- decision regret after subsequent evidence or shocks.

### Participation and rights
- position fidelity confirmed by its source;
- accessible participation and independent appeal;
- retaliation, coercion and essential-access conditioning;
- exit cost and whether access to essentials is independent of agreement or participation;
- who controls agenda, data, funds, implementation and review.

Report denominators, uncertainty, missingness and subgroup variation. A high number of recorded concerns is not itself failure; a low number is not proof of safety.

## 8. Exact conformance gates

- Every safeguard, hazard and disposition state change is represented by exactly one event.
- Replaying a valid event stream from the same genesis state yields identical state and history.
- Invalid receipt links, duplicate receipts, reordered events and backdated timestamps fail closed.
- A refuted hazard remains in history; reopening requires a new evidence-linked event.
- A deferral has a named accountable owner, review deadline and accessible affected-party route.
- An expired emergency exception cannot silently retain authority.
- The system does not convert normative choices into factual resolutions.
- Essential entitlements are not conditional on agreement, contribution, reputation, attendance or a waiver of dissent.

These gates test the software and declared process contract. They do not prove the truth of attestations or the absence of coercion in the real world.

## 9. Empirical hypotheses (pre-register before pilot)

- H1: receipt-linked append-only history reduces silent loss of minority positions after leadership turnover versus baselines without a durable lineage mechanism.
- H2: explicit review ownership and deadlines reduce stale deferrals without materially increasing premature closure.
- H3: cost and care-burden telemetry reveals transfers that are invisible in vote or consensus outputs.
- H4: the ability to reopen with new evidence reduces the duration of harmful outdated policies.
- H5: independent appeal and accessible offline voice reduce the gap between formal rights and effective access.

These hypotheses may fail. Pre-register primary and secondary outcomes, exclusions, subgroup analyses, missing-data handling, stopping rules and a policy for reporting null or harmful findings.

## 10. Comparative position

- **OECD deliberative evaluation** provides process-design, deliberative-experience and impact-pathway evaluation guidance. This benchmark should implement and extend those evaluation layers with issue-level event lineage and material incidence.
- **Decidim** provides participatory spaces, proposals, amendments, voting and accountability functions. LBE should interoperate with such infrastructure rather than recreate a civic portal.
- **Delphi** provides independent/anonymous rounds, iteration, controlled feedback and statistical summaries. LBE should preserve non-consensus and normative dissent where agreement is inappropriate.
- **Adversarial collaboration** asks opposing researchers to characterize each other fairly, agree on a fair test in advance, and publish regardless of outcome. LBE should make this type of test traceable and reusable.
- **Team-conflict and paradox research** show why conflict dimensions, power and temporality matter; do not assume conflict improves performance.

The sought contribution is an executable, longitudinal benchmark connecting durable dissent, review/appeal, event provenance and cost-shifting to real service, care and ecological outcomes. No superiority claim is warranted before matched evaluation.

## 11. Research references

- Johansen & Nielsen (2026), [A process-temporal perspective on organizational paradoxes](https://doi.org/10.1111/ijmr.70032), *International Journal of Management Reviews*.
- Yuan, Yin & Sun (2026), [The paradox of team conflict revisited: An updated meta-analysis](https://doi.org/10.1037/apl0001315), *Journal of Applied Psychology*.
- [Whether and when team task conflict and relationship conflict transform over time](https://doi.org/10.1108/IJCMA-09-2024-0238), meta-analysis of 32 longitudinal studies.
- OECD, [Evaluation Guidelines for Representative Deliberative Processes](https://doi.org/10.1787/10ccbfcb-en) and [Guidelines for Citizen Participation Processes](https://www.oecd.org/en/publications/oecd-guidelines-for-citizen-participation-processes_f765caf6-en.html).
- Decidim, [general platform description](https://docs.decidim.org/en/develop/features/general-description.html).
- BMJ (2025), [Proposal for a critical appraisal tool for studies using the Delphi method](https://www.bmj.com/content/391/bmj-2025-084509.long).
- University of Pennsylvania, [Adversarial Collaboration Project](https://web.sas.upenn.edu/adcollabproject/).

These works motivate the protocol; they do not prove that LBE will outperform existing methods.
