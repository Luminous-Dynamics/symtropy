# AGRI-ROBOT-SERVICE-007 — channel and human-assistance field evaluation protocol v1

**Status:** Protocol draft. No participants recruited, no field data collected, no service-impact results claimed.
**Tracking:** [Issue #2023](https://github.com/Luminous-Dynamics/symtropy/issues/2023)
**Canonical service measurement:** [AGRI-ROBOT-SERVICE-002 / PR #1906](https://github.com/Luminous-Dynamics/symtropy/pull/1906)
**Offline/institutional access:** [AGRI-ROBOT-SERVICE-006 / issue #1936](https://github.com/Luminous-Dynamics/symtropy/issues/1936)
**Cross-channel provenance:** [Mycelix FARMER-UX-006 / PR #5210](https://github.com/Luminous-Dynamics/mycelix/pull/5210) and its draft oracle [PR #5212](https://github.com/Luminous-Dynamics/mycelix/pull/5212)
**Optional advice linkage:** [FARMER-LINK-001 / issue #1982](https://github.com/Luminous-Dynamics/symtropy/issues/1982)

## 1. Research question and paradigm

Does making the same service request operable through digital and institution-mediated channels improve dependable, affordable, farmer-accepted service—or does it merely increase registrations, messages, provider bookings or apparent completion?

**Primary outcome:** the proportion of all eligible service requests resulting in explicit farmer acceptance within the farmer-declared useful window.

The protocol treats app, SMS/voice, cooperative or extension assistance, broker-mediated and paper-digitized paths as alternative/combined ways to participate. No channel is presumed superior, and a farmer does not have to abandon an existing trusted channel. This protocol measures service access; it does not introduce a new service lifecycle.

### Research motivation

- Abdelaziz & Abay, *Food Policy* (2026), studied variants of digital-literacy training through an RCT involving two Egyptian agricultural apps. The authors report significant changes in uptake/use and improvements in trust, including heterogeneous effects. The context and intervention differ from South Africa; this is a reason to measure training/facilitation separately, not to import effect sizes. https://doi.org/10.1016/j.foodpol.2026.103108
- Gatti et al., *Journal of Rural Studies* (June 2026), report multi-year qualitative fieldwork in Ghana and Kenya. The article highlights the gap between subscription and sustained use, socio-technical misfit, and roles for extension, peer and low-tech pathways. It does not establish a universal failure of digital services or local South African effects. https://doi.org/10.1016/j.jrurstud.2026.104210
- FAO's Digital Services Portfolio describes local/country adaptation of service themes for smallholders and family farmers. Local language, season and institutional fit must be tested rather than assumed. https://www.fao.org/digital-services/about/

### South African evidence — local warnings, not national estimates

- A 2024 Eastern Cape study of 250 purposively selected smallholder farmers reported perceived cost, skill and usability barriers; 78.8% described digital technologies as expensive and 54.4% did not find them user-friendly. These values describe that study sample, not all South African farmers. DOI: https://doi.org/10.17159/2413-3221/2024/v52n5a16200
- A 2025 survey of 104 smallholder farmers in Ingquza Hill Local Municipality, Eastern Cape, found mobile phones and social messaging were important extension-information paths while respondents reported connectivity and data-cost barriers. Its specific municipality and survey design limit generalization. https://www.frontiersin.org/journals/sustainable-food-systems/articles/10.3389/fsufs.2025.1704550/full
- A 2025 City of Tshwane study found extension access was associated with adoption of some online information sources, but reported no statistically significant income difference attributable to digital-information adoption in that sample. This is a useful reminder to measure service access and income separately rather than assume one causes the other. DOI: https://doi.org/10.17159/2413-3221/2025/v53n3a19230
- A 2024 study of South African agricultural advisers reports readiness to use digital platforms alongside affordability, network-coverage and digital-literacy barriers; the reported national mobile/internet figures should not be treated as proof that every farmer can reliably complete a service transaction. DOI: https://doi.org/10.17159/2413-3221/2024/v52n4a16851

**Protocol consequence:** before any South African pilot, recruit through local farmer/cooperative/extension relationships, let participants select their language and existing channel, and document provider availability, data/network costs and assistance exposure. Do not recruit only smartphone-app users, equate possession of a phone with reliable transaction access, or promise an income/productivity impact from a higher adoption rate.

## 2. Staged study design

### Stage 0 — engineering and synthetic controls

Before involving participants, verify the event/provenance contract, report denominators, adversarial schedules, data minimization and no-auto-resend behavior using synthetic records. Label all output synthetic. No field-ready claim follows from passing this stage.

### Stage 1 — formative comprehension/usability

Recruit approximately 5–8 consenting participants for failure discovery, not population estimates. Where feasible include different roles: farmers, extension/cooperative staff, and service-side participants. Choose language from participant preference after human review/translation. Do not require a smartphone, personal device or mobile-data plan.

Use role-play and synthetic request IDs. Do not collect real farm names, exact coordinates, phone numbers, member keys, financial history, or real provider/customer contracts. Recording is optional; anonymous notes are preferred.

Run seven critical tasks: (A) continue one request across app-to-cooperative channel change; (B) distinguish asserting actor from relayer; (C) distinguish relay from authoritative provider receipt; (D) preserve suspected duplicates with different refs; (E) distinguish paper/client capture time from receipt/verification time; (F) block forwarding after consent revocation; and (G) keep a lost remote response unknown until authoritative reconciliation, then append correction rather than rewriting history.

**Stop/revise:** any participant believes real data was sent when it was not, a relay is a provider receipt or booking, unknown outcome authorizes auto-resend, consent revocation still permits forwarding, or a suspected duplicate was silently deleted/merged. One critical misconception blocks progression until reviewed and corrected; do not average it away.

### Stage 2 — prospective feasibility pilot

Only after Stage 1 gates, local review, consented recruitment, authorized access to service records and safety procedures are approved, prospectively observe one well-defined task/service in a declared region and crop/season window. Preserve existing channels and record which channel(s) were actually used. No causal impact claim: user-selected channel, provider availability, distance, timing and prior relationships confound outcomes.

Freeze the protocol, definitions, study window, eligible-request rule, data retention/access, language materials and analysis plan before field outcomes are inspected. Report whether the intended logs can be collected completely and safely; publish no personal or commercially sensitive records.

### Stage 3 — comparative study (separate approval)

If the feasibility pilot supports a fair comparison, preregister the estimand, comparator, assignment/analysis method, sample-size or precision justification, stopping criteria and harms analysis before starting. Do not randomly deny an existing channel or necessary human assistance. Training/onboarding comparisons are a separate intervention with its own protocol; this small formative round cannot estimate training effects. If channels are chosen by participants rather than assigned, report associations and measured confounding rather than calling differences causal.

## 3. Unit, cohort and denominator contract

**Unit:** one eligible logical farmer service request, keyed by the canonical opaque request reference. A channel handoff does not create a new request; a separate reference is not auto-merged based on similarity. Corrections are appended and linked.

Before recruitment, declare:
- service and task definition, region/scope, crop/season and the useful-window rule;
- eligible start/end dates and how duplicate/suspected-duplicate, withdrawn, out-of-scope and test requests are handled;
- whether the source is prospective event capture, authorized historical records or both;
- what makes an observation authoritative and who is allowed to record/relay each event;
- how missing, stale, conflicting, inaccessible or revoked evidence is represented;
- outcome follow-up deadline, exclusions, sampling/recruitment and stopping rules.

Keep all eligible requests accounted for. No-provider, refusal, quote expiry, abandoned, cancellation, no-show, late, provider-reported-but-not-accepted, disputed/rework and unresolved outcomes remain distinct. If the preregistered cohort excludes a case, record a reason and counts; never quietly discard it or treat missing follow-up as success, failure or zero.

### Outcome ascertainment and missingness

The primary numerator is **confirmed** farmer acceptance inside the useful window, supported by the required decision evidence and timing. The fixed denominator includes all preregistered eligible requests. This produces an *evidence-confirmed attainment rate*, not an assertion that every unconfirmed request failed.

At study close, classify every request as one of: accepted within window; accepted late; known not accepted (typed reason); disputed/rework; outcome unknown after possible remote effect; follow-up missing/inaccessible; or protocol-defined exclusion with reason. Do not collapse the last two into a known failure. Report the number in each category and outcome-ascertainment coverage.

Alongside the fixed-denominator evidence-confirmed rate, report a transparent uncertainty range when unresolved/missing cases could plausibly contain an unobserved in-window acceptance:
- lower evidence bound = confirmed in-window acceptances / all eligible requests;
- upper evidence bound = (confirmed in-window acceptances + cases whose in-window outcome is genuinely unascertainable) / all eligible requests.

Only include cases in the upper-bound addition when acceptance-within-window cannot be ruled in or out from the authorized evidence. Known late, no-provider, cancellation, no-show, unaccepted completion and documented disputes are not silently reclassified as unknown. Explain any contradiction between a reported “no provider”/cancellation event and later acceptance as a reconciliation case. A classified-case rate may be shown as secondary descriptive analysis, never as a replacement for the fixed denominator. These bounds are identification limits, not confidence intervals; show statistical uncertainty separately.

## 4. Endpoints

### Primary

`accepted_within_useful_window / all_eligible_requests`

Numerator requires a distinct farmer acceptance/decision record (or explicitly declared authorized recorded mode with its assurance class) and verified event time within the predeclared useful window. A provider-reported completion alone is not acceptance. Keep the denominator frozen independently of whether a request was completed or accepted.

### Secondary — report lifecycle stages separately

- request to authoritative provider receipt; request to quote; quote to booking; request to completion; and completion to farmer decision;
- accepted within window versus accepted late; no-provider/typed-refusal, abandonment, quote expiry, cancellation, no-show, rework, quality rejection and dispute;
- farmer cash expenditure, fees, travel, waiting and time-cost (each declared by source and unit); provider/agent/cooperative operating cost and provider cost coverage reported separately;
- network outages and delay to sync/authoritative reconciliation; count of unknown remote outcomes and time to resolution;
- human-agent time and number/type of help contacts; correction rate and disagreement about source/authority;
- language/accessibility barriers and whether the needed channel was available, only where necessary, consented and safe to measure.

Report medians and nearest-rank P50/P90 of each explicitly named lifecycle wait, with cohort size and missingness. Do not label request-to-completion wait as request-to-accepted-service wait. Do not use registrations, downloads, page views, messages, training attendance or booked jobs as the primary outcome.

## 5. Training, orientation and assistance controls

Version one neutral orientation script for all Stage 1 sessions; it states that data are synthetic, no provider is contacted, the participant may stop at any time, and help is available. Do not teach the answer before baseline tasks.

For each task record outcome before rescue and one assistance code:
- `A0` self-directed;
- `A1` prompt re-read verbatim;
- `A2` technical/navigation hint without explaining the authority decision;
- `A3` explains domain/authority/consent rule;
- `A4` moderator rescue to finish.

Keep unaided and assisted results separate. A critical error before help remains a critical error; do not convert assisted completion into unaided success. Record the orientation version, participant-selected language, test channel/mode, prior tool familiarity if consented, and whether the task was self-directed or mediated. Collect no extra demographic attributes unless justified by an approved plan; never use demographics as proxies for competence.

## 6. Outcome/evidence separation invariants

- Relay is not provider receipt, quote, booking or farmer acceptance.
- Provider-reported completion is not farmer acceptance; a dispute remains visible.
- No unknown outcome is automatically resent; it remains unknown without authoritative reconciliation.
- No client/paper timestamp alone rewrites received/verified time or establishes authoritative order.
- Consent revoked before forwarding blocks new disclosure. Past receipts do not imply permission for new sharing.
- A local event ID is not a remote service idempotency guarantee; a digest is not actor identity or truth.
- An optional advice-to-service link is not evidence that advice caused the request or outcome. Linkage does not create treatment assignment or a causal impact label.
- Report request counts, no-provider, cancellations/no-shows, late/unfinished, unaccepted, disputes and missing follow-up distinctly.

## 7. Analysis and uncertainty

Before Stage 2, define the primary estimand and compute a sample size or desired confidence-interval width from declared local baseline assumptions and expected eligible-request volume. If that input is unavailable, report feasibility and uncertainty without an arbitrary success threshold. Do not retrofit the sample size after inspecting outcomes.

For every rate publish numerator, denominator, missing/unknown count, protocol exclusions, observation window and uncertainty interval. Use exact counts for small formative rounds; do not present percentages as population estimates. For time outcomes, show distribution and censoring/missingness rather than reporting only the mean or only completed jobs.

If comparing non-random channel use, state that self-selection, farmer need, task difficulty, season, distance, provider capacity, prior relationships and assistance may explain differences. A causal claim requires a defensible assignment/comparison design and independently sourced outcomes.

## 8. Privacy, governance and reporting

- Store only opaque request/event IDs in public artifacts; keep sensitive payloads in an authorized domain store with explicit access, retention, correction and deletion rules.
- Never publish raw phone numbers, farm names, precise coordinates, identity/member keys, private quotes or supplier/customer contracts.
- Require consent and purpose limitation for research use and for any channel-specific aggregation. A minimum cohort size alone is not a proof of anonymity.
- Separate participant safety/accessibility observations from judgments about competence. Participants may withdraw from research; withdrawal must not affect service eligibility.
- Report adverse events and protocol deviations alongside outcomes. Version source commit, schema, fixtures, orientation script, language review, analysis plan and report.

## 9. Targets (proposed gates, not empirical claims)

### Engineering/conformance gates
- 100% of valid channel handoffs retain the same logical request reference.
- Zero duplicate side effects on exact replay; zero silent overwrite of same-event-ID envelope conflicts.
- Zero automatic merges of distinct request references without authorized human resolution.
- Zero unauthorized forwarding after consent revocation.
- Zero promotion of relay/provider completion/unknown into a higher-authority claim without required evidence.
- 100% of eligible requests accounted for and all declared exclusion reasons visible.

### Formative comprehension gates
- At least 90% comprehension of who asserted a fact, who relayed it, what is confirmed and what remains unknown; report the raw count and exact task-level denominator.
- Zero critical false belief that relay equals provider acknowledgement/booking. Any occurrence blocks expansion until the issue is fixed and retested.
- Zero unauthorized disclosure in the consent-revocation task; zero silent merge/drop in duplicate task.

### Field-effect gates
Do not set an arbitrary percentage improvement, adoption/retention target or affordability cutoff before local baseline, season, task, provider availability, sampling and cost basis are declared. Stage 2 should first establish denominator completeness, follow-up feasibility, outcome provenance, workload and harm guardrails. Only then preregister a comparative target that is justified by the local baseline and uncertainty/precision needs.

## 10. Machine-checkable report preflight

The Rust report preflight in `src/field_eval_report.rs`, runnable as `verify-field-eval-report`, validates the screened cohort, fixed eligible denominator, preregistered exclusions and one outcome row per eligible request before outputting identification bounds. It is deliberately downstream of the canonical service evaluator: it does not generate, repair or infer service lifecycle events.

Run from the repository root:

```sh
cargo run --locked --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml --bin verify-field-eval-report -- crates/domains/symtropy-garden-sim/fixtures/field_eval_report.v1.json
cargo test --locked --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml --all-targets
```

The frozen example is synthetic. Its six eligible requests include one confirmed in-window acceptance, one late acceptance, a typed no-provider outcome, a provider-completion record awaiting a farmer decision while the useful window is open, unresolved dispute/rework, and an unknown outcome. Two screened candidates are explicitly excluded using preregistered rule references. The expected evidence-confirmed lower bound is (1/6), the missingness upper identification bound is (4/6), and ascertainment coverage is (3/6). These are worked fixture values, not service-performance measurements.

The preflight blocks all computed rates if any screened candidate is unaccounted for, an eligible request is missing/duplicated in the outcome records, an outcome is assigned to an ineligible reference, an exclusion is duplicated/undocumented, or the useful-window/outcome classification is inconsistent. Every outcome must cite a source-event reference for the window deadline and its classified result; every exclusion must cite its supporting event and a rule from the declared rule-ID inventory. Accepted outcomes must also cite a distinct acceptance event, a decision mode (farmer-direct or authorized intermediary-recorded), and an acceptance time. The preflight compares that time with the declared deadline; intermediary-recorded acceptance requires a mandate reference. It blocks an in-window label whose time is after the deadline, and a late label whose time is at/before it. These references are cross-checked for shape/consistency only—the tool does not authenticate the cited events, verify real consent, or prove the rule inventory was frozen before outcome inspection. The study operator must archive and version the actual analysis plan before outcome review, and cite its immutable commit/digest in the report. The CLI receipt emits `receipt_version` plus `source_manifest_sha256` over the exact manifest bytes; the workflow independently recomputes that digest with `sha256sum` and rejects a mismatch. The digest binds a report to a specific byte sequence but does not authenticate source or truth. A nested `Cargo.lock` plus `--locked` commands freezes dependency resolution so builds cannot silently re-resolve dependencies. Rates use exact integer numerator/denominator pairs rather than rounded floating-point percentages. Known late, typed non-acceptance, and resolved no-acceptance states do not inflate the upper bound; only explicitly unascertainable outcomes do. A rework acceptance inside the useful window contributes to the primary numerator while remaining in the dispute/rework breakdown.

## 11. Claim ceiling

Stage 0 supports only claims about synthetic model and fixture behavior. Stage 1 supports only formative comprehension/usability observations under the exact declared script, language, prototype and participants. Stage 2 supports descriptive feasibility and scoped service-outcome estimates for the declared cohort. Stage 3 supports causal claims only if its predeclared design and assumptions justify them. None alone proves broad South African adoption, causal yield/income/water benefits, universal channel superiority or agricultural safety.
