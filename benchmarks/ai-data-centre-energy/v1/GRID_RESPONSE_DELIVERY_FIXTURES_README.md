# Grid-response deliverability known-truth fixtures

**Claim class:** synthetic known truth only. These vectors are not measured grid, utility, aggregator or data-centre events.

The fixtures pin a 100 kW reduction request over four 1-second intervals. Baseline PCC import is 1,000 kW. The response threshold is 90 kW, maximum response latency is 2 seconds, minimum compliant interval fraction is 0.75, maximum underdelivery energy fraction is 0.10, maximum overshoot fraction is 0.05, and maximum ramp is 150 kW/s.

Samples are interval-end readings at 1, 2, 3 and 4 seconds after instruction issuance. The first ramp is measured from a pre-event delivered reduction of zero. Ramp results therefore apply only at 1-second telemetry resolution; sub-second transients remain unverified.

## Known-truth vectors

| Case | Delivered reduction (kW) by interval | Main expected result |
|---|---|---|
| Fast sustained | 100, 100, 100, 100 | Pass; response latency 1 s; delivered energy fraction 1.0 |
| Late initial | 0, 100, 100, 100 | Fail underdelivery despite eventual response; response latency 2 s |
| Sustained under-delivery | 100, 40, 40, 100 | Fail sustained compliance and underdelivery gates |
| Excessive overshoot | 120, 120, 120, 120 | Fail overshoot gate; delivered fraction 1.2 |
| Import increased | -20, -20, -20, -20 | Negative delivered energy retained; no response threshold crossing |
| Excessive ramp | 100, 100, 100, 100 with max ramp 50 kW/s | Fail ramp gate; measured first-interval ramp 100 kW/s |

The hostile contract also requires rejection when sample interval is 2 seconds but the product response deadline is 1 second. A verifier cannot prove a response requirement at finer resolution than its supplied telemetry.

Every case includes separate synthetic baseline and response evidence identifiers. The Rust API computes under-delivery and overshoot from the signed baseline-minus-response difference; it must never clamp negative delivery to zero.

## Reproduction and limitations

Construct the GridResponseIdentity and GridResponseContract from the pinned fixture. Convert each paired baseline/response value into a GridResponseSample at the specified interval-end timestamp, using the paired evidence IDs. The report must match the exact latency, energy, compliance, overshoot and ramp arithmetic.

The module qualifies time-resolved response tracking only. Whole-horizon workload loss, facility energy, temperature, and rebound remain owned by the existing flexibility-event evaluator. Baseline quality/counterfactual validity remains upstream M&V responsibility. No fixture establishes a real market-product qualification.
