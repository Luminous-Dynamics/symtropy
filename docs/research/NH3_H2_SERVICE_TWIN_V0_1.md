# NH3-H2 service twin v0.1

**Status:** Synthetic-only process-accounting contract and route-comparison harness. No Luminous process result, operating recipe, control law, or experimental authorization is claimed.  
**Parent programme:** https://github.com/Luminous-Dynamics/symtropy/issues/5627  
**Scientific benchmark:** https://github.com/Luminous-Dynamics/symthaea/issues/12503  
**Dynamic service target:** https://github.com/Luminous-Dynamics/symthaea/issues/12488

## Decision

The correct comparison unit is not an ammonia reactor or plasma source. It is accepted hydrogen service at a frozen customer boundary: agreed H2 purity and contaminant limits, delivery pressure and mass flow, delivery interval and availability, grid-to-customer energy boundary, NH3 conversion/recovery/recycle/purge/slip, separation, purification, compression, buffering, maintenance, degradation, rejected product and downtime.

This is a common service contract for dissimilar routes, not a universal ammonia reactor model.

## Sources preserved as separate route identities

The Symthaea source register covers at least these distinct configurations:

| Route / source | Evidence class | Source-specific result that must remain separate |
|---|---|---|
| Atmospheric microwave NH3 cracking (2026) | Lab experiment | Three conversion/energy operating points; throughput pairing discrepancy remains unresolved |
| QWCCR microwave resonator (2026) | Lab experiment | 1.31 ms / 1.10 ms ACR diagnostic response; not accepted-H2 delivery response |
| Waveguide MWPJ (2026) | Lab experiment | Ar-balance-gas configuration and relative ~7x efficiency comparison |
| PEDMADS dual membrane (2025) | Lab experiment + TEA | DBD + Ru/SiO2 + Pd membrane + Silicalite NH3-recovery cascade |
| Catalytic membrane reactor (2026) | Numerical process study | 85.4% base configuration and separate 87.5% higher-permeate-pressure / larger-area configuration |
| Electrochemical MEA (2026) | Institution-reported result | High-throughput and low-energy modes are separate; cell energy is not complete plant energy |
| Packed-bed membrane optimization (2026) | Numerical study | >98% conversion/recovery and ANN metrics are model outputs, not physical performance |

The source records are linked by exact source-case and operating-point identity. They are not treated as scoreable plant scenarios merely because a paper reports one conversion, energy, selectivity or rate.

## Why the comparison is difficult

The 2026 microwave cracking paper contains a source discrepancy: its abstract and conclusion associate the 150 g/h production rate with different conversion and energy-yield values. The register preserves the distinct stated points and blocks exact throughput-point pairing pending table, figure and supplementary reconciliation.

The 2025 PEDMADS study reports below 4 vol% NH3 after its five-stage recovery cascade; that is residual NH3 effluent, not accepted H2 purity. Its LCOH is modeled, not a realized plant price.

The catalytic membrane study's 85.4% and 87.5% energy-efficiency values belong to different permeate-pressure and membrane-area configurations. The MEA's 9.38 and 2.03 kWh/kg H2 values belong to distinct electrical modes. No value is copied into another mode.

## Paradigm shifts

### 1. Reactor metrics to delivered service

Maximize accepted H2 delivered per complete plant input, not single-pass NH3 conversion. A lower conversion can win only if NH3 recovery/recycle plus separation and purification deliver better full-system service with acceptable maintenance and losses.

### 2. Plasma speed to customer-side flexibility

Preserve separate response layers: power command, power delivered, reactor state, diagnostic proxy, measured gas, purified H2 and customer delivery. An OES or ACR response in milliseconds is not a customer H2 service response. Dynamic superiority must be measured at the customer boundary with independent H2 flow and purity.

### 3. Component efficiency to full-grid boundary

The benchmark requires complete, non-overlapping grid-side energy meters. Plasma source, power conditioning, cell/reactor, feed conditioning, NH3 recycle, gas handling, membrane/sweep/vacuum, H2 polishing, compression, cooling, controls, standby and startup are explicit. No missing term is zero.

### 4. Single best point to Pareto envelope

Retain conversion, H2 recovery, accepted flow, purity, pressure, grid kWh/kg, transient response, uptime, maintenance, critical materials and risk-adjusted cost as separate dimensions. A scalar score cannot hide a failed product contract or unknown energy boundary.

### 5. Coproduct and heat credit to qualified useful product

No credit for recovered NH3, useful heat, coproducts or simultaneous electrical output without measured quantity, quality, receiving duty/customer, complete metering and uncertainty.

## Process accounting

The fixture models the stoichiometric identity:

2 NH3 -> N2 + 3 H2

Each scenario independently reconciles NH3 input against reacted, recovered, purge and slip mass; reaction mass against generated H2 and N2; H2 generation against accepted/rejected/unrecovered H2; N2 generation against recovered/purge streams; accepted H2 against product-side measurement; product purity against H2/gross stream mass; and grid energy against non-overlapping subsystem meters.

The fixture uses synthetic quantities solely to test the arithmetic. These are not published operating points or process settings.

## Comparability contract

Two routes are comparable for accounting only when NH3 feed contract, accepted H2 contract, grid-side energy boundary and duty-cycle identity match. If any differ, the comparison is NOT_COMPARABLE. If either route has incomplete product or energy evidence, it is BLOCKED. Even a passing synthetic comparison is labelled SYNTHETIC_COMPARABLE_ACCOUNTING_ONLY; it cannot establish a physical winner.

## Dynamic service

A claimed dynamic H2 service result requires synchronized event identities for power command, measured grid-side power, reactor/conversion state, diagnostic proxy, measured gas, purified H2, customer-accepted H2 and customer delivery settling.

The evaluator refuses a diagnostic proxy promoted to customer response. It separately checks monotonic event time, product-flow overshoot, off-spec duration and product-side purity.

## Prospective research goals

These are targets, not standards or achieved results:
- same customer H2 specification and grid-side boundary for all route comparisons;
- at least 99% H2 recovery and at least 99.9% purity only if required by the receiving contract;
- at least 10% lower full-grid kWh/kg accepted H2 or risk-adjusted cost than the best matched incumbent, without violating other mandatory gates;
- at least 1,000 hours operation and 100 start/stop or load-following cycles before a durability/dynamic superiority claim;
- at least 95% accepted-product availability over the declared campaign;
- end-to-end accepted-H2 settling time of 1 second as a candidate target, not an inference from millisecond diagnostic response.

## Next tranche

- Resolve the exact microwave paper abstract/conclusion discrepancy from its figure, table and supplementary material.
- Extract complete process flows and energy boundaries for PEDMADS, MEA, catalytic membrane, packed-bed membrane and plasma-only routes.
- Build source-specific uncertainty and degradation records.
- Add route-matched TEA/LCA with a no-credit baseline for unqualified coproducts.
- Add source-reproduced dynamic traces only where raw data are available and authorized.
- Keep physical tests behind separate facility, metrology, environmental, safety and authorization gates.

## Claim ceiling

This artifact is a synthetic process twin and literature-comparison protocol. It proves no Luminous hydrogen yield, efficiency, cost, purity, lifetime, commercial advantage or safety envelope. It contains no operating recipe, hazardous-gas handling procedure, hardware control or experimental authorization.
