---
title: Wood Cut-Plan Reference Evaluator
version: 0.1
status: synthetic-reference-implementation
date: 2026-10-17
---

# Wood Cut-Plan Reference Evaluator v0.1

## Why another benchmark?

A real sawmill planner must eventually handle log taper, out-of-round geometry, opening-face selection, internal defects, sawing variation, target oversizing, grade rules, edging/trimming, machine sequence and drying. A full 3D log optimizer is not an honest first step without measured geometry and a qualified grade model.

This slice therefore tackles a smaller but executable problem: **exact one-dimensional cutting of already squared stock to satisfy bounded product demand**. It is a lower-level reference contract for lengthwise cut planning, not a log breakdown simulator and not a replacement for existing sawmill tools.

## Research rationale

- The US Forest Service's 3D log-sawing studies used log geometry, defect modeling, sawing algorithms and grading to optimize lumber value rather than raw board count. One study reported value recovery improvements on its central-Appalachian hardwood dataset; those percentages must not be transferred to this implementation: https://research.fs.usda.gov/treesearch/39811
- The Forest Service's 2024 LORCAT validation compared simulation against 42 hardwood logs and found no significant difference in total sawn volume, but significant differences in board counts related to opening-face board-size targeting. This is an important benchmark lesson: volume alone is insufficient; board-count/size distribution and baseline calibration matter: https://research.fs.usda.gov/treesearch/68788
- The 2025 LORCAT user guide describes a tool for studying relationships among log input, process parameters, lumber and by-product recovery. We should compare on fixed, attributable data and include kerf, size variation, log geometry, grade and residual pathways as later phases: https://research.fs.usda.gov/treesearch/69793
- FAO describes cascading use and circularity through waste prevention, reuse, recycling, value addition and improved biomass utilization. Our benchmark therefore keeps reusable offcut, unrecoverable residual and kerf loss separate: https://www.fao.org/forestry-fao/industries/98953/en/

## Paradigm shift

**Do not find one globally "best" cut from a single score. Generate an auditable feasible set, expose the Pareto frontier, and apply caller preferences only afterwards.**

The evaluator enumerates product counts for each stock item subject to exact integer dimensions, saw kerf, demand caps and structural-grade evidence gates. Every stock item is accounted for as product lengths, kerf loss, a reusable offcut, a small/unusable residual, or unprocessed stock. No silent mass/length balancing is permitted.

### Scope

Inputs are already-squared stock lengths in integer millimetres. A cut pattern is one-dimensional: pieces are ordered only by aggregate SKU counts, not by a physical cutting sequence. Products that claim a structural application are eligible only when their evidence state is `qualified` and the declared inspection passes. This code does not determine or certify a grade; it consumes the supplied evidence gate.

The exact solver is intentionally bounded:
- at most 12 stock pieces;
- at most 6 product types;
- at most 12 requested units per product;
- a fixed search-node budget;
- integer arithmetic with overflow rejection.

An exhausted search budget returns an error and no partial frontier. It must not return an apparently optimal answer from an incomplete search.

## Metric vector and Pareto rule

Each feasible plan reports:
- sale value in synthetic microcredits;
- product length fulfilled;
- piece count and demand fulfilled;
- reusable offcut length;
- unusable residual length;
- kerf loss;
- unprocessed stock length;
- product counts and per-stock patterns.

The Pareto objectives are: maximize sale value, maximize product length fulfilled, maximize reusable offcut length, and minimize combined kerf plus unusable residual. Demand cap and structural-use gates apply before optimization. Unprocessed stock is disclosed but is not assigned an invented positive or negative value. The receipt also includes a transparent greedy comparator that repeatedly takes the eligible, in-demand product with the highest declared synthetic value-per-length that fits. This comparator is intentionally simple; it is a reproducible baseline, not an industry-optimal policy.

This frontier preserves economically meaningful choices. It does not invent prices for residuals, imply that all offcuts have a real market, or hide preferences inside arbitrary weights.

## Benchmark gates

1. Every stock piece closes exactly in millimetres: product lengths + kerf loss + reusable offcut + unusable residual + unprocessed stock = input length.
2. Demand caps are never exceeded.
3. Structurally designated products lacking qualified grade evidence or passing inspection cannot be produced as fulfilled structural demand.
4. A product count is feasible only if its required length and inter-piece kerf fit the stock.
5. Every option uses integer quantities; overflow or unknown fields reject the input.
6. The exact same bytes and source revision produce byte-identical normalized receipts.
7. Pareto output contains no strictly dominated plan and keeps distinct SKU mixes visible when they produce different demand/value outcomes.
8. If the bounded search limit is exhausted, the run rejects explicitly rather than implying an optimum.
9. A comparison against a greedy baseline uses the same stock, kerf, demands, value inputs, grade gates and objective definitions.
10. No runtime pass is described as evidence of real mill performance.

## Future measured benchmarks

After the reference implementation passes its own contracts, compare baseline and candidate on the same frozen, licensed corpus. Report:
- value recovery and eligible product length by SKU;
- output piece/length distribution and demand fulfilment;
- kerf loss, reusable offcuts and other residuals separately;
- cut count and runtime / search nodes, median and p95;
- infeasible-plan rejection and structural-evidence leakage;
- robustness when kerf or stock lengths vary within measured uncertainty.

The next model must incorporate log geometry, taper, defects, opening-face choice, target oversizing, sawing variation, grading/inspection rules and real product specs. Calibrate it against measured mill logs and observed outputs before claiming improvements. The 2011 study's reported gains are prior research context, not an acceptance threshold for these synthetic cases.

## Cross-stack responsibilities

- **Sol-Atlas:** sourced and time-bounded spatial context; no map inference grants harvest authority.
- **Mycelix:** batch lineage, process/custody attestations and append-only corrections.
- **Symtropy:** deterministic cut-pattern reference, later process/lifecycle simulator and baseline comparison.
- **Symthaea:** proposes hypotheses and explains the evaluated frontier; it may not override eligibility or claim the reference result proves a physical-world outcome.
