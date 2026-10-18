# Repair and Parts Recovery Benchmark — V1

**Status:** draft chronology contract with synthetic fixtures. It does not claim a real fleet's failure rate, repair duration, parts lead time, or reliability.

## Paradigm shift

Treat repairability as service capacity. A robot that is technically repairable but waits weeks for a needed part is not available capacity during the waiting period. Also distinguish machine breakdowns from scheduled maintenance, weather/site-access interruptions, and operator unavailability so that the wrong cause is not attributed to the robot.

The ledger links outage episodes to canonical service-access request IDs and separates failure-to-verified-restoration time from parts lead time and external interruption duration.

## Event contract

A versioned repair episode declares asset ID/configuration, scope, period, currency, outage cause, event times, part requirement state, affected request IDs and evidence state. Events distinguish outage start, failure detection, asset removal, diagnosis, part required/ordered/received, repair start/completion, service restored, and asset verified available.

- A breakdown requires failure detection and removal from service.
- Scheduled maintenance is separate from breakdown and does not claim a failure-detection event.
- Weather/site-access and operator unavailability cannot contain machine failure, parts or repair events.
- Part requirement has three states: not assessed, not required, required. Unknown is not treated as not required.
- A repair cannot start without completed diagnosis, or before a declared required part is received.
- A restored machine is not considered available until the asset availability verification event is recorded.
- Every affected request must join to a canonical request ID. Duplicate, unknown or cross-scope IDs block the aggregate.
- Open episodes remain open. Incomplete history is never assigned zero duration.
- All event timestamps must be monotone and at or before the assessment's as-of minute.
- Source references and digest shape do not authenticate event evidence.

## Benchmark gates

| Dimension | Target |
|---|---|
| Event identity and join integrity | 100% unique episode IDs and known affected request IDs |
| Lifecycle negatives | 100% impossible event sequences block |
| Downtime accounting | Exact integer elapsed times; zero imputed-zero durations for open incidents |
| Repair restoration | Report P50/P90 from failure detection to verified availability for closed breakdowns only |
| Parts delay | Report P50/P90 for completed order-to-receipt intervals and count open parts waits separately |
| Cause attribution | Zero weather/access/operator interruptions included in breakdown restoration percentiles |
| Scheduled maintenance | Separate count from breakdowns |
| Request impacts | Preserve affected-request links and separately report unique affected requests |
| Evidence completeness | Explicitly retain whether log-window completeness is declared; a boolean declaration is not authenticated evidence |
| Transferability | Set field targets only after observing a full local service season across multiple assets and providers |

A zero-event dataset with an incomplete log is not evidence of zero incidents. Percentiles use only episodes with verified closure and must state that denominator.

## Field qualification

Record outage and repair events over a locally relevant service season. Report machine-specific breakdown frequency only against a declared exposure denominator; this contract currently measures event chronology and restoration, not operating-hour failure rates. Keep scheduled maintenance, weather/site-access and operator downtime separate. Preserve censored/open episodes and data missingness.

Compare locally repairable designs, supplier-supported repair pathways and parts stocking only under matched task, asset configuration and operating exposure. Inventory or predictive-maintenance optimization should beat a simple held-out baseline and must not increase unserved requests or total lifecycle cost.

## Research rationale

- A 2026 Zimbabwe service-provider case followed one selected provider for 42 months and highlighted spare-parts availability, repair support, locally appropriate machinery and dispersed service demand. Its single-provider design limits generalisation. https://www.tandfonline.com/doi/10.1080/23311932.2026.2715245
- A 2026 study of 642 farmers across Kenya, Zambia and Zimbabwe reported that distance to service providers and other contextual factors influenced hired mechanization adoption, with heterogeneous estimated effects. The results motivate measuring interruptions and access but do not set local reliability targets. https://www.nature.com/articles/s41598-026-60618-3

The ledger does not duplicate the annual maintenance-cost equation, provider revenue coverage, or the quote-affordability model. It supplies explicit repair/outage event evidence for later cost and service-access analysis.
