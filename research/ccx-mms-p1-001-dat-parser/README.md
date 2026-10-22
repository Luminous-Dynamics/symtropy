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

## Real CalculiX MMS execution path

The `mms-p1-real-verify` binary is the end-to-end numerical postprocessor used by
`.github/workflows/ccx-mms-p1-real-run.yml`. For each generated mesh it:

1. regenerates the declared CPE3 deck and requires byte-for-byte deck equality;
2. strictly parses genuine DAT displacement and RF output for the complete `NALL` node inventory at the frozen static time;
3. evaluates displacement (L^2), plane-strain energy, and stress errors from the solver-produced displacement field against the independent continuum oracle;
4. independently reassembles all nodal body-force loads, verifies their resultant against the analytical integral, applies the frozen omitted-boundary-load reaction correction, and audits global force balance;
5. optionally compares a repeat-run displacement field against its reference field at the same node inventory.

The workflow builds stock CalculiX 2.22 from the SHA-pinned official source archive using the pinned OmnibusCloud builder commit and snapshot-pinned container/packages. It executes mesh resolutions (n=8,16,32,64), repeats (n=16), keeps stdout/stderr and raw DAT/FRD files, hashes the source/build/deck/output/report provenance, and uploads the complete evidence directory even when a gate fails. Only after every mesh has run does the strict gate evaluate resultants, corrected global equilibrium, boundary displacements, strict error decrease, observed displacement/energy orders, and repeat stability.

The parser-side JSON report always leaves `qualification_pass=false`; it does not attest to the process that created the DAT file. The workflow stores a separate process receipt that binds the exact executable, deck, DAT/FRD, logs, build manifest, and report digests. If the real DAT header, output inventory, force interpretation, or numerical trend disagrees with the protocol, the gate fails and retains evidence rather than substituting a synthetic fixture or relaxing thresholds.

A passing run may qualify only this pinned linear plane-strain manufactured-solution case. It does not establish physical validation, engineering-use authorization, broad CalculiX capability, or superior performance to an industry solver. Performance work must remain a separate matched-accuracy experiment.


## Executable qualification and follow-on diagnostics

The draft real-run workflow is `.github/workflows/ccx-mms-p1-real-run.yml`. It reuses the
source/build identity from the provenance-locked CCX 2.22 axial-bar workflow and adds:

- Real CPE3 MMS executions for (n=8,16,32,64), plus a repeated (n=16) solve.
- Independent load-resultant and corrected-reaction equilibrium checks, displacement
  (L^2), plane-strain energy, stress errors and observed-order gates.
- A smooth divergence-free manufactured field with exact (
abla\cdot u=0), verified
  independently through stress-divergence finite differences and a lambda-independent
  body-force expression. Its global runs at (
u=0.3,0.49,0.4999,0.49999), resolutions
  (n=16,32), report finite-element volumetric strain and displacement/energy/stress errors.
- Seven repeated CCX process timings per baseline mesh resolution, with warm-up excluded,
  solver-only wall time, peak RSS, raw samples, median and interquartile range. Each timed
  displacement field must also match the baseline result at the frozen repeat tolerance.
- Pinned source archive, builder commit, container/package snapshot, executable digest,
  toolchain/hardware inventory, per-deck/output/log/report hashes, and raw failure-artifact
  upload.

The real-run postprocessor never sets its own `qualification_pass` to true. A separate
workflow gate checks the complete run family. Even if that gate passes, it qualifies only
the declared pinned CCX 2.22 manufactured linear plane-strain case; it does not establish
physical validation or engineering-use authorization.

The incompressibility sweep is intentionally labelled a **diagnostic**, not a locking
qualification: it lacks the planned mixed-formulation and/or authoritative reference
comparison, so the JSON explicitly records
`volumetric_locking_measured=false`. The work–precision report is also intentionally
not a cross-solver or industry-superiority comparison: it measures repeatability, resource
use and numerical error for CCX 2.22 alone. Those boundaries follow recognized code
verification practice, where exact-solution mesh convergence and independently sourced
reference cases are distinct evidence levels.
