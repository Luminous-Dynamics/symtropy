# EV-CARRIER-QUALIFICATION-001 — Evidence-bound qualification chain v0.1

**Implementation:** `tools/ev-passenger-cost-frontier/src/carrier_qualification.rs`  
**CLI:** `ev_carrier_qualification_screen`  
**Input 1:** carrier engineering screen report  
**Input 2:** EV carrier module factory process report  
**Input 3:** configuration-bound requirement/evidence matrix  
**Output:** qualification completeness report with source report SHA-256 commitments  
**Claim ceiling:** evidence-completeness review only; never product release authority

## Why this increment exists

The preceding engineering and manufacturing tools answered separate questions:

- Can the base EV and the incremental carrier module fit the declared mass, battery-energy, power and site-energy envelope?
- Can the declared process route produce the target module volume using the available parallel cells, staffing, floor area, equipment and utility assumptions?

Neither question establishes that the physical product has passed its required tests, that the manufacturing process is qualified, that the installed carrier configuration is traceable, or that an external authority has approved the vehicle/aircraft/trailer.

This chain joins those records to an explicit qualification matrix. It is deliberately an evidence-completeness screen—not a certification engine.

## Architecture

```text
EV vehicle design report
          |
carrier configuration screen
          |
          v
carrier engineering report -----------------------+
                                                    |
EV factory design report                            |
          |                                         |
carrier module process screen                       |
          |                                         |
          v                                         |
module factory capacity report --------------------+
                                                    |
configuration-bound qualification matrix ---------+
                                                    |
                                                    v
                            source-hash-bound qualification report
                                                    |
                                                    v
                         independent human / authority release process
```

The tool does not replicate the EV mass/energy/power model or factory station-sizing model. It consumes their reports, checks their configuration identifiers and status semantics, and adds the evidence gate that those screens do not own.

## Configuration and evidence identity

Each qualification case binds:
- carrier configuration ID;
- base vehicle design ID;
- product architecture;
- configuration SHA-256 digest;
- optional module-process configuration ID;
- whether a separate module process route is required;
- a versioned list of qualification requirements and evidence records.

Each qualified evidence record must bind to the exact configuration digest. A test performed against another revision, another pod, a different installed battery or an obsolete aircraft/firmware configuration cannot silently qualify the current product.

The three source files are hashed as raw bytes and their SHA-256 values are included in the output report. This proves which exact input bytes the report consumed. It does **not** authenticate the source, prove who issued an evidence receipt, validate a signature, establish a physical measurement, or confer a legal authority. Those remain independent evidence/identity and external-authority responsibilities.

For production integration, the configuration digest must be generated from the canonical engineering configuration identity owned by Mycelix Mobility Commons #3709. The synthetic fixture's digest strings are test scope tokens, not release artifacts.

## Evidence classes and gates

Required evidence classes are explicit and are not converted into a numerical score:

- `measured`: a source-bound measurement, including method and scope;
- `independent_test`: an independently witnessed or qualified test result;
- `external_authority`: an approval/decision issued by the relevant authority;
- `synthetic`: a test/model input that only validates arithmetic;
- `unknown`: evidence not supplied or not admitted.

A measured result cannot satisfy an independent-test requirement. An independent test cannot substitute for an aviation, road-vehicle or trailer approval issued by an external authority. Synthetic inputs never satisfy a qualification gate.

The fixture defines 67 requirement rows across six configurations, including:
- gross mass, axle/centre-of-gravity and braking;
- structural mounting, retention and fatigue;
- cargo restraint and access;
- HV isolation and electrical protection;
- thermal/fire/venting/propagation;
- vibration and environmental ingress;
- EMC, cybersecurity and diagnostics;
- end-of-line process testing and configuration traceability;
- installation/removal/repair time study;
- road-vehicle homologation;
- architecture-specific aviation authority, trailer approval and AMR functional-safety requirements.

Each case has an explicit requirement list. An architecture does not inherit a blanket `not applicable` status for a requirement merely because the data are absent.

## Disposition semantics

- `blocked`: a required gate explicitly failed, a configuration scope mismatch occurred, or a module process route is known to be blocked.
- `indeterminate`: a gate is unknown, the evidence class is insufficient, the carrier engineering report is synthetic/incomplete, a module process route is missing or synthetic, or the base factory is not production-release-ready.
- `evidence_complete_for_human_review`: all declared gates and upstream evidence screens meet their declared evidence class and scope. This is only a handoff to a competent human/authority release process.

There is no `released`, `certified` or `airworthy` output state.

The report hard-codes both:
- `release_authority_granted = false`
- `production_release_ready = false`

Even an evidence-complete case is only prepared for independent human review. Product release must remain owned by the designated manufacturer/operator and relevant external authority.

## Current fixture result contract

The test fixture has six architectures:
1. urban truck-only control;
2. light-commercial van-only control;
3. integrated enclosed drone bay;
4. detachable drone pod;
5. mobile drone-station trailer;
6. AMR cargo handoff.

The first two have no incremental module process route. The pod, integrated bay and AMR map to their corresponding module process configurations. The trailer route is deliberately not yet modelled in the process catalogue and therefore remains indeterminate.

The carrier engineering report fixture is explicitly synthetic. The populated pod and AMR factory process routes are also synthetic; the integrated bay has unknown manufacturing inputs. All 67 product qualification gates are unknown.

The workflow therefore requires:
- exactly six qualification results;
- 67 declared gates;
- all six dispositions to be `indeterminate`;
- zero release-authority grants;
- zero production-release-ready results;
- the pod process screen to account for 36 factory scenarios;
- the mobile trailer to remain `route_not_modelled`.

This is a negative-control expectation: the system must not turn a plausible architecture or a synthetic factory capacity pass into a product release.

## Run

First generate the base factory and carrier module factory reports, then run:

```bash
cargo run --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_carrier_qualification_screen -- \
  carrier-engineering-report.json \
  ev-carrier-module-factory-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_carrier_qualification_v0_1.json
```

The CLI hashes the three input files exactly as read and emits a deterministic JSON report. The workflow uses a synthetic carrier-report contract fixture for interface validation; production integration must replace it with the exact report emitted by the carrier engineering screen.

## Benchmark programme

The immediate benchmark is evidence correctness rather than optimizer performance:

| Metric | v0.1 acceptance target |
|---|---:|
| Configuration/result coverage | 100% exact ID coverage |
| Evidence subject configuration match | 100% or reject |
| Source report SHA-256 commitments | 3/3 present and valid |
| Failed required gate promoted to qualified | 0 |
| Synthetic evidence promoted to physical qualification | 0 |
| External authority requirement satisfied by an internal test | 0 |
| Unknown gate silently treated as passed | 0 |
| Release authority emitted | 0 |
| Production release asserted | 0 |
| Deterministic replay on identical source bytes | 100% |

Further qualification metrics should include evidence review latency, change-impact closure time, false rejection rate, missed requirement rate, evidence renewal backlog, substitution/requalification coverage, maintenance-triggered revalidation time, and independent-verifier reproducibility.

## Industry / standards alignment

The qualification chain should connect existing digital engineering standards rather than replace them:
- STEP/AP242 for product definition and configuration exchanges;
- QIF for dimensional/quality inspection and measurement results;
- MTConnect or a suitable machine/process interface for manufacturing telemetry;
- Mycelix Mobility Commons for configuration identity, physical artifact lineage, substitution, custody and maintenance;
- Mycelix Battery Passport projection for category/date/access-controlled battery fields;
- Sol-Atlas for sourced site/supplier/route context, without treating spatial suitability as regulatory approval.

The European Commission's published Digital Product Passport timeline states that the Battery Passport applies from 18 February 2027 to specified EV, light means of transport and industrial batteries. Applicability must be resolved for the actual pack/category and intended use; this qualification tool neither determines legal scope nor creates a passport. Source: https://single-market-economy.ec.europa.eu/single-market/digital-product-passport_en.

The IEA's Global EV Outlook 2026 reports that global electric-truck production reached around 440,000 in 2025, with more than 90% concentrated in China, and notes that Daimler and Volvo can assemble battery-electric trucks on the same lines as diesel vehicles. Our factory comparison therefore needs to include mixed-model integration and flexible assembly, not assume a dedicated EV plant is automatically superior. Source: https://www.iea.org/reports/global-ev-outlook-2026/manufacturing-and-trade.

## Ownership

- Symtropy EV carrier design #6109 owns physical configuration and mass/energy/power interfaces.
- Symtropy EV carrier module manufacturing #6176 / PR #6193 owns module process capacity and production-resource assumptions.
- Symtropy EV validation #2114 owns vehicle/module verification stages.
- Mycelix Mobility Commons #3709 owns domain-neutral configuration identity, substitution and physical artifact lifecycle lineage.
- Mycelix Battery Passport #6062 / refresh #6076 own regulatory passport projection and refresh planning.
- Symthaea measurement/oracle components can help assess and explain evidence, but cannot self-authorize release.
- Sol-Atlas provides site/supplier/rights context, not product certification.

## Non-claims

This v0.1 does not prove a physical test occurred, authenticate a test lab, establish supplier capability, certify fire or electrical safety, qualify a manufacturing process, approve a trailer, issue road homologation, establish aircraft airworthiness, authorize flight, register a battery passport, or grant production release. Its synthetic fixture exists to prove that incomplete evidence remains incomplete.
