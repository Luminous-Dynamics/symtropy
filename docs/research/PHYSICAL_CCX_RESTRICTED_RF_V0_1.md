# CalculiX RF restricted-equivalence normalizer v0.1

**Status:** synthetic parser and semantic-proof tests only. No external CCX binary has been invoked or qualified by this module.  
**Related gates:** [PHYS-CCX-OUTPUT-001 #2468](https://github.com/Luminous-Dynamics/symtropy/issues/2468), [PHYS-CCX-RF-RESTRICT-001 #2477](https://github.com/Luminous-Dynamics/symtropy/issues/2477), and [external solver qualification #2321](https://github.com/Luminous-Dynamics/symtropy/issues/2321).

## Why there are two reaction-output paths

CalculiX documents RF as an external-force quantity that can combine prescribed reactions with concentrated and distributed loading. It is therefore not universally a support-reaction variable. See the upstream manual sections on [RF semantics](https://feacluster.com/CalculiX/ccx_2.18/doc/ccx/node201.html), [NODE PRINT](https://feacluster.com/CalculiX/ccx_2.18/doc/ccx/node307.html), and [NODE FILE](https://feacluster.com/CalculiX/ccx_2.18/doc/ccx/node305.html).

The separate RR parser requires an explicit reaction-only RR request and an RR output section. This module is a different, stock-CCX-compatible route: it proves a small allowlisted deck has no semantic feature that makes RF differ from support reaction for the declared support nodes.

The intended statement is conditional:

> For this exact deck only, every declared support node has prescribed translational constraints, no CLOAD is applied to any support node, the deck has no distributed/body/pressure/thermal/follower load or unsupported constraint transformation, and the only applied loads are request-matched point forces on disjoint free nodes. Under those restrictions, the selected RF values may be normalized as support reactions.

If a condition cannot be proven from the exact deck/request/catalogue bytes, the normalizer rejects the case. This is not a blanket reinterpretation of RF.

## Strict envelope

Version 0.1 accepts only a narrow single-step, small-strain static linear-elastic T3D2 bar/truss deck in SI units and one explicit global coordinate frame. The deck parser uses a positive allowlist. Supported cards are limited to heading, nodes, T3D2 elements, explicit node sets, one isotropic linear-elastic material, a matching positive truss area, zero-valued translational boundary conditions, one point-force CLOAD inventory, one STATIC step and an RF-only NODE PRINT request for the expected support node set.

It rejects unrecognized keywords—including DLOAD, DSLOAD, GRAV, pressure/surface loads, TRANSFORM, SYSTEM, EQUATION, contact, dynamics and nonlinear keywords—rather than ignoring them. It rejects nonzero prescribed displacements, rotational DOFs, repeated or malformed node/load rows, duplicate set definitions, missing/unexpected node IDs, ambiguous output sections and any non-finite numerical field.

This allowlist intentionally has low breadth to make the equivalence argument tractable. Extending the allowed grammar is a physics-semantics change that needs its own test and review.

## Exact evidence bindings

The normalizer binds the manifest to the byte digests of the request, .inp deck and node catalogue. The catalogue is itself bound to request, scene revision, coordinate-frame ID and exact deck digest. Support-node positions are cross-checked between the independent catalogue and the coordinates parsed from the deck.

The manifest maps every request support target to a node ID and every point-load target to a node ID. The request load inventory is translated to CCX CLOAD degrees of freedom 1, 2 and 3 and compared to every deck CLOAD row. The load-node and support-node inventories must be disjoint. Duplicate load DOFs, unknown nodes or any exact-value mismatch are unresolved.

The parser selects only one ordinary forces (fx,fy,fz) section for the exact support set, checks its complete node inventory, retains raw source lines and line numbers, and rejects a reaction-only RR section on this path. It makes no implicit unit or coordinate transformation. The normalized payload can be passed to the existing equilibrium auditor, which still requires independent reference and mesh evidence.

## CLI

Given exact request, input deck, raw .dat output, a restricted-RF manifest and a deck-bound support-node catalogue, run the physical-solver-contract CLI with:

- --request request.json
- --deck model.inp
- --dat model.dat
- --node-catalog support-nodes.json
- --restricted-rf-manifest restricted-rf-manifest.json

The result is a machine-readable normalization report with source semantics, exact digests, the support/load node inventories, the conditions that were proved, raw RF rows and a normalized reaction-force payload. This CLI mode does not authenticate that CCX produced the .dat file; the calling qualification harness must bind the executable identity, invocation, exit status and raw output through its receipt.

## Synthetic acceptance controls

The included tests cover a clean axial-bar fixture, disallowed distributed-load and frame-transform keywords, unknown cards, wrong output set, a reaction-only section accidentally supplied to the stock-RF path, stale deck digest, load/support confusion and unexpected output-node IDs.

Targets before a real backend can be qualified:
- Zero false PASS outcomes over a frozen deck-semantic mutation corpus.
- Every successful report binds exact request, deck, node catalogue and .dat bytes.
- No unknown input card may be ignored.
- Every normalized reaction row maps to a known support node and exact catalogue coordinates.
- A real pinned CCX build must then pass analytical-reference agreement, independent equilibrium and at least three eligible mesh levels.

## Claim ceiling

A PASS proves only the restricted RF-to-support-reaction equivalence checks for the exact allowlisted deck plus successful parsing of one corresponding output section. It does not authenticate process execution, prove the FEM solution is numerically correct, establish physical validity, certify structural safety, or authorize construction. Physical qualification remains UNRESOLVED.
