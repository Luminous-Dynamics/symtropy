# CO2 warm-plasma quenching benchmark — Symtropy process-twin contract v0.1

**Status:** Source-pinned operating-point registry and replay contract. No Luminous plasma measurements, CFD result, chemistry result or device qualification is claimed.

**Symtropy issue:** [PLASMA-QUENCH-TWIN-001 #5391](https://github.com/Luminous-Dynamics/symtropy/issues/5391)

**Scientific owner:** [Symthaea PLASMA-QUENCH-001 #12342](https://github.com/Luminous-Dynamics/symthaea/issues/12342)

**Manifest:** benchmarks/plasma/co2-quench-operating-points-v0.json

## Objective

Preserve the identity of every warm-plasma CO2 quenching operating point as the process moves between source papers, physical observations, model predictions, replayable scenarios and eventual product-service comparisons.

The benchmark unit is one exact operating point, not one paper and not one architecture label. A source paper may contain several operating points with different power, flow, conversion, efficiency and product boundaries. The twin must not combine their best values into a synthetic best case.

## Source cases and exact operating points

### Counterflow experiment — Chen et al. (June 2026)

Enhanced CO2 decomposition by plasma-cold gas counter flow, Journal of Environmental Chemical Engineering 14 (2026), DOI 10.1016/j.jece.2026.122322.

The paper reports:
- MAGD-stabilized point at 4 slm and 792 W: 23.6% CO2 conversion and 25.1% energy efficiency.
- A separate low-power highlight at 302 W: 46.2% energy efficiency. The cited highlight does not supply a matching conversion value; conversion is therefore recorded as unknown.
- A separate parameter-sweep point at counterflow ratio 2.5 and distance 30 mm: 207.3% relative conversion improvement and 203.6% relative energy-efficiency improvement against the source-defined conventional gliding-arc baseline.

The relative improvements are not absolute conversion or efficiency. The 302 W and 792 W points are not interchangeable.

### Particle-assisted catalytic quench — Albrechts et al. (October 2026)

Improving CO2 conversion in warm plasma by suppressing CO-O back-reactions through particle-assisted rapid cooling and catalytic O-O recombination, Journal of CO2 Utilization 112 (2026), DOI 10.1016/j.jcou.2026.103579.

A chemical-kinetic model predicts:
- conventional quench baseline: 49% conversion / 29% efficiency;
- particle-assisted Al2O3 / catalytic O-O recombination: 79% / 47%;
- SO2-assisted gas-phase pathway: 72% / 43%.

These are model predictions, not experimental performance.

### Multidimensional arc quench CFD — Albrechts et al. (2026)

Predictive multidimensional modeling as a tool to optimize quenching designs for improved CO2 conversion in arc plasma, Chemical Engineering Journal (2026), DOI 10.1016/j.cej.2026.176010.

The 2D axisymmetric model couples turbulent flow, heat transfer, electric currents, species transport and chemistry and is compared with nozzle and heat-exchanger experiments. The source reports good agreement under stable takeover operation, with modeled values up to around 20% conversion and efficiency approaching 30% in source-comparable cases. Heat-exchanger predictions exceed 30% efficiency to about 5 eV/molecule; conversion above 50% is conditional on perfect wall cooling.

Perfect wall cooling is an ideal boundary assumption, not demonstrated hardware performance.

### Thermal management — Kaufmann et al. (July 2026)

Temperature management as decisive optimization parameter for plasma-based CO2 splitting and biogas reforming, Results in Chemistry 27 (2026), DOI 10.1016/j.rechem.2026.103513.

The source combines experimental thermal measurements, modeling and energy-flow balance for a magnetically rotating discharge reactor. It reports source-specific radiation losses up to 70% of plasma power, temperature gradients up to 110 K/mm and thermal-gradient scales up to 1000 K/s in the studied configuration. These are not universal warm-plasma constants and do not independently establish CO2 conversion or accepted CO yield.

## Typed evidence and scenario contract

Every source observation must retain exact source case and operating-point identity; evidence class; reactor, feed, pressure, geometry, magnetic/discharge mode, flow and power point; measured versus model-predicted quantities and the original denominator; conversion, selectivity, gas yield, efficiency, product purity and delivery rate as separate metrics; sampling, calibration, uncertainty and raw-artifact identity where available; thermal trajectory, quench configuration, pressure drop, quench-gas / particle / catalyst burdens and heat recovery; complete grid-side energy boundary and omitted/unknown subsystems; explicit comparison status and claim ceiling.

A model with published experimental comparison remains a model result. A source experiment remains a literature observation. A scenario receipt proves replay identity, not physical truth.

## Comparability gate

Two points are not score-comparable until the evaluator can reconcile:
1. gas feed and mixture;
2. pressure and flow;
3. reactor/discharge family and stabilization mode;
4. power boundary (grid-side, power-supply input, or reactor-terminal);
5. specific-energy denominator;
6. CO2 conversion and CO product definitions;
7. sampling and calibration;
8. quench auxiliary gas/particle/catalyst inputs and separation burden;
9. cooling/heat-recovery boundary;
10. uptime, startup, interruptions and maintenance.

Until then, use NOT_COMPARABLE or BLOCKED, not a forced ranking.

## Adversarial controls

The machine-readable manifest preserves eight refusal cases: mixing 302 W / 46.2% efficiency with 792 W / 23.6% conversion; treating 207.3% / 203.6% relative improvements as absolute values; promoting the 79% / 47% kinetic-model prediction to experiment; treating more than 50% conversion under perfect wall cooling as measured performance; transferring warm-NH3 turbulence results to CO2 or vice versa; omitting quench-gas, pressure-drop, particle, catalyst or auxiliary-energy burden; treating one reactor's radiation-loss fraction as universal; and ranking DBD, warm arc and kinetic-model outputs without matched denominators and evidence classes.

## Benchmark targets

Proposed research gates, not achieved results:
- reproduce at least one source-pinned counterflow and one heat-exchanger point with exact operating identity;
- separately validate thermal trajectory, chemistry/species and discharge-stability observables;
- predict held-out quench geometry or operating points within predeclared measurement/model uncertainty, otherwise abstain;
- improve accepted CO service per complete grid input without sacrificing discharge stability or product-quality gates;
- include useful heat recovery, quench-gas demand, pressure drop, separation and maintenance in the whole-system objective;
- preserve independent model-vs-experiment scorecards and do not collapse them into one leaderboard.

## Non-goals

This contract does not define a plasma recipe, gas flow setting, catalyst/particle handling instruction, high-voltage or magnetic actuation command, reactor construction detail, experiment procedure or hardware authorization. It does not claim that Symtropy solves warm CO2 plasma chemistry or CFD.
