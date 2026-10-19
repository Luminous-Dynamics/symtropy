# AGRI-ROBOT-SERVICE-009 — farmer-reported need episodes v1

**Status:** Contract and synthetic reference implementation draft. No farm-level data have been collected.
**Tracking:** [Issue #2125](https://github.com/Luminous-Dynamics/symtropy/issues/2125)
**Canonical service lifecycle:** [AGRI-ROBOT-SERVICE-002 / PR #1906](https://github.com/Luminous-Dynamics/symtropy/pull/1906)
**Field-report integrity:** [AGRI-ROBOT-SERVICE-007 / PR #2024](https://github.com/Luminous-Dynamics/symtropy/pull/2024)

## Paradigm shift

A request-led system cannot observe a service need that never turns into a request. A farmer may avoid enquiring because a service seems unaffordable, no provider is known, the response is likely too late, the technology does not fit the local task, the channel is inaccessible, or they prefer a trusted cooperative, broker or other arrangement.

The system must not invent needs from inactivity or turn a stated need into a request automatically. Instead it adds a separate, farmer-confirmed **need episode** record to measurement and leaves the canonical request/quote/booking/completion lifecycle in Symtropy's existing service-access module.

This is motivated by bounded research observations:
- A 2025 South African study of 242 farming households in four villages in Fetakgomo Tubatse Local Municipality found that extension access alone did not guarantee sustainable land-management adoption, discussing structural resource barriers and mismatch between offerings and farmer needs. It is a local cross-sectional study, not a national prevalence estimate: https://doi.org/10.3389/fsufs.2025.1618938
- A 2026 *Scientific Reports* study of 642 farmers across Kenya, Zambia and Zimbabwe found provider distance, labour demand, cultivated area and social capital associated with mechanization-hire adoption. This motivates recording farmer-confirmed barriers, not assuming effects transfer to South Africa: https://doi.org/10.1038/s41598-026-60618-3
- A 2026 Zimbabwe provider case reports negligible demand for equipment that did not fit the locally common basin-planting practice. It is a single-provider case, useful for generating a fit hypothesis, not for estimating demand elsewhere: https://doi.org/10.1080/23311932.2026.2715245

## Identity and ownership

- `need_ref` identifies a reported need episode.
- `request_ref` is an optional pointer to an existing canonical Symtropy request.
- Neither value is an event ID, evidence-authentication token, farmer identity or remote idempotency receipt.
- The episode contains no quote terms, booking state, payment state, route truth, personal/farm identifier or copied service lifecycle.
- A need episode has no dispatch, booking, price, eligibility, subsidy or service-side effect.
- A farmer must take a distinct action (or give a properly scoped, recorded mandate) to create or relay a canonical service request.

## Evidence basis

Eligible report bases are explicitly distinct:
- `farmer_confirmed_prospective`: farmer confirmed the current/anticipated need as it was observed;
- `farmer_confirmed_retrospective`: farmer confirmed a need in a defined recall period; report it separately from prospective observations;
- `authorized_relay_with_farmer_confirmation`: a relayer records a farmer-confirmed need within a referenced authority scope; this is not a direct digital signature;
- `unverified_intermediary_relay`: blocks the report;
- `inferred_from_non_use_telemetry`: blocks the report; an absent click/login/request never creates a need record.

For the authorized-relay basis, the fixture uses an opaque `auth-` reference. Its syntax is not proof that the mandate or consent is real; an actual integration needs its own authority verification.

## Request existence and outcomes are separate axes

Request presence can be:
- `linked_canonical_request` with an existing opaque `req-` reference;
- `farmer_confirms_no_request` (a farmer assertion, not a conclusion from missing platform logs);
- `no_canonical_request_located` (a bounded search found no matching canonical request; not proof that no request existed anywhere);
- `request_existence_unknown`;
- `not_assessed`.

Each eligible need episode gets exactly one declared outcome:
- `adequate_service_within_window`;
- `service_late_or_inadequate`;
- `need_still_open_no_adequate_service`;
- `farmer_deferred_need`;
- `resolved_without_service`;
- `outcome_unknown`;
- `follow_up_missing`.

An attainment claim also names its source: canonical service-lifecycle evidence, a farmer report of service obtained through another channel, an authorized relay of such a report, not applicable, or unknown. A farmer-reported off-platform service stays an off-platform report and is never promoted to a canonical platform booking or fulfillment receipt.

A voluntary no-request reason can be one of expected unaffordability, no provider known, timing mismatch, task mismatch, inaccessible channel, trust/privacy concern, required inputs unavailable, existing trusted channel chosen, chose not to proceed, other, unknown, unanswered, or not asked. Missing reason metadata is counted separately; it is not interpreted as affordability, provider scarcity, or lack of need.

## Denominators and outputs

Report these separately:
1. **Need denominator:** all eligible, explicitly reported/confirmed need episodes under the predeclared method, task, scope and time window.
2. **Service-request denominator:** the canonical service evaluator's eligible requests.
3. **Request-linkage count:** need episodes with an actual canonical `request_ref`; this is not a substitute for either denominator.
4. **Unrequested/reported need counts:** farmer-confirmed no-request, no canonical request located, unknown and not-assessed are separate.
5. **Outcome counts:** adequate within-window service, late/inadequate service, open need without adequate service, deferred, resolved without service, outcome unknown and follow-up missing.
6. **Attainment source:** canonical lifecycle versus farmer-reported/authorized-relayed other-channel attainment.

Do not calculate a need-to-request conversion rate from a retrospective convenience sample and call it prospective conversion. A transition metric requires a declared prospective cohort and fixed observation window. Do not conflate “no request found” with “farmer says no request was made.” Do not collapse “no service” into “no need.”

## Validation and claim ceiling

The Rust verifier checks declared scope, opaque refs, report-basis eligibility, optional request-link consistency, no-request reason placement, compatible outcome source, and screened/eligible/exclusion reconciliation. It rejects inference from non-use and unverified relays and emits no summary when blocked.

The synthetic oracle covers valid prospective/retrospective/authorized-relay rows; a canonical request; a farmer-confirmed no-request barrier; no canonical request located; alternative-channel service reported by the farmer; unresolved need; unknown/missing follow-up; and mutations that fabricate request IDs, remove authority, mix scopes, duplicate need refs, or silently lose the denominator.

A passing synthetic test proves only the model and committed fixtures. It does not estimate unmet-need prevalence, verify identity/consent, authenticate any provider or service, establish affordability, predict adoption or demonstrate agricultural benefit. A field study needs separate local review, informed consent, language review, data-minimization and retention rules, sampling/recall-window definitions, and a prospective protocol before claims are made.
