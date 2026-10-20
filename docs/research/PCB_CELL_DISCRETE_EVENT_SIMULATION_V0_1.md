# PCB cell discrete-event benchmark v0.1

**Benchmark family:** `PCB-CELL-001`  
**Simulation owner reused:** [EV-FACTORY-SIM-001 / PR #2372](https://github.com/Luminous-Dynamics/symtropy/pull/2372)  
**Status:** synthetic design-screening fixtures; no physical calibration  
**Authority:** no machine actuation, production release, supplier contact, or safety/quality certification

## Why extend the existing simulator?

Use the existing deterministic Rust discrete-event engine rather than creating a second queueing/buffer/bottleneck engine. The present engine models finite inter-station buffers, variable supplier arrivals and station cycle times, changeovers, first-pass yield, bounded station-local rework, breakdown/repair intervals, and baseline versus one-extra-server counterfactuals.

These fixtures are a first domain-specific consumer. The present runner lives under the EV cost-frontier package; this is pragmatic reuse, not the final ownership boundary. If more manufacturing domains use it, the long-term improvement is to extract a domain-neutral DES library with versioned input/output contracts, then keep PCB and vehicle semantics in their own profiles.

## Route profiles

### P0 — outsourced bare board, local assembly

The supply stream represents arrival of a bare board and its declared component kit from an external supplier. The modeled stations are incoming-board/kitting, paste application, SMT placement, reflow soldering, and inspection/electrical test/bring-up.

This route does **not** claim local bare-board fabrication. External supplier capability, queue, transit, lot acceptance and cost are not included in the current simulation engine; represent those separately until their data model is qualified.

Fixture: `tools/ev-passenger-cost-frontier/fixtures/pcb_cell_p0_outsourced_bare_board_v0_1.json`.

### P1 — local isolation milling/drilling, local assembly

The supply stream represents copper-clad stock and components. The modeled stations are stock/datum preparation, isolation milling and unplated drilling, board registration/cleaning/inspection, paste application, placement, reflow and final inspection/test.

This is only a **simple prototype board** route. It does not claim plated-through vias, multilayer boards, controlled impedance, production solder mask, production surface finish, or high-density PCB capability.

Fixture: `tools/ev-passenger-cost-frontier/fixtures/pcb_cell_p1_local_milling_v0_1.json`.

Both profiles use invented cycle times, variability, yields, breakdown/repair intervals, buffers, variants and operating-calendar assumptions. Gates for calibration, local compliance, line safety and product-quality release stay `unknown`; simulated throughput must never release a product.

## Run

```sh
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- \
  tools/ev-passenger-cost-frontier/fixtures/pcb_cell_p0_outsourced_bare_board_v0_1.json
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- \
  tools/ev-passenger-cost-frontier/fixtures/pcb_cell_p1_local_milling_v0_1.json
```

The shared runner's tests now validate both PCB fixtures, repeat each simulation with the same fixed seed, require baseline plus one-extra-server counterfactuals, enforce buffer/output/time invariants, and ensure synthetic/unknown gates leave release false.

The workflow also checks both fixtures and repeats their reports for byte-equivalent determinism. No test pass is claimed here before the updated workflow executes on the exact new head.

## What this benchmark can and cannot answer

**It can screen:** whether assumed station times and finite buffers create bottlenecks; how an additional server changes modeled good output; how much work-in-process accumulates; and where variability, rework and downtime propagate through a linear route.

**It cannot yet answer:** which PCB route is cheaper for the same accepted design, real production yield, delivered lead time, supplier-queue performance, bare-board dimensional conformity, electrical reliability, or safety/compliance. The current DES output does not calculate labour/consumable/tooling/depreciation cost, model separate process graphs in a single run, or represent physical workcell collision/reachability. The P0 and P1 results are not a fair winner/loser comparison until the same frozen board requirement set is confirmed feasible on both routes.

Next additions should be, in order:
1. Bind each run to an exact product/acceptance-profile digest and declare which features are in/out of the route.
2. Add supplier lead-time/backlog and unit-cost accounting as distinct measured/declared inputs, rather than smuggling them into cycle time.
3. Add repeated-seed intervals and common random streams for paired counterfactuals; the current sequential RNG makes one-seed intervention deltas exploratory only.
4. Use the separate [PCB-CELL-001 spatial benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/2385) for workcell geometry, reachability, collision and handling.
5. Calibrate against authorized measured production-event traces; split training/calibration and held-out validation windows.
6. Only after a defined board family has reproducible physical results compare accepted-board cost and lead time against manual bench work and a qualified commercial fabricator/assembly provider.

## Industry comparison

NIST's July 2026 manufacturing digital-twin report identifies interoperability, verification/validation/uncertainty quantification, cybersecurity, and workforce readiness as continuing cross-industry challenges: https://www.nist.gov/publications/digital-twins-workshops-summary-report.

IPC's Electronics Assembly Quality Benchmark study describes industry comparisons using yields, defect rates, DPMO, scrap/rework cost, returns and supplier performance, with segmentation by company size, region and production type. The 2019 study sampled 57 assembly companies; its numerical distributions are in a paid report, so this project does not invent or quote unavailable yield percentiles: https://www.ipc.org/news-release/ipc-releases-2019-quality-benchmark-study-electronics-assembly.

Once physical data exists, use those same types of denominators—first-pass accepted boards / boards started, final accepted boards / boards started, defects per opportunity, returns per shipped unit, cost per accepted board, and on-time completion—then compare distributions and sample size, not just means or the best run.

## Claim ceiling

This v0.1 extension is a **synthetic deterministic flow sensitivity benchmark**. It is not an empirical PCB-factory digital twin, production forecast, price estimate, physical-quality result, factory safety case, capital-investment recommendation, or evidence that local milling outperforms commercial PCB services.
