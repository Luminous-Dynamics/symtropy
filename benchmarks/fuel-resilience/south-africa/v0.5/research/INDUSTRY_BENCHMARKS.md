# SAFRB v0.5 — industry benchmarking context

**Research cut:** 24 November 2026. Vendor capability statements are vendor-published; they are not independent head-to-head benchmark results.

## Established industry capabilities

| System / project | Established capability | Fair comparison for SAFRB | What SAFRB must not claim |
|---|---|---|---|
| AVEVA PI System | Industrial operational-data collection, cleansing, storage, enrichment, visualisation and delivery. | Reconcile read-only historian observations to batch/tank state and preserve source provenance across organisations. | Replacing a site's historian, control-system connectivity or deployed OT ecosystem. |
| SLB Lumi | Energy-focused data and AI platform connecting subsurface and operations data, domain workflows and AI deployment. | Cross-operator evidence reconciliation, independent replay and physical-quantity/service assurance. | Better petroleum-domain AI than SLB without matched field data and expert evaluation. |
| Google OR-Tools | Production-grade min-cost-flow formulations and solver examples. | Compare our v0.3 baseline against OR-Tools on identical graph instances, then report objective equality, runtime and memory. | Claiming that our small research solver is faster or more scalable. |
| HiGHS | Open-source LP/MILP/QP solver with simplex, interior-point, PDLP and branch-and-cut methods. | Use as a reproducible LP/MILP reference solver for the planning tournament. | Reimplementing a competitive general-purpose mathematical-programming solver as the differentiator. |
| Gurobi 14.0 | Current commercial LP/MIP/nonlinear solver release; vendor reports substantial performance gains over 13.0. | Licensed matched model tests, identical time limits, hardware, optimality gaps and solution quality. | Using Gurobi's internal benchmark statistics as an independent comparison. |
| PyPSA | Open energy-system optimization for generators, renewables, storage, sector coupling, network flows, stochastic scenarios and CVaR risk preferences. | Compare coupled port/backup energy scenarios and stochastic/CVaR planning against an established open energy-system framework. | Claiming state-of-the-art energy-system planning without a matched PyPSA baseline. |
| Aspen HYSYS / AspenTech | Industrial process modelling and engineering simulation in oil/gas/process industries. | Independent process/property-model comparison when SAFRB later models refinery or process-unit behavior. | Replacing validated process simulation with the current fuel accounting kernel. |

## Standards and governance

- **ISO 20815:2026** is the current published edition for production assurance and reliability management across petroleum, petrochemical and natural-gas assets, including midstream and downstream.
- **ISO 23247-5:2026** specifies digital-thread principles for manufacturing digital twins across lifecycle. Reuse provenance, connectivity and lifecycle ideas carefully; do not claim that this manufacturing standard certifies a petroleum deployment.
- **W3C Verifiable Credentials Data Model 2.0** is a Recommendation. It can encode verifiable issuer claims such as quality certificates, inspection reports and calibration records, but a valid signature does not prove the underlying physical measurement.
- **NIST SP 800-82 Rev. 4 Initial Public Draft** was published 21 September 2026 with comments due 30 November 2026. Treat it as draft guidance, not a final requirement. Its expanded treatment of maritime, freight rail, IIoT/cloud convergence, CSF 2.0, asset management and zero trust is relevant to SAFRB's read-only architecture.

## Benchmark policy

1. Keep vendor product claims separate from independently reproduced measurements.
2. Compare exact same source data, state snapshots, physical boundaries and uncertainty intervals.
3. Publish solver gaps, runtime, memory, timeouts and unsuccessful runs.
4. Compare energy-service results in Wh/kWh at the same service boundary; do not use litres as a proxy for useful service.
5. Report both expected and tail performance, plus geographic and service-class breakdowns.
6. Use independent lab or field measurements for validation; model-generated truth must not validate itself.

## Sources

- [AVEVA PI System](https://www.aveva.com/en/products/aveva-pi-system/)
- [SLB Lumi data and AI platform](https://www.slb.com/products-and-services/delivering-digital-at-scale/software/lumi)
- [OR-Tools minimum cost flow](https://developers.google.com/optimization/flow/mincostflow)
- [HiGHS documentation](https://ergo-code.github.io/HiGHS/stable/)
- [Gurobi 14.0 release notes](https://www.gurobi.com/product/whats-new)
- [PyPSA stochastic optimization](https://docs.pypsa.org/latest/user-guide/optimization/stochastic/)
- [ISO 20815:2026](https://www.iso.org/standard/89174.html)
- [ISO 23247-5:2026](https://www.iso.org/standard/87425.html)
- [W3C Verifiable Credentials 2.0](https://www.w3.org/news/2025/the-verifiable-credentials-2-0-family-of-specifications-is-now-a-w3c-recommendation/)
- [NIST SP 800-82 Rev. 4 IPD](https://csrc.nist.gov/pubs/sp/800/82/r4/ipd)
