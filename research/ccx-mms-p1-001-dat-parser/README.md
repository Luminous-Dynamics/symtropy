# Strict CalculiX 2.22 .dat parser

This crate parses the output of NODE PRINT for the PHYS-CCX-MMS-P1-001 workflow.

**Important semantic distinction:** the forces (fx,fy,fz) section requested by RF contains the sum of external nodal forces: applied loads plus reactions where applicable. It is not universally reaction-only. The pinned CalculiX 2.22 manual documents this explicitly in section 6.11.5, “Forces obtained by selecting RF” (manual page 394); it states that RF reduces to the reaction force only when the node has no concentrated load and no attached element has distributed loading. In the frozen MMS deck, boundary nodes have no CLOAD and there are no DLOADs, so boundary RF can serve as the reduced-load support reaction. The correction for omitted boundary-equivalent loads is still gated on a real pinned-solver run.

References:
- CalculiX 2.22 User's Manual, section 6.11.5, pages 394–395: https://www.dhondt.de/ccx_2.22.pdf
- CalculiX 2.18 NODE PRINT manual: https://www.feacluster.com/CalculiX/ccx_2.18/doc/ccx/node307.html
- Documented RF/RR output example on a later forum thread: https://calculix.discourse.group/t/output-reaction-forces-only/3809

The parser requires exactly one target-time displacement block and one target-time external-force block for the expected set, checks every node ID against the generated inventory, rejects duplicate/missing/unexpected nodes, malformed rows, non-finite fields, invalid times, and ambiguous matching blocks. It accepts E and D exponent numeric notation.

The included .dat fixture is SYNTHETIC, not output from a real CalculiX process. It tests parser semantics only. Pinned-CCX integration remains a qualification gate; no solver execution or numerical result is claimed by this crate.


## Global force-balance audit

`audit_global_force_balance` combines the corrected boundary support reactions with the
**complete** nodal load vector produced independently by the mesh/load assembler. It first
requires the load map to contain exactly the node inventory present in the parsed output,
rejects non-finite loads and accumulation overflow, and then reports:

- residual vector: `sum(R_full on boundary) + sum(F_i over all nodes)`;
- scale: `max(1, sum(||R_full||_1) + sum(||F_i||_1))`;
- normalized residual: `||residual||_infinity / scale`;
- an explicit `passed` boolean against the caller-provided finite nonnegative tolerance.

A finite over-tolerance residual is reported as `passed = false`; the caller must treat
that value as a failed qualification gate. Missing/extra load nodes and non-finite values
are errors. Unit tests include a balanced synthetic example and a deliberately perturbed
load that must fail the numeric gate. These tests verify arithmetic and fail-closed handling
only. They do not qualify CalculiX's output semantics, the actual assembler, or a real solver
run.
