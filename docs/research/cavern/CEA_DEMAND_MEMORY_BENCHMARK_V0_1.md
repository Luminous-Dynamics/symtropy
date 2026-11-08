# CEA demand-memory and rolling-ratchet control benchmark v0.1

**Parent:** [CEA-DEMAND-MEMORY-001 #4194](https://github.com/Luminous-Dynamics/symtropy/issues/4194)  
**Composes:** #4074 harvest-constrained control, #4081 tariff source, #4120 bill-component evaluator, #1916 storage / thermal dispatch.

**Status:** source-pinned tariff rule, synthetic state-transition fixture and benchmark protocol. It does not implement a battery model, optimizer, crop model, tariff parser or physical controller.

## 1. The field problem

The City Power 2026/27 Industrial TOU tariff makes monthly billed demand persistent. The current month is charged against the maximum of:
- current measured monthly peak;
- 70 kVA floor;
- 80% of the mean of the three largest monthly demands in the preceding twelve months.

Official source: City of Johannesburg, *Medium-Term Budget 2026/27–2028/29*, Industrial TOU tariff section 5, printed pp. 95–97:
https://joburg.org.za/documents_/Documents/2026-27-Final-Medium-Budget-Reports/2026-27-FINAL-BUDGET-BOOK.pdf

The budget's clause 5.4.3 states a new Industrial TOU eligibility gate of supply capacity strictly greater than 100 kVA, subject to application, qualification and approval. This is not the same as the 70 kVA demand floor. Actual account approval and the current signed tariff statement govern eligibility.

The tariff's measurement interval for determining apparent-power demand must be supplied from the account's qualified meter / tariff documentation. Focal Energy's 25 September 2026 engineering note describes City Power demand as fixed half-hour slots and confirms the rolling top-three/12-month rule; treat the 30-minute slot as a secondary comparator until the actual account confirms it:
https://www.focalenergy.co.za/knowledge/kva-maximum-demand-charges-gauteng

## 2. Stateful billing mathematics

Let:
- (D_m) = current measured monthly peak in kVA;
- (H_m = [d_{m-12}, ldots, d_{m-1}]) = twelve previous qualified monthly peaks;
- (Top3(H_m)) = the three largest values in the history;
- (F = 70\text{ kVA});
- (ho = 0.8).

The ratchet floor and billed demand are:

`R_m = rho × mean(Top3(H_m))`

`B_m = max(D_m, F, R_m)`

At the next billing transition, the current month's measured peak enters the history and the oldest month's observation expires:

`H_{m+1} = tail_11(H_m) ++ [D_m]`

This means the control decision has a multi-month economic externality. A peak avoided now can reduce future bills; a peak caused by grid charging, simultaneous HVAC/LED starts or a battery recharge can remain financially relevant long after the current billing period.

Do not count this as energy saved. It is a billed-demand cost trajectory. kWh, kW, kVA, energy charge, demand charge, fixed charges and the per-kWh network surcharge remain separate.

## 3. Why existing 2026 controllers do not fully close the gap

**Stateful stochastic MPC comparator.** Bernardino, dos Santos & Marafão, *Stochastic model predictive control for PV–BESS microgrid energy management under demand charge*, Energy (2026), DOI 10.1016/j.energy.2026.142809. The publisher abstract describes a 15-minute SMPC/MILP with an explicit historical peak state and reports 12.57% cost reduction against an uncontrolled/no-storage baseline on a medium-voltage university campus. It includes BESS degradation and forecast uncertainty; this is a model / site-specific result, not a CEA or City Power outcome.
https://doi.org/10.1016/j.energy.2026.142809

**Prescient-oracle MPC comparator.** Perez-Pineiro, Skogestad & Boyd, *Home Battery Dispatch under a Tiered Peak Power Tariff*, Optimization and Engineering (online Aug 2026). The work supplies a perfect-foresight MILP lower bound and reports its MPC within 1.7% of that bound on one year of household data. Its tariff uses the average of the N largest daily peaks per month, not the City Power prior-12-month top-three rule.
https://web.stanford.edu/~boyd/papers/hbd.html

**Tariff-aware imitation comparator.** Katholnigg et al., *Tariff-Aware Imitation Learning for Transferable Battery Storage Control*, ACM E-Energy '26, DOI 10.1145/3744255.3811715. The study evaluates 25 commercial-building profiles across three synthetic tariff families; training on two schedules per family transfers to held-out schedules and reports 12–15% bill reductions. The authors explicitly list demand ratchets and multi-month coupling as not covered by their tariff families. This is the direct transfer-generalization gap we target.
https://doi.org/10.1145/3744255.3811715

## 4. Benchmark baseline ladder

Compare on identical trace, tariff, billing history, battery, forecast and crop constraints:
1. No-storage/static operating baseline.
2. Simple peak-threshold heuristic.
3. Tariff-aware heuristic using current peak but no rolling history (deliberately stateless negative comparator).
4. Day-ahead MPC with current billing-cycle peak only.
5. Rolling-ratchet-aware MPC with prior 12-month top-three state.
6. Stochastic MPC with forecast scenarios and calibrated uncertainty.
7. Tariff-aware imitation policy distilled from the ratchet-aware MPC.
8. Perfect-foresight MILP / exhaustive oracle for bounded small cases.

For the CEA campaign, hard crop/harvest, climate, condensation, water, equipment and outage reserve constraints must remain hard constraints. A cheaper bill cannot compensate for a missed harvest or unsafe climate excursion.

## 5. Frozen case families

Use at least 40 deterministic cases:
- top-three average, ratchet floor and 70 kVA floor arithmetic;
- current peak dominates prior ratchet;
- prior ratchet dominates current peak;
- new high peak enters next month and changes future state;
- oldest high peak expires after rolling twelve-month update;
- 11-month / 13-month / duplicate / skipped / stale / unqualified history rejection;
- a peak set by HVAC, LED, pump, CO2 or synchronized restart;
- off-peak battery recharge creates a new maximum in an all-slot demand tariff;
- peak-shaving battery is empty before an unforecast morning event;
- insufficient battery kW despite sufficient kWh;
- sufficient battery kW but insufficient energy duration;
- inverter apparent-power / reactive-power competition;
- poor power factor changes kVA without equivalent kW/kWh change;
- solar uncertainty at the ratchet-setting interval;
- tariff measurement interval differs from controller interval;
- wrong 15-minute/30-minute aggregation;
- month-boundary / clock / missing-data / stale-meter evidence errors;
- tariff transfer across changed TOU hours with the same ratchet rule;
- changed ratchet rule / tariff version rejection;
- demand history missing -> NEEDS_EVIDENCE, never a zero-history default;
- crop harvest constraint prevents load shedding;
- BESS reserve and tariff savings compete for SOC;
- degradation / cycle-throughput cost;
- forecast bias, calibration drift and conformal interval expansion;
- outage recovery and staggered motor restarts;
- one-month savings that increase future twelve-month demand charges;
- full invoice reconciliation against #4120;
- exact small-case oracle and deterministic randomized seed corpus.

## 6. Metrics

Report separately:
- energy charge, demand charge, fixed/service/network charges and surcharge/VAT state;
- current measured peak, billed demand, ratchet floor and top-three membership;
- monthly and rolling twelve-month bill trajectory;
- new peaks set, avoided peaks and the months over which each peak remains influential;
- whole-facility kWh and peak kW/kVA;
- BESS SOC, throughput, losses, degradation proxy, inverter kW/kVA, reactive support and reserve;
- crop accepted mass, harvest deadline and climate service;
- forecast calibration, oracle gap, fallback events, infeasible solve frequency and compute latency.

Do not merge cost, physical energy reduction and emissions reduction into one number.

## 7. Advancement goals

- Exact agreement with independent arithmetic and exhaustive oracle on all bounded cases.
- Zero resetting of the historical peak state at a billing-month boundary.
- Zero use of missing / unqualified history as zero.
- Zero false bill reduction when demand-charge memory, tariff interval, account surcharge or crop constraint is omitted.
- Target under 3% optimality gap to the prescient MILP on bounded synthetic cases and under 5% on unseen synthetic trace families; these are research targets, not demonstrated performance.
- Beat stateless MPC and threshold heuristics on complete bill outcomes under the same constraints.
- Transfer to held-out TOU schedules using explicit tariff features and historical-peak state; no absolute-time schedule memorization.
- Any physical savings claim requires matched meter traces, actual account tariff, complete 12-month history and independently verified crop / climate service.

## 8. Claim ceiling

This benchmark qualifies policy calculations on source-pinned tariff rules and synthetic / admitted meter histories. It does not establish any customer's tariff eligibility, battery sizing, real facility savings, crop performance or permission for autonomous physical dispatch.