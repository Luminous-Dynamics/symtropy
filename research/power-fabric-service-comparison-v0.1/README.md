# Power Fabric service-first cross-vector comparison v0.1

**Parent:** [POWER-FABRIC-TRANSPORT-PRIMITIVES-001 #5832](https://github.com/Luminous-Dynamics/symtropy/issues/5832)  
**Implementation issue:** [POWER-FABRIC-SERVICE-COMPARISON-001 #5900](https://github.com/Luminous-Dynamics/symtropy/issues/5900)  
**Foundation branch:** [Electrical Service Graph PR #5188](https://github.com/Luminous-Dynamics/symtropy/pull/5188)  
**Status:** executable metadata contract and source-backed benchmark plan; no physical superiority claim.

## What this slice does

The existing `symtropy-energy-systems` crate now includes a carrier-neutral transport contract:

- `EnergyCarrier`, `TransportMechanism` and `TransportMedium` are distinct types.
- Ordered conversion stages and storage state remain separate from the transport mechanism.
- A transport path composes existing primitive port identities in order. It is not another topology graph and performs no path search.
- A useful-service contract binds service kind, quality profile, duty-cycle digest, scenario digest, location/environment profile and both source/useful-service boundaries.
- A comparison gate returns `Comparable`, `NotComparable` or `Unresolved` with stable reasons.
- Evidence uses the existing `EvidenceClaim` and caller-supplied `EvidencePolicy`; no parallel evidence scale is introduced.

The synthetic test demonstrates that wired conduction, inductive coupling and optical/radiative transfer can be structurally comparable when all three target the same versioned regulated-DC service under the same declared scenario and boundaries. It contains **no performance numbers**.

## What it deliberately does not do

A `Comparable` result means only that the comparison contract matches. It does not mean that the candidates have equal performance, are physically validated, safe, compatible to energize, standards-compliant or superior to the state of the art.

The contract does not:
- calculate efficiency, exergy, cost or lifecycle metrics;
- create a new energy ledger or duplicate the WPT metrology comparator;
- create an electrical or cross-vector topology graph;
- implement EM, thermal, acoustic, plasma, fluid, chemical or grid solvers;
- select routes, dispatch energy or authorize energization;
- admit scalar waves, free energy or nonlocal transmission as a transport primitive.

Numeric observations and end-to-end accounting remain with the existing WPT measurement and energy-accounting owners (#3469, #3472); electrical topology remains with #5183; modality-specific physics remains with its existing owner.

## Refreshed industry reference envelope (checked 2026-11-17)

These values describe different system boundaries and must not be combined into one ranking.

| Lane | Public reference | What was actually reported | Comparison caveat |
|---|---|---|---|
| High-power IPT | Riekerk et al., 50 kW bus prototype, IEEE JESTPE 2026 | 97.165% to 96.824% DC-to-DC efficiency under 100 mm X/Y misalignment; 13.86 µT stray field | This is a DC-to-DC converter boundary, not full grid-to-battery or lifecycle efficiency. [Paper DOI](https://doi.org/10.1109/JESTPE.2025.3631233) |
| Dynamic roadway IPT | VINCI / Electreon A10 motorway pilot, 1.5 km, 24 Oct 2025 | More than 300 kW instantaneous and more than 200 kW average under optimal steady-state conditions | The announcement does not provide a matched whole-route efficiency or all-weather availability figure. [VINCI announcement](https://www.vinci.com/en/newsroom/news/world-first-dynamic-wireless-charging-motorway) |
| Optical / laser | NTT / MHI, 1 km, 17 Sep 2025 | 1 kW optical output, 152 W receiver electrical output, 15% optical-to-electrical efficiency, 30 min operation | The 15% is optical-output-to-receiver-electricity, not wall-plug-to-load. Equipment was enclosed in safety booths. [NTT announcement](https://group.ntt/en/newsrelease/2025/09/17/250917a.html) |
| Optical / laser | DARPA POWER, 8.6 km, 16 May 2025 | More than 800 W delivered for 30 s; more than 1 MJ over the campaign | The public announcement does not give a directly comparable end-to-end electrical efficiency. [DARPA announcement](https://www.darpa.mil/news/2025/darpa-program-distance-record-power-beaming) |
| Acoustic / underwater | Liao et al. (2025) | Reported prototype at 35 cm, 2.5 W transmitter and 433 mW received (17% as indexed) | Verify the primary paper's exact input meter boundary before treating 17% as end-to-end efficiency. [DOI](https://doi.org/10.24425/aoa.2025.154821) |
| Cross-modal systems | Li et al., Nature Reviews Electrical Engineering, 2 Apr 2026 | Review identifies direct wireless conversion, primary-side control and simultaneous power/data as important directions; near-field currently dominates practical power and efficiency | This supports service-first comparison, not a claim that multi-modal routing is already solved. [Review](https://www.nature.com/articles/s44287-026-00279-6) |

A 2026 survey screened 133 peer-reviewed experimental IPT/CPT papers from 2020–2025. It is useful as a source index, but it is not a universal apples-to-apples leaderboard. [Survey](https://www.mdpi.com/2079-9292/15/8/1575).

## Standards and industry constraints

- **IEC/IEEE 63184:2025** is an active exposure-assessment standard for stationary and dynamic WPT; its initial scope covers 1 kHz–30 MHz and focuses on inductive WPT. It is not itself an efficiency or product-certification standard. [IEEE Standards page](https://standards.ieee.org/ieee/63184/10156/).
- **IEEE P3735** is an active PAR for a radiative wireless-power-transfer standard; the project is in development, not an approved standard. [IEEE project page](https://standards.ieee.org/ieee/3735/12230/).
- Optical systems require a separate laser-safety case, enclosure/beam-termination strategy and hard interlocks. RF, acoustic, thermal, chemical and plasma systems likewise need modality-specific hazard controls.
- The 2026 Nature review highlights a need for standardized energy and information interfaces. Power Fabric should test this as an interoperability contribution, not claim to have invented wireless power, direct conversion or power/data transfer.

## Proposed benchmark gates

These are programme targets, not achieved results or regulatory limits.

### Contract and comparison correctness

- 100% of frozen known-mismatch cases return `NotComparable`.
- 100% of missing-profile or inadmissible-evidence cases return `Unresolved`, never `Comparable`.
- Zero silent substitution between carrier, mechanism, medium, conversion and storage.
- Stable results independent of candidate input ordering.
- Zero performance scores or rankings emitted by this metadata gate.
- Every synthetic reference candidate binds an exact source boundary, useful-service boundary, service-quality profile, duty cycle, environment and scenario.

### Modality-specific physical targets

| Lane | Reproduction gate | Stretch research target |
|---|---|---|
| IPT stationary | Reproduce a 50 kW-class 96.8–97.2% DC-to-DC reference at its declared 100 mm misalignment conditions | At least 97.5% DC-to-DC at the same meter planes and misalignment, without worse thermal, field, stress or availability margins |
| Dynamic roadway | Reproduce the reported >300 kW instantaneous / >200 kW average service under a matched route and duty cycle | >220 kW average over the same declared route/service with better route availability and measured end-to-end energy accounting |
| Photonic 1 km | Reproduce 152 W receiver electrical output for 30 min and independently reconstruct the optical-to-electrical boundary | ≥18% optical-output-to-useful-DC efficiency with no reduction in delivered power/duration and a complete laser-source energy boundary |
| Photonic 8.6 km | Reproduce >800 W for 30 s with receiver and source boundaries measured | 1 kW for 60 s, with full source-to-load energy and safety evidence |
| Acoustic subsea | Reproduce the 35 cm reference after verifying the primary paper's metering boundary | Improve range or useful receiver service while preserving cavitation, thermal and marine-environment constraints |
| Plasma | Reproduce published channel length/lifetime/guidance observations in a source-matched model/reference study | No efficiency target until total laser/channel-formation/sustainment energy and useful-load energy are measurable |
| Scalar-wave claims | Reproduce a known-truth Maxwellian discrimination corpus and preserve unresolved outcomes | No new-carrier target; a new physical channel would require blinded independent replication and complete energy accounting |

### System-level advantage

The actual Luminous differentiator should be tested at the **same useful-service boundary** across a set of candidate paths:

- service availability and continuity;
- useful exergy delivered per total source exergy;
- path switching interruption and recovery;
- receiver capture / alignment tolerance;
- degradation and environmental robustness;
- calibration/evidence coverage;
- lifecycle cost and maintenance burden;
- safety and hard-constraint violations.

A proposed stretch goal is at least 5% improvement in net useful-exergy delivery over the best matched architecture in a declared operating region, with uncertainty intervals excluding parity. Do not average unlike thermal, optical, mechanical and electrical services into one global score.

## Why this is a paradigm shift

The field already has strong modality specialists: high-efficiency IPT, dynamic roadway charging, optical power beaming, and specialized acoustic systems. Power Fabric should not try to beat each discipline by claiming a universal transmitter.

Instead, test whether it can become the **service-level interoperability and evidence layer** that:
1. describes each physical modality without collapsing its physics;
2. composes candidate routes without inventing a new solver or ledger;
3. verifies that candidates perform the same useful service;
4. attaches the correct modality-specific measurements and safety case;
5. exposes trade-offs and unknowns instead of hiding them in one efficiency number.

## Reproduction

From the repository root:

```bash
cargo fmt --manifest-path crates/domains/symtropy-energy-systems/Cargo.toml -- --check
cargo clippy --manifest-path crates/domains/symtropy-energy-systems/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path crates/domains/symtropy-energy-systems/Cargo.toml --locked --lib
```

The unit tests are authored but have not been executed in this authoring environment. The dedicated workflow is intended to provide exact-head Rust validation on the dependent branch.

---

**Claim ceiling:** contract and synthetic-fixture correctness only. No physical performance, safety, certification, interoperability in hardware, or SOTA superiority is established by this artifact.


## Integration ownership and next quantitative stage

This metadata gate is intentionally not an optimizer or numeric performance comparator. The next stage composes existing owners:

- [#3421 WELL-TO-SERVICE](https://github.com/Luminous-Dynamics/symtropy/issues/3421) supplies primary-energy and lifecycle accounting for matched services.
- [#3453 WPT-PARETO-SAFETY](https://github.com/Luminous-Dynamics/symtropy/issues/3453) supplies interval-aware, constraint-first feasibility and robust Pareto dominance. Its Feasible / Infeasible / Uncertain / NotEvaluable states must remain distinct; hard safety constraints must not be traded away by weighted scoring.
- [#3469 typed WPT metrology](https://github.com/Luminous-Dynamics/symtropy/pull/3469) and [#3472 source-to-load accounting](https://github.com/Luminous-Dynamics/symtropy/issues/3472) supply numerical measurement planes, loss partition and energy-balance evidence.
- [#3680 direct wireless conversion](https://github.com/Luminous-Dynamics/symtropy/issues/3680) owns wireless delivery to mechanical, thermal, optical and chemical service endpoints.
- [#3559 robust multi-receiver dispatch](https://github.com/Luminous-Dynamics/symtropy/issues/3559) owns WPT-specific dispatch optimization.

The intended chain is: this PR establishes matched-service metadata; modality owners provide measured/uncertain metrics; #3453 filters feasible candidates and computes robust Pareto relations; #3421 adds primary-energy/lifecycle context; Symthaea may propose routes using these qualified inputs but cannot certify its own physical predictions or override safety.

A synthetic `Comparable` result from this crate is not evidence of an energy-efficiency win.
