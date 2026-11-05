# PRAS Interoperability Known-Truth Fixtures

**Schema:** `symtropy.pras-interop-fixtures.v1`  
**Classification:** synthetic known truth  
**External reference:** NLR PRAS 0.9.0  
**Parent:** Symtropy #3687

This fixture freezes deterministic arithmetic and mapping expectations before a Julia/PRAS runner is implemented.

## Cases

| Case | Main check | Expected result |
|---|---|---|
| Hourly single region | Period/day/hour/EUE/CVaR semantics | 3 periods, 1 event-day, 3 hours, 40 MWh |
| Quarter-hour | Resolution semantics | 2 periods, 1 event-day, 0.5 hours, 5 MWh |
| Midnight crossing | Calendar-day boundary | 2 periods, 2 event-days, 2 hours, 20 MWh |
| Paired signed workload | Complete trajectory and EUE-ELCC | 10 MWh baseline EUE, 0 flexible EUE, 5 MW EUE-ELCC |
| Rebound trap | Full-horizon recovery | 30 MWh baseline EUE, 50 MWh flexible EUE |
| Two-region location | Interregional deliverability | 5 MW relief at constrained load region improves EUE; relief in surplus region does not |

All cases use idealized deterministic availability. The eventual PRAS runner must first verify initial availability semantics against PRAS 0.9.0 and must fail the test if the emitted shortfall trace differs from the declared fixture. Do not quietly alter fixture values to match observed output.

## Important interpretation rules

- PRAS LOLE is event-periods, not a universal hours or contiguous event count.
- PRAS LOLD counts calendar days containing one or more shortfall periods.
- PRAS energy CVaR is tail severity of sample-level total EUE.
- NTCSA contiguous-event LOLE, LOLH and power CVaR require separate definitions and calculations.
- A workload trajectory is injected at the regional load profile exactly once. The same shift must not also appear as native PRAS demand response or storage.
- Multi-region PRAS regions are copper-sheet aggregates; no intra-region bus or distribution constraint is implied.

## Validation status

The JSON was authored as a declarative fixture specification. It has not been executed by PRAS, and no Julia or PRAS result is claimed. The next work package is a PRAS 0.9.0 pinned Julia harness that generates/loads native `.pras` models, evaluates these fixtures, saves PRAS result metadata and compares the sample-level results to this file.
