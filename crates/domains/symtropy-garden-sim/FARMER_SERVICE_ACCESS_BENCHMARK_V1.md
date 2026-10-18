# Farmer-First Service Access Benchmark — V1

**Status:** draft implementation/benchmark contract. All included cases are synthetic. No local farmer demand, access rate, affordability, service quality, or field outcome is claimed.

## Paradigm shift

Do not rank farm technologies by machine ownership, nameplate capacity, theoretical hectares per day, or robot task completion alone. Rank the full service pathway:

**request → complete quote → accepted booking → completed job → farmer-accepted delivery within the farmer-declared useful window.**

Keep requests with no provider, quotes declined, cancellations, no-shows, unfinished bookings, rework, and quality rejection in the denominator. A machine finishing work is not proof that the farmer received an acceptable outcome. A booking or forecast is never silently promoted to fulfilment.

This contract complements the field-robot cost kernel and provider-wide revenue coverage. It does not merge them into one composite score and does not infer farmer affordability from provider cost coverage.

## Hard data gates

- Every request has a stable request ID, scenario, scope, period, task, explicit service unit, positive requested quantity and a farmer-declared useful-window deadline.
- Request, quote, booking, completion and delivery-decision times use one declared minute origin and must be monotone.
- A quote requires fee, currency, fee basis, validity and explicit travel-charge treatment. Separate travel charges are shown as distinct fees.
- Booking requires a complete quote accepted before quote expiry. Completion requires booking; delivered units must be present and positive.
- Farmer acceptance/rework/rejection has a decision timestamp after completion. Completed-but-unreviewed work is not reported as accepted.
- Missing, stale, conflicting, malformed, duplicate-ID, scope-mismatched or period-mismatched evidence blocks the whole aggregate; no partial denominator is emitted.
- Source references and digest shape are validated structurally only. They are not authenticated.
- Optional cohort aggregates require explicit consent and at least 10 records per displayed cohort. Smaller cohort keys and values are omitted. This threshold is not differential privacy and does not guarantee against all re-identification; collect no names/direct identifiers, minimise fields, and seek privacy review before deployment.

## Frozen synthetic oracle

The fixture at crates/domains/symtropy-garden-sim/fixtures/service_access_cases.v1.tsv contains 19 hand-authored case IDs matched by name to integration tests. The verify-service-access-manifest binary only checks manifest/test-name structure and prints STRUCTURE_VALID_ONLY; the Rust tests must run separately. Synthetic pass results would prove only the evaluator against these fixtures, not farm outcomes.

The evaluator reports request, quote, booking, completion, accepted delivery, completed-but-pending farmer decision, rework, quality rejection, no-outcome and unresolved counts. It reports accepted deliveries inside/outside the farmer's useful window and nearest-rank P50/P90 request-to-completion wait in integer minutes. The percentiles use completed requests, including completed-but-not-yet-accepted work; they must not be read as accepted-service response-time statistics.

## Benchmarks and gates

| Area | Minimum gate / target |
|---|---|
| Frozen hand-calculated oracle | 100% exact match |
| Invalid lifecycle, evidence, units, price terms and duplicate IDs | 100% of negative controls block |
| Partial or silently dropped requests | Zero |
| Request → quote → booking → completion → farmer decision chain | 100% valid event ordering |
| Quote unit, validity and travel treatment | 100% complete for every counted quote |
| P50/P90 timing | Exact nearest-rank calculation; clearly label the denominator |
| Cohort reporting | No displayed cohort below k=10; no label without consent |
| Field response-time objective | Use farmer-declared useful windows; do not prescribe one universal number before local baseline |
| Field service outcome | Pre-register task-specific accepted-delivery and rework criteria before pilot |
| Transferability | Validate over multiple providers, communities, task types and seasons; hold out at least one provider/site for external validation |
| Inclusion | Report service request outcomes by consented cohorts only where sample sizes allow; investigate access gaps rather than hiding them in a global average |
| Service resilience | Record cancellation/no-show, downtime, parts-wait and recovery time so outages are counted as reduced access |
| Affordability | Record fee, currency, unit, validity and travel terms; pair with farmer-declared affordability or willingness-to-pay evidence. Do not infer a universal income threshold |

A provisional operational hypothesis for a future pilot is to improve the proportion of farmer-accepted deliveries completed within the farmer-declared useful window and reduce P90 wait against a matched baseline, without worsening rework, fee transparency, or access gaps. Freeze target percentages only after local baseline, task type, season and sampling plan are declared.

## Field evidence plan

1. Shadow-log every request (including no-service outcomes) for at least one full relevant service season; record task, requested units, local conditions, service deadline, quote, travel charges, booking, completion, farmer decision and any repair/parts delay.
2. Keep provider accounts, farmer outcomes and task quality as separate linked records. Do not equate provider revenue coverage with affordability, yield gain or equitable service.
3. Compare manual/conventional service, hired equipment and robot-assisted service on the same task and scope where feasible; record labour hours, travel, weather/soil constraints, rework, machine downtime and completed/accepted units.
4. Use matched comparisons or pre-registered causal designs for claims about yield, income, labour reduction or welfare. A service register alone is descriptive, not causal.
5. Report site/provider variation and confidence intervals; preserve missing-data and non-service counts. Do not generalise from a single best-case provider.

## Research motivation

- A 2026 survey study of 642 farmers across Kenya, Zambia and Zimbabwe found that distance to providers, labour demand, cultivated area and social capital were associated with mechanization-hire adoption; reported impacts were heterogeneous. This motivates measuring geographic access and subgroup outcomes, not assuming universal benefit. https://www.nature.com/articles/s41598-026-60618-3
- A 2026 Southern African Zimbabwe service-provider case followed one selected high-performing provider for 42 months. It highlights repair/parts availability, mismatch between machinery and local practice, and diversification across service lines, while explicitly limiting generalisation from a single provider. https://www.tandfonline.com/doi/10.1080/23311932.2026.2715245
- FAO's 2026 discussion argues against one-off equipment delivery disconnected from skills, finance, local fit and after-sales service. https://www.fao.org/africa/news-stories/news-detail/mechanizing-africa-s-farms-won-t-work-unless-we-do-it-differently/

The new contract deliberately measures access and farmer-accepted outcomes before trying to optimise fleet placement, dynamic scheduling, maintenance inventory or provider matching. Those optimisations should be downstream of trustworthy request, task and failure data—not a substitute for it.
