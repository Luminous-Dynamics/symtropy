# Farmer Quote and Budget Transparency Benchmark — V1

**Status:** draft implementation and synthetic benchmark. No real-farm affordability rate, willingness-to-pay estimate, or income inference is claimed.

## Paradigm shift

A provider quote being issued, a farmer accepting it, and a service being affordable are different events. Make all three observable without guessing farmer income or converting non-responses into acceptance.

This layer composes with the canonical service-access ledger; it does not reimplement the cost model, duplicate request lifecycle truth, or collapse access, affordability and service quality into one score.

## Contract

- Exactly one quote-response snapshot for each canonical quoted request. Missing, duplicate, unknown or unquoted response rows block the aggregate.
- Distinct states: accepted, declined, expired, pending. Decision timestamps must align with quote validity and bookings.
- Decline categories: price above budget, timing, task/scope mismatch, trust/safety concern, alternative provider, other or not stated. Farmers do not have to disclose a numeric budget just to state that a price is too high.
- Every quote explicitly declares whether the fee is for the total requested job or per service unit. Per-unit math and additional fees use checked integer arithmetic.
- Explicitly include separately quoted travel fees and all additional mandatory fees in the all-in total. The current canonical request schema supports one separately quoted travel fee; unmodeled mandatory charges must not be omitted from a real quote receipt.
- A farmer's total budget ceiling is optional and retained only with explicit disclosure consent. When not provided, affordability is unknown, not “affordable” and not “unaffordable”.
- Only aggregate counts are emitted. Individual budget ceilings are not copied into the assessment output. Production collection still requires purpose limitation, access control, retention limits and consent-withdrawal handling.
- Source references and 64-hex digest shape do not authenticate artifacts.

## Frozen synthetic oracle

The TSV manifest contains 22 named cases and is checked by a standalone structural verifier. The manifest verifier does not import the evaluator or assert that test cases passed. Run the integration tests and warning-free Clippy independently.

Hand-calculated example: 100 units × 10 minor units = 1,000; a separate travel fee of 250 and additional mandatory fee of 50 yield 1,300 total minor units. A disclosed budget of 1,300 is within the ceiling; 1,200 is below the quote. This is a test fixture, not a market price.

## Benchmarks

| Gate | Target |
|---|---|
| Request-to-response coverage | 100% of canonical quoted requests represented exactly once |
| Invalid quote decision/lifecycle cases | 100% of negative controls block |
| Quote total arithmetic | 100% exact integer oracle match; zero overflow or implicit rounding |
| Fee transparency | 100% of counted quotes declare total-job vs per-unit basis and include each known mandatory charge |
| Decline evidence | 100% of declined quotes carry a reason category; NotStated is allowed |
| Budget handling | Zero budgets retained without consent; zero invented income/budget values |
| Affordability classification | Compare only the explicit quote total with a voluntarily disclosed total budget; otherwise report unknown |
| Quote conversion | Report accepted/declined/expired/pending separately; do not claim causal reasons from a descriptive log |
| Equity and privacy | Disaggregate only under consent and privacy review; expose no individual budget in aggregate output |

For field qualification, collect a local baseline across several providers and tasks before setting targets for quote response time, acceptance, fee burden or price-related declines. Pre-register the denominator and retain all requests, including no quote, decline, expiry and pending states. Numeric ceiling comparisons must not become a universal affordability score.

## Research rationale

- A 2026 study of 642 farmers across Kenya, Zambia and Zimbabwe found that provider distance, labour demand, cultivated area and social capital were associated with mechanization-hire adoption; estimated impacts were heterogeneous. These observations justify measuring access and opt-in affordability, not transferring effect sizes to South Africa. https://www.nature.com/articles/s41598-026-60618-3
- A 2026 Zimbabwe service-provider case reported geographic spread, local-practice mismatch, repair/parts barriers and different service-line economics over 42 months, but studied one selected provider. This motivates complete quote terms and locally validated service data; it does not set a price target for another country. https://www.tandfonline.com/doi/10.1080/23311932.2026.2715245
- A willingness-to-pay study in Malawi found substantial variation by service type and farmer characteristics. Such estimates are context-dependent, not substitutes for voluntarily disclosed budgets in individual service transactions. https://onlinelibrary.wiley.com/doi/10.1002/agr.21841

The next separate workstream covers repair/parts recovery in AGRI-ROBOT-SERVICE-003 (#1907). Affordability, provider cost coverage, geography, service quality and machine reliability should remain distinct diagnostic dimensions.
