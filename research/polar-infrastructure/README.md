# Polar Infrastructure Stressor Catalog v0

**Status: proposed stressor definitions only. No physical system has been tested.**

The goal is to improve survivability, graceful degradation, maintainability and safe recovery—not merely nominal design-day efficiency. Current cold-region guidance addresses utilities, foundations and buildings; emergency guidance emphasizes response difficulty in cold, dark and remote areas.

- [UFC 3-130-05 Arctic and Subarctic Utilities (2025)](https://www.wbdg.org/FFC/DOD/UFC/ufc_3_130_05_2025.pdf)
- [UFC Arctic/subarctic criteria index](https://legacy.wbdg.org/dod/ufc)
- [GNWT Good Building Practice for Northern Facilities, 4th edition](https://www.gov.nt.ca/en/newsroom/gnwt-releases-4th-edition-good-building-practice-northern-facilities-design-guide)
- [Arctic Council emergency preparedness](https://www.admin.arctic-council.org/about/working-groups/eppr/)

## Do not merge unlike geographies

The Arctic has inhabited land communities, local/Indigenous governance, variable permafrost and thaw/erosion risks. Antarctic stations have different ground, resupply and governance conditions. The geographic North Pole is over mobile sea ice; a platform there is not a land building. Every project needs a site-specific design basis and applicable engineering review.

## Included in v0

Fifteen definitions cover polar-night resource deficit, wind icing, battery derating, generator/inverter failures, delayed supplies, communications loss, sensor faults, cold restart, water/wastewater freeze, heat loss, permafrost settlement, snow/access blockage, correlated failure and gloved maintenance.

Each case records scope, required inputs, measures, hard gates and assumptions. All are marked `not_run`; none encodes universal temperature thresholds or asserts validated behavior.

## Benchmark protocol

Freeze site envelope, source/model revisions, scenario splits, hard constraints and resource budgets before evaluation. Compare incumbent, transparent rules, conventional optimization (LP/MILP/MPC where applicable), robust/stochastic methods when supported, candidate and ablations. Use held-out scenarios and deliberately defective input/controller cases.

Report critical-service continuity, time to professionally defined unsafe conditions, water/sanitation continuity, black-start/recovery, fuel and resupply, lifecycle cost/emissions, common-mode and tail failures, repair time, tools/spares, runtime/memory and uncertainty separately. Safety, water, environmental and rights constraints are hard gates: no aggregate-score gain compensates for their violation.

This catalog supports [Symtropy issue #2726](https://github.com/Luminous-Dynamics/symtropy/issues/2726) and is intended to integrate with the proposed [Mycelix Evidence Plane](https://github.com/Luminous-Dynamics/mycelix/pull/5452) after review. The shell checker is a structural smoke test, not a full JSON Schema validator.
