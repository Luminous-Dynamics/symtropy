# WaterLoop Storage Sizing and Pump Boundary v0.1

**Tracking:** [WATERLOOP-STORAGE-SIZING-001 #5467](https://github.com/Luminous-Dynamics/symtropy/issues/5467)  
**Implementation:** `waterloop-sizing-t0`  
**Evidence class:** synthetic-only  
**Qualification:** NotRun  
**Claim ceiling:** time-resolved storage and idealized pump power arithmetic only

## 1. Why storage must be sized from traces

A tank size is not established by a household population multiplied by a convenient number of outage days. The minimum usable reserve depends on when demand occurs, when a source is available, whether that source is admitted for the intended use, and how much inventory overflows because the tank is already full.

The calculator freezes these conventions:
1. initial usable storage is full at the start of the design horizon;
2. at each time step, admitted source inflow is added first;
3. storage is capped at usable capacity and excess is reported as overflow;
4. that time step's demand is served from the remaining usable inventory;
5. any shortfall is reported, never silently converted into supply;
6. raw inflow is credited only if source quality and metrology are both admitted for the same circuit/use profile and timebase.

The resulting minimum capacity is the smallest starting usable capacity that meets the complete frozen trace within the declared numerical tolerance. It is a trace-specific reserve calculation, not a forecast of actual source availability.

## 2. Usable and gross volume are different

`gross installed volume = minimum usable capacity + dead volume + freeboard + measurement uncertainty allowance + contingency allowance`

- **Usable capacity:** volume the design can safely draw while meeting the declared outlet/service conditions. The T0 evaluator assumes the full usable capacity is present at time zero.
- **Dead volume:** heel, pump submergence, sediment allowance or geometry that cannot be credited as useful reserve.
- **Freeboard:** physical headspace needed for the vessel/overflow design. It is not inventory.
- **Measurement uncertainty allowance:** explicit margin associated with the level-volume and flow measurements.
- **Contingency:** separately declared design allowance. It must not be hidden inside a larger unexplained “tank litres” number.

The calculator does not prescribe these allowances. T1 values need vessel geometry, calibrated level-volume data, operating limits, structural assessment and a declared design basis.

## 3. Time-resolved inventory calculation

For candidate usable capacity (C), storage (S_t), admitted inflow (I_t) and demand (D_t):

`A_t = S_t + I_t`

`O_t = max(0, A_t - C)`

`S'_t = min(C, A_t)`

`Y_t = min(D_t, S'_t)`

`U_t = D_t - Y_t`

`S_(t+1) = S'_t - Y_t`

where (O_t) is overflow, (Y_t) is delivered volume and (U_t) is unmet volume.

The T0 implementation uses a bounded binary search over (0) to total horizon demand. It returns the smallest tested upper capacity that meets all demand within the supplied tolerance, plus the remaining bracket width and iteration count. A declared maximum tank capacity is checked independently; if that limit is too small the evaluator reports the unmet volume at that limit.

### Important interpretation

The trace assumes **full initial storage**. It does not automatically imply that a rainwater tank will refill before an outage or that greywater is available during a mains interruption. Those assumptions must be represented as source inflow over the actual time steps, with seasonality, user activity, rainfall and quality gates admitted separately.

## 4. Cases 001–016

| Case | Purpose | Guardrail / result |
|---|---|---|
| 001 | Balanced demand/inflow | Derives 0.55 m³ usable and 0.77 m³ gross in the synthetic example |
| 002 | Time-shifted inflow | Later admitted source pulse replenishes the reserve; early capacity limits are visible as overflow |
| 003 | Unqualified source | Reports raw source volume but credits zero inflow |
| 004 | Potable/greywater circuit mismatch | Reject |
| 005 | Intended-use mismatch | Reject |
| 006 | Negative demand | Reject; never clip |
| 007 | Unequal demand/inflow trace lengths | Reject |
| 008 | Separate volume allowances | Same usable capacity, different gross installed volume |
| 009 | Declared max tank capacity too small | Reports theoretical minimum and unmet demand at the declared max |
| 010 | Zero demand / zero capacity | Zero usable capacity; no invented minimum |
| 011 | Timebase mismatch | Reject |
| 012 | Hydraulic power lower-bound arithmetic | 1.2 m³/h at 18 m TDH and 55% admitted efficiency |
| 013 | Efficiency greater than 1 | Reject |
| 014 | Demand measurement unadmitted | Indeterminate; no tank size claim |
| 015 | Negative source inflow | Reject; no clipping |
| 016 | Pump efficiency evidence unavailable | Hydraulic power may be shown; electrical lower bound remains null |

The independent input, manifest, oracle and schema are separate. The CLI receipt binds their SHA-256 values, Cargo.lock, exact implementation revision, case counts and mismatch records.

## 5. Pump boundary is not pump selection

For flow (Q) in m³/h and total dynamic head (H) in metres:

`P_hydraulic = ρ × g × (Q / 3600) × H`

`P_electrical_lower_bound = P_hydraulic / η`

The second result is emitted only when a pump-efficiency input is admitted and (0 < η ≤ 1). The formula supplies an idealized lower bound only. It does not select a pump or qualify a duty point. Pump selection still requires:
- pump and system curves;
- static lift and pipe/fitting losses;
- required fixture pressure and simultaneous demand;
- NPSH available/required and suction arrangement;
- transient/surge behaviour, start/stop cycling and dry-run conditions;
- motor/drive efficiency and measured whole-boundary electricity;
- maintenance, spare parts and power-interruption behaviour.

The reference architecture remains gravity/passive-first where the actual site can meet the service floors without a pump. Where pumping is necessary, compare complete pump+drive+pipe+storage systems rather than nameplate motor efficiencies.

## 6. Manufacturing and T1 sizing package

Before freezing any tank or pump:
1. Measure actual end-use demand by time step and identify essential/discretionary service classes.
2. Admit source inflow on the same timebase and exact source/use circuit; preserve dry periods and low-quality batches.
3. Declare the outage/design horizon and initial inventory condition.
4. Survey vessel geometry, dead volume, usable drawdown, freeboard, inspection access, cleanout, vent and overflow.
5. Calibrate level-volume relation; tank level alone is not volume.
6. Derive system curve, elevation, pipe losses, operating pressure/flow floor and pump duty where required.
7. Include material compatibility, structural loads, foundations, seismic/wind where applicable, local plumbing/electrical review and installation constraints.
8. Commission source isolation, backflow/cross-connection protection, reject route and fail-safe states.
9. Re-run service and cost benchmarks with the selected physical configuration and admitted measurements.

## 7. Proposed performance targets

These are engineering targets, not achieved results:
- zero quality-unadmitted inflow credited toward useful reserve;
- zero missing/negative inventory silently clipped into a favourable capacity estimate;
- exact timebase and use-profile match for every demand/inflow pair;
- all useful/gross/dead/freeboard/uncertainty/contingency volumes reported separately;
- a deterministic receipt for every frozen sizing configuration;
- size against matched service demand and outage traces, not nominal tank class;
- optimize gross installed volume and lifecycle cost only after maintaining the same essential service floor, quality gate and recovery time.

A future competitive advantage is an optimizer that jointly compares gravity-first storage, distributed storage, shared community storage and modular treatment using the same admitted service trace and costs. Do not collapse these alternatives into a single “best tank” score.

## 8. Nonclaims

All 16 cases are synthetic. This does not qualify a tank, vessel geometry, pump, pipe, foundation, installation, pressure system, water quality, water source, outage outcome or building service. It does not establish local plumbing/structural/electrical compliance, and authorizes no purchase or construction.
