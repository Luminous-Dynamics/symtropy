# Case9 published aggregate comparison (not a local reproduction)

Source: Wang, Yin, and Khan, “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding,” Table 6, https://doi.org/10.1049/cim2.70066. The values are pinned in tools/shipyard-benchmark-contract/fixtures/published-case9-table6-summary.json. This is analysis of published summary values, not a local optimizer result.

## Case9_50_4 published values

| Method | Yard utilization (Aur) | Algorithm time (s) | Vehicle time (min) |
|---|---:|---:|---:|
| PA (published proposed method) | 0.775 | 308.0 | 2,339 |
| IA (traditional ABC) | 0.765 | 398.7 | 2,339 |
| GA | 0.760 | 335.6 | 2,386 |
| DE | 0.768 | 352.1 | 2,360 |

The table records two workstation disruptions and two vehicle disruptions for this case. The paper says each case is repeated 15 times, but the public article page provides aggregate table entries rather than the underlying per-run samples or random seeds.

## Arithmetic contrasts from Table 6

These are direct arithmetic differences between published table entries; they are not independent measurements and do not establish statistical significance.

- **PA vs IA:** +0.010 utilization (1.0 percentage point on the printed fractional scale); algorithm time is 90.7 s lower (22.75% relative to IA); vehicle time is tied at 2,339 min.
- **PA vs GA:** +0.015 utilization (1.5 points); algorithm time is 27.6 s lower (8.22% relative to GA); vehicle time is 47 min lower (1.97% relative to GA).
- **PA vs DE:** +0.007 utilization (0.7 points); algorithm time is 44.1 s lower (12.52% relative to DE); vehicle time is 21 min lower (0.89% relative to DE).

This is a multiobjective trade-off, not a universal ranking. PA's published headline on this row improves utilization and runtime relative to all three baselines; travel time only improves over GA and DE and ties IA.

## Reproduction protocol implications

The paper reports a Python implementation on Windows 11 with an Intel Core i7 at 2.40 GHz and 32 GB RAM. Its sensitivity tables select SN = 30 nectar sources and MaxIter = 200 iterations; its weighted-goal criterion assigns weights of 0.5 to space and 0.5 to time. These parameters and hardware are part of the comparison contract, to the extent the full implementation confirms them.

Before comparing our code to these values:
1. Recover the full input and exact case-level scoring definitions, including yard aisle treatment and shape representation.
2. Match the same objective, disruption event set, decision timing and feasible-operation semantics for the headline reproduction.
3. Run 15 repetitions with declared deterministic seed handling (for our implementation) and save every per-run artifact; do not compare best-of-15 to reported aggregate as though equivalent.
4. Compare the published method and our methods on the same frozen case and same compute budget, and report distributions, feasibility, utilization, vehicle time, recovery horizon, plan churn and re-handles separately.
5. Run stronger operational variants—delayed clearance, finite qualified fleet, load/unload durations, travel-time dependence and resource disruptions—as separately named ablations.

A credible “better-than-paper” claim requires the same source instance and objectives, comparable budget/hardware, a feasible output for every claimed result, per-run uncertainty, and public evidence of the exact tested revisions. If input data are unavailable, the comparison remains non-comparable regardless of how closely a synthetic run matches the printed row.

## Higher-bar follow-on

The paper's stated simplifying assumptions leave room for a more operational digital twin: outgoing blocks treated as relocated before confirmed clearance, concurrent arrivals not ordered within a short-term plan, loading/unloading time neglected, workstation travel time treated as constant, and sufficient vehicle variety assumed. The Symtropy recovery stack already models delayed clearance and unary resources in its own declared scope. The next controlled ablations should isolate the value of each added constraint rather than bundling all changes into one result.
