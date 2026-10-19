# AGRI-ROBOT-SERVICE-007 — field evaluation report template v1

**Template status:** Empty. This file contains no participant or service data.
**Protocol:** [AGRI_ROBOT_SERVICE_007_FIELD_EVALUATION_V1.md](AGRI_ROBOT_SERVICE_007_FIELD_EVALUATION_V1.md)
**Tracking:** [Issue #2023](https://github.com/Luminous-Dynamics/symtropy/issues/2023)

> Do not publish this report until recruitment, consent, local review, and the exact protocol version have been approved. Never include names, phone numbers, exact farm coordinates, raw contracts, or event payloads with personal/commercial content. Use aggregate counts and opaque synthetic or approved study references.

## 1. Preregistration identity

| Field | Value |
|---|---|
| Protocol version and digest | _Not set_ |
| Analysis plan version and digest | _Not set_ |
| Source repository commit | _Not set_ |
| Canonical service evaluator version | _Not set_ |
| Cross-channel schema/oracle version | _Not set_ |
| Study stage (0/1/2/3) | _Not set_ |
| Local review / ethics reference | _Not set; field work blocked until approved_ |
| Consent/purpose version | _Not set_ |
| Language and reviewed translation version(s) | _Not set_ |
| Orientation script/version | _Not set_ |
| Recruitment and observation dates | _Not set_ |
| Declared service/task and scope | _Not set_ |
| Region-level scope and crop/season window | _Not set_ |
| Farmer-declared useful-window rule | _Not set_ |
| Intended sample/precision justification | _Not set_ |
| Stopping rules and adverse-event contact/process | _Not set_ |

## 2. Eligibility and denominator reconciliation

| Count | Value | Definition/source |
|---|---:|---|
| All candidate requests screened | _Not set_ | |
| Eligible logical requests | _Not set_ | Fixed primary denominator before outcome inspection |
| Eligible requests with confirmed acceptance inside useful window | _Not set_ | Numerator for evidence-confirmed attainment |
| Eligible requests accepted late | _Not set_ | Outside useful window |
| Known not accepted: no provider/typed refusal | _Not set_ | Keep each reason separate in detailed results |
| Known not accepted: cancelled/no-show/abandoned | _Not set_ | Keep each reason separate |
| Provider reported complete; farmer acceptance absent | _Not set_ | Not an accepted service |
| Disputed/rework/quality rejection | _Not set_ | Keep the dispute visible |
| Remote outcome unknown after possible effect | _Not set_ | Not success/failure evidence |
| Follow-up missing/inaccessible | _Not set_ | Not success/failure evidence |
| Confirmed exclusions | _Not set_ | State preregistered rule and reason per category |
| Denominator reconciliation difference | _Not set_ | Must be zero or explicitly explained before report is final |
| Outcome-ascertainment coverage | _Not set_ | Give formula and numerator/denominator |
| Evidence-confirmed attainment rate | _Not set_ | Confirmed in-window accepts / all eligible requests |
| Missingness-based identification range | _Not set_ | Lower/upper bounds as described in protocol; not a confidence interval |

**Gate:** Counts must reconcile to the frozen eligible cohort, including unresolved/missing states and documented exclusions. No implicit deletion, no partial denominator, no missing-as-zero assumption.

## 3. Primary outcome and uncertainty

- Confirmed in-window acceptances: _Not set_
- All eligible requests: _Not set_
- Evidence-confirmed attainment rate: _Not set_
- Lower evidence bound: _Not set_
- Upper evidence bound: _Not set_
- Statistical uncertainty interval and method: _Not set_
- Number with outcome unknown/missing/inaccessible: _Not set_
- Reasons for missingness, permission limits or inaccessible follow-up: _Not set_
- Any sensitivity analysis preregistered in advance: _Not set_
- Deviations from the preregistered outcome rule: _Not set_

Keep the identification range caused by unknown/missing outcomes separate from a confidence interval. Known late, no-provider, cancelled, no-show, unaccepted-completion and documented-dispute states are not automatically added to the unknown category.

## 4. Lifecycle timings (report separately)

| Metric | Unit | N observed | Median | P50 method | P90 method | Missing/censored N | Source/authority |
|---|---|---:|---:|---|---|---:|---|
| Request → authoritative provider receipt | elapsed time | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ |
| Request → complete quote | elapsed time | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ |
| Quote → booking | elapsed time | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ |
| Request → provider-reported completion | elapsed time | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ |
| Completion → farmer decision | elapsed time | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ |
| Request → farmer acceptance | elapsed time; accepted cases only, labelled | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ |
| Offline capture → authoritative reconciliation | elapsed time | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ | _Not set_ |

State which clock/source defines each endpoint; do not use client/paper capture time as verified provider receipt. Do not relabel completion latency as accepted-service latency.

## 5. Channel, facilitation and context exposure

Only collect approved, consented, non-identifying fields necessary for interpretation.

| Field | Allowed values / unit | Result |
|---|---|---|
| Channel(s) used | app / SMS / voice / extension / cooperative-broker / paper-digitized / other approved | _Not set_ |
| Who initiated channel change | farmer / intermediary / provider / unclear | _Not set_ |
| Orientation version | opaque version ID | _Not set_ |
| Assistance code A0–A4 | self-directed / reread / navigation hint / rule explanation / rescue | _Not set_ |
| Prior tool familiarity | approved minimal categories / prefer not to answer | _Not set_ |
| Network/offline duration | elapsed time / unavailable / not collected | _Not set_ |
| Provider capacity/availability | source and scope noted | _Not set_ |
| Language review version | approved translation ID | _Not set_ |
| Accessibility or accommodation requested | minimal, consented, non-identifying summary | _Not set_ |

Report aided and unaided task outcomes separately. A critical error that occurred before assistance remains a critical error in the unaided result.

## 6. Safety, authority and comprehension gates

| Gate | Target | Observed count / denominator | Status and explanation |
|---|---|---|---|
| Relay misunderstood as provider receipt or booking | Zero critical cases; revise on any occurrence | _Not set_ | _Not evaluated_ |
| Provider completion misunderstood as farmer acceptance | Zero critical cases; revise on any occurrence | _Not set_ | _Not evaluated_ |
| Automatic resend after unknown remote outcome | Zero | _Not set_ | _Not evaluated_ |
| New disclosure after consent revoked | Zero | _Not set_ | _Not evaluated_ |
| Same-event-ID envelope conflict silently overwritten | Zero | _Not set_ | _Not evaluated_ |
| Distinct request refs silently auto-merged or dropped | Zero | _Not set_ | _Not evaluated_ |
| Eligible requests missing from denominator reconciliation | Zero unexplained | _Not set_ | _Not evaluated_ |
| Unauthorised access, privacy event, harm or safety concern | Zero tolerated; follow approved incident process | _Not set_ | _Not evaluated_ |

Any critical gate violation requires documented review and a stop/revise decision before expansion. Do not average a critical failure away in an overall satisfaction score.

## 7. Costs, service quality and provider viability

| Measure | Unit/basis | Farmer side | Provider/agent/cooperative side | Source/uncertainty |
|---|---|---|---|---|
| Service price and fee | currency per declared unit | _Not set_ | _Not set_ | _Not set_ |
| Travel charge and travel time | currency + elapsed time | _Not set_ | _Not set_ | _Not set_ |
| Waiting / time cost | elapsed time; valuation method if any | _Not set_ | _Not set_ | _Not set_ |
| Agent/cooperative labour | hours / declared cost basis | _Not set_ | _Not set_ | _Not set_ |
| Provider cost coverage | declared cost units / method | Not applicable or separately explained | _Not set_ | _Not set_ |
| Quality dispute/rework | count and typed reason | _Not set_ | _Not set_ | _Not set_ |
| Farmer-reported service quality | versioned scale and response rate | _Not set_ | Not applicable or separately explained | _Not set_ |

Do not combine farmer affordability and provider viability into a single score without showing both source values, units and denominator.

## 8. Missingness, deviations and adverse findings

- Missing/inaccessible evidence by type and reason: _Not set_
- Requests with uncertain/out-of-order/conflicting events: _Not set_
- Permission revocations and their effect on follow-up: _Not set_
- Protocol deviations and dates: _Not set_
- Adverse events and approved response/disposition: _Not set_
- Participant withdrawal counts and handling: _Not set_
- Unplanned analysis or post-hoc subgroup requests: _Not set_
- Any divergence between source ledger and report: _Not set_

## 9. Interpretation and claim ceiling

Choose the strongest supported level, not the desired level:

- [ ] Stage 0 — synthetic contract/model behavior only.
- [ ] Stage 1 — formative observations for the specific tested task, script, version, language and participants.
- [ ] Stage 2 — descriptive feasibility and scoped outcome estimates for the defined cohort.
- [ ] Stage 3 — comparative or causal inference, only if the preregistered design, compliance and assumptions support it.

Summary of supported findings: _Not set_

Alternative explanations (selection, season, task difficulty, provider capacity, distance, prior relationships, training/help): _Not set_

Claims explicitly not supported: _Not set_

Required follow-up, safety response or protocol revision: _Not set_

**No conclusion of South African population impact, causal yield/income/water benefit, universal channel superiority or agricultural safety follows from this template or synthetic data.**


## 10. Machine-checkable denominator receipt

The synthetic input contract is illustrated by `crates/domains/symtropy-garden-sim/fixtures/field_eval_report_example.v1.json`. Exact count values are decimal strings. Outcome classes are mutually exclusive and must total the eligible-request count: accepted in window, accepted late, known not accepted, disputed/rework, completed pending the farmer's decision, outcome unknown, and follow-up missing. Typed nonacceptance reasons must total the known-not-accepted class.

Run the verifier from the repository root after checking that the file contains only approved aggregate data and opaque references:

```sh
cargo run --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml --bin verify-field-eval-report -- \
  --input path/to/approved-field-report-input.v1.json \
  --report-json crates/domains/symtropy-garden-sim/target/field-report-receipt.v1.json
```

The receipt binds the assessment to the exact input bytes with SHA-256 and emits exact integer lower/upper identification bounds.

**Synthetic example interpretation (not field data):** the committed fixture has 12 screened requests, 10 eligible and 2 protocol exclusions. The eligible outcomes partition as 3 accepted within window, 1 accepted late, 2 known not accepted, 1 disputed/rework, 1 completed pending the farmer's decision, 1 outcome unknown and 1 follow-up missing. Thus the evidence-confirmed rate is bounded from below by `3/10` and from above by `6/10`; the three distinct pending/unknown/missing cases do not disappear, and the known-late/disputed cases do not inflate the upper bound. This is only a checker fixture, not a measured service-access result. `Blocked` means the counts or declared references do not satisfy the report contract; `NotAssessable` means no eligible request is present and no attainment rate is emitted. Neither state is evidence that a farmer did or did not receive a service. The verifier does not authenticate source records, estimate a confidence interval, or prove causality.

