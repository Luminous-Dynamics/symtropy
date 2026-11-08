# AER-01 read-only NASTRAN BDF intake v0.1

**Tracking:** [AERO-NASTRAN-READONLY-001 / #4139](https://github.com/Luminous-Dynamics/symtropy/issues/4139)  
**Parent:** [DPW-8/AePW-4 reference intake PR #4137](https://github.com/Luminous-Dynamics/symtropy/pull/4137)  
**Status:** synthetic fixed-field syntax/topology implementation; external NASA source not acquired or admitted.

## Purpose and boundary

This crate is a read-only intake/parser for an explicitly narrow subset of small-field NASTRAN bulk data. It checks field boundaries, continuation association, card inventory and reference topology. It does **not** assemble an element stiffness matrix, calculate displacement/stress, solve SOL 101, validate the CRM FEM, or provide aircraft structural properties.

The actual DPW-8 CRM equivalent-beam file has only been inspected through a browser text preview. Its exact bytes, SHA-256, terms, units, model interpretation and solver result remain unknown. This crate's included BDF is synthetic; its identifiers and numeric values do not come from NASA.

## Admitted v0.1 syntax

Fixed-width small-field rows, up to ten 8-character columns within 80 columns, with explicit + continuation labels only.

Supported typed cards:
- GRID
- CBEAM
- MAT1
- PBEAM (core ID/material/section fields; remaining card fields retained as strings)
- CONM2
- RBE2
- SPC / SPC1
- FORCE
- LOAD
- PARAM

Unknown bulk cards are preserved as opaque raw records and do not contribute to typed topology. Blank implicit continuation, large-field * cards, inline comments and unsupported continuation syntax fail closed.

For every card the parser retains source line numbers, byte spans, raw lines and field strings. It does not turn unknown source semantics into guessed numeric values.

## Integrity and topology checks

- Exact source bytes are SHA-256 checked against the caller-supplied digest.
- Duplicate GRID / element / property / material identifiers are rejected.
- CBEAM, CONM2, RBE2, SPC/SPC1 and FORCE grid references are resolved.
- PBEAM material references must resolve to an admitted MAT1 card.
- LOAD references must resolve to a FORCE set in this narrow subset.
- Numeric inputs must parse to finite numbers; implicit NASTRAN exponents such as 1.25-3 are handled.
- Unsupported large-field cards, malformed/truncated lines, orphan or mismatched continuations and missing ENDDATA are rejected.
- Isolated grids are reported, not automatically rejected.
- Units, material allowables, solver result and structural response remain UNKNOWN / NOT_ASSESSED.

This is a deliberately bounded parser, not a complete MSC/NX/Nastran dialect implementation. Before parsing the real CRM deck, compare its exact card/continuation dialect with the supported subset. Extend only from acquired source bytes and controlling NASTRAN syntax documentation; unknown cards stay opaque.

## Local execution

From the repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/nastran-bdf-intake/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/nastran-bdf-intake/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/nastran-bdf-intake/Cargo.toml > /tmp/nastran-bdf-inventory.json
cargo run --quiet --manifest-path tools/civil-aero-benchmark/nastran-bdf-intake/Cargo.toml -- --crm-shaped > /tmp/nastran-bdf-crm-shaped-inventory.json
python3 tools/civil-aero-benchmark/nastran-bdf-intake/verify_independent.py \
  --rust-output /tmp/nastran-bdf-inventory.json \
  --rust-shape-output /tmp/nastran-bdf-crm-shaped-inventory.json
~~~

The independent Python oracle parses the same synthetic fixed-field fixture and checks the Rust output. It is not imported by the Rust crate.

A second fixture, `data/synthetic-crm-equivalent-beam-shape.bdf`, scales the topology to the 180-line NASA browser preview's visible inventory counts (49 GRID, 16 CBEAM, 16 PBEAM, 15 CONM2, 16 RBE2, one SPC and one FORCE). It uses invented coordinates, material values, masses and loads. It is **not** a transcription of the NASA deck and cannot be used as a CRM physical model.

The executable accepts `--crm-shaped` to emit this second inventory. CI runs both fixture paths and independently compares both Rust JSON outputs with the separate Python implementation. The second fixture specifically exercises implicit blank-field continuation on all 16 CBEAM and 15 CONM2 records, plus the optional blank SPC enforced-displacement field.

## Hostile controls

The Rust suite and Python oracle exercise:
- duplicate GRID ID;
- missing CBEAM endpoint;
- missing PBEAM material;
- missing RBE2 dependent node;
- mismatched continuation label;
- stale source digest;
- non-finite GRID coordinate;
- large-field card and over-wide line;
- missing ENDDATA;
- malformed SPC/LOAD field-group boundaries;
- implicit exponent notation.

## Claim ceiling

The only positive output claim is syntax_and_topology_inventory_only. The result is not a FEM solution, not a stress or deflection result, not a validated structural model, not an aircraft safety finding, not flutter/durability evidence, and not certification or permission to manufacture/test/fly.

The next source step remains exact-byte acquisition and terms review of the 8 KB CRM equivalent-beam deck plus its adjacent README. Only after source admission should its actual card dialect be added to the fixture corpus.