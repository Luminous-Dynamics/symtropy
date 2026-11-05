# PRAS Interoperability Known-Truth Fixtures

**Schema:** `symtropy.pras-interop-fixtures.v1`  
**Classification:** synthetic known truth  
**External reference:** NLR PRAS 0.9.0  
**Parent:** Symtropy #3687

The fixture contains six deterministic known-truth cases plus a stochastic sample-level energy-CVAR convention case.

| Case | Main check | Expected result |
|---|---|---|
| Hourly single region | Period/day/hour/EUE semantics | 3 periods, 1 event-day, 3 hours, 40 MWh |
| Quarter-hour | Resolution semantics | 2 periods, 1 event-day, 0.5 hours, 5 MWh |
| Midnight crossing | Calendar-day boundary | 2 periods, 2 event-days, 2 hours, 20 MWh |
| Paired signed workload | Complete trajectory and EUE load restoration | 10 MWh baseline EUE, 0 flexible EUE, 5 MW deterministic EUE restoration |
| Rebound trap | Full-horizon recovery | 30 MWh baseline EUE, 50 MWh flexible EUE |
| Two-region location | Interregional deliverability | 5 MW relief at constrained load region improves EUE; relief in surplus region does not |
| Non-degenerate energy-CVAR | Sample-level tail semantics | CVAR reproduced from the exact PRAS 0.9.0 strict-above-quantile convention |

## Important metric nuance: PRAS 0.9.0 energy CVAR

The upstream 0.9.0 implementation computes the sample EUE quantile and averages only samples **strictly greater than** that quantile. If no samples exceed it, the implementation returns zero. Therefore, an all-identical deterministic sample vector with 40 MWh EUE in every sample returns energy-CVAR 0 MWh.

This is surprising relative to common empirical expected-shortfall conventions that include mass at the VaR threshold. The harness asserts the version-specific PRAS implementation and records a non-degenerate stochastic sample vector. Cross-model comparisons must pin the empirical tail-mass convention rather than equating metrics based only on the name "CVaR".

## Execution harness

The optional Julia environment and runner live in `benchmarks/ai-data-centre-energy/pras/`:
- `Project.toml` pins PRAS exactly to 0.9.0 and Julia compatibility to 1.10.
- `run_pras_interop.jl` runs the tests.
- `test/runtests.jl` constructs native PRAS models, evaluates metrics, saves/reloads a native HDF5 `.pras` model, and emits a machine-readable execution receipt.
- `.github/workflows/pras-interop.yml` runs the harness and uploads that receipt.

The time-region test uses non-symmetric 2-region × 3-interval load data so accidental matrix transposition is observable.

All fixture values are synthetic. PRAS/Julia execution results must come from the version-pinned runner; arithmetic checks alone do not establish an executed pass. No official NTCSA model reproduction, substation deliverability, market accreditation or field result is claimed.
