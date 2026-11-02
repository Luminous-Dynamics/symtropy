# SU2 lineage-bound grid-family study v0.1

## Purpose

This study API composes separate replayable production-run lineages into a coherent grid-refinement study. It deliberately accepts no caller-supplied grid coefficient values. Each scalar is derived from the terminal monitor window of the receipt-bound history import whose corresponding convergence screen passed.

A member must carry:
- replayable external solver receipt;
- config, fixed geometry, mesh and additional output bindings;
- imported SU2 history with exact output digest;
- declared residual and quantity-monitor convergence screen;
- canonical run-lineage binding;
- grid ID and ordered refinement level; characteristic spacing is derived from the staged mesh bytes.

The study replays every lineage, requires at least three unique run IDs and mesh digests, checks strictly coarse-to-fine spacing order, and requires identical solver executable digest, version claim, physics configuration digest/path and geometry digest across every grid. It then derives the quantity from the exact monitor terminal window and invokes the existing Richardson/GCI estimator.

## Mesh-derived characteristic spacing and remaining limitations

For the 2D linear triangle/quadrilateral lane, the study derives characteristic spacing from the exact mesh bytes rather than accepting a caller-entered (h):

`h = sqrt(A_total / N_cells) = sqrt(A_mean_cell)`

The parser calculates each linear triangle's area from its signed cross product and each linear quadrilateral's area from the shoelace formula, then records total/mean/min/max area, cell count, area coefficient of variation, coordinate-unit declaration, mesh byte length and SHA-256. The mesh digest and length must match the independently replayed run-lineage artifact before the derived spacing is admitted to GCI. Strict coarse-to-fine monotonicity is checked using the derived (h).

This definition is intentionally narrow. It rejects 3D meshes, high-order elements and unsupported cell types; it does not calculate anisotropy, skewness, orthogonality, boundary-layer resolution, (y^+), wake refinement or local error indicators. The coordinate unit is still a caller declaration because the SU2 mesh bytes do not encode a universal physical unit. It is therefore a mesh-derived global area spacing, not a complete mesh-quality or systematic-refinement proof. A 3D CFD lane needs a separately versioned volume-based definition and its own validation fixtures.

The protocol-manifest digest remains caller supplied and bound into the study receipt; it is not source-authenticated by this library.

## Outputs

\`report_to_csv\` emits one row per grid member, linking each run lineage and the full GCI-report digest. \`gci_report_to_csv\` emits every sliding coarse/medium/fine triplet from the GCI study. A six-grid family therefore preserves all four three-grid windows; it is not collapsed to the final triplet.

## Evidence ceiling

\`individually_replayed_su2_run_lineages_and_terminal_monitor_derived_gci_with_mesh_derived_2d_area_spacing_not_full_cfd_admission_or_experimental_validation\`

The study is numerical grid-convergence evidence only. It does not establish that the selected physical boundary conditions are correct, that conservation and all governing equations are sufficiently satisfied, that model-form uncertainty is bounded, or that the result agrees with experiment. Complete CFD admission and ASME V&V 20-style validation remain separate gates.

## Synthetic CI control

The Rust integration fixture creates three fake solver run receipts with:
- the same byte-identical fake executable, physics config and geometry artifact;
- three distinct, nested triangular meshes of the same unit-square geometry (1×1, 2×2, 4×4 cells);
- matched residual/force histories with an analytic second-order drag sequence;
- independent lineage replay and a deliberately reordered-grid negative control.

The known-answer test exercises study composition only. It is not CFD evidence.
