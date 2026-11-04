# Grid firm-response reliability known-truth fixtures

**Claim class:** synthetic known truth only. No vector represents a real data-centre dispatch event, utility meter, balancing product or system resource-adequacy assessment.

All scenarios use one committed reduction of 100 kW in one fixed product/PCC/baseline/response-configuration/condition stratum. Confidence failure probability is delta = 0.05 (one-sided 95%). The exact expected event IDs are frozen before trial outcomes; unavailable opportunities remain explicit failed trials.

## Confidence examples

| Scenario | Successful | Failed/unavailable | Empirical pass rate | Hoeffding lower bound | Required | Result |
|---|---:|---:|---:|---:|---:|---|
| 20 perfect events | 20 | 0 | 100.00% | 72.63% | 70% | Qualified for tested 100 kW event stratum |
| 100 perfect events | 100 | 0 | 100.00% | 87.76% | 85% | Qualified for tested 100 kW event stratum |
| 101 events, one unavailable | 100 | 1 | 99.01% | 86.83% | 90% | Not qualified |

The result is a lower bound on mean event-success probability under independent bounded Bernoulli trials. The code cannot prove independence; opportunity-set digest and independence-basis ID are supplied by the caller. Do not interpret the examples as firm system capacity, ELCC, EFC, LOLE or EUE.

## Manifest and hostile controls

The sorted scheduled event IDs are part of the contract and report. The opportunity-set digest must be independently computed by the caller over its canonical manifest. Missing, extra, duplicate or cross-stratum opportunities reject before calculating a confidence bound. Product, PCC boundary, baseline method, response-configuration digest and committed kW must match each evaluated response report.

The fixture also includes controls for missing and unexpected event IDs, duplicate opportunity IDs, wrong stratum, wrong product/boundary/baseline/configuration and changed commitment level.

## Reproduction

1. Build the GridFirmCapacityIdentity and GridFirmCapacityContract from the fixture.
2. Supply exactly one GridFirmCapacityTrial for every scheduled event ID.
3. Use a validated GridResponseReport for an evaluated event, or GridFirmCapacityTrial::unavailable with a reason and evidence ID.
4. Compare empirical success, Hoeffding radius, lower confidence success and event-response reliability gate to the expected values.

## Scope

This is an event-response reliability screen within one frozen condition stratum. System resource adequacy requires chronological system simulation and probabilistic metrics such as ELCC/EFC, LOLE and EUE with correlated weather, workload, generation, storage, grid scarcity, outages, recovery and response degradation.
