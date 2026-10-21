# PHYS-CCX-EXEC-001 — Real CalculiX axial-bar execution v0.1

**Status:** draft execution/verification harness. No current exact-head workflow pass is asserted by this document.  
**Qualification status:** UNRESOLVED until the pinned build, real solver outputs, exact-byte checks, analytical comparisons, negative controls, and report artifact all pass on the same PR head.

## Objective

Move one narrow structural capability from synthetic receipt/parser tests to a provenance-locked **real CCX execution**, independently compared with a closed-form result. This is deliberately a narrow qualification slice, not a general FEM capability claim.

The frozen model is a straight, uniform, one-dimensional axial bar under static tension:

- Length: (L=1\,\mathrm{m})
- Cross-sectional area: (A=0.01\,\mathrm{m^2})
- Young's modulus: (E=210\,\mathrm{GPa})
- Point load: (F=1000\,\mathrm{N}) along global (+x)
- Fixed support at the first node; lateral motion constrained on remaining nodes
- Linear-elastic, small-displacement, static analysis
- T3D2 bar elements with (N=1,2,4) uniform elements

The independent oracle is

\[
u_{\mathrm{tip}}=\frac{FL}{EA}
=\frac{1000\cdot1}{210\times10^9\cdot0.01}
\approx 4.7619047619\times10^{-7}\,\mathrm{m}.
\]

The support reaction in global (x) should be approximately (-1000\,\mathrm{N}), so the force-balance residual is (R_x+F_x).

## Provenance and build identity

The focused workflow is designed to:

1. Use a digest-pinned Ubuntu container and a pinned apt snapshot.
2. Check out the build recipe at **OmnibusCloud/CalculiX** commit **b7977a4e90b40fd28fa149fc69f1647661d10f56**.
3. Download the official CCX 2.22 source archive, verify SHA-256 **3a94dcc775a31f570229734b341d6b06301ebdc759863df901c8b9bf1854c0bc**, and compare the builder's imported source tree byte-for-byte against the verified archive's source tree.
4. Build with the pinned SPOOLES dependency and explicit feature profile: **WITH_PARDISO=0**, **WITH_PASTIX=0**, **WITH_ARPACK=0**, **WITH_MT=1**.
5. Record compiler/toolchain and installed-package inventory, build-log digest, executable SHA-256, builder commit, source-archive digest and container identity in a machine-readable build manifest.
6. Execute CCX against checked-in request-bound input decks and preserve raw .dat/.frd outputs, process logs and per-run receipts.

The build manifest is an input to the displacement comparator. The parser checks it against the pinned build profile and binds the report to exact request/deck/output/build-manifest bytes. It does **not** authenticate process execution by itself. The workflow separately records executable hash, command invocation, exit status, deck hash, output hashes, and thread policy. Claims remain below process attestation or physical validation.

## Evidence chain

### Displacement

The Rust CLI option **--displacement-manifest** consumes the exact request, deck, .dat output, build manifest and a manifest containing SHA-256 digests for those inputs. It accepts only a narrow bar-deck grammar:
- one material and one elastic definition;
- a T3D2 chain on global x, with strictly increasing node coordinates;
- fixed support translations and restrained lateral degrees of freedom on each other node;
- one global-x point load at the declared tip;
- one node-print displacement section for the exact tip node and expected step time;
- finite, unique, correctly identified node rows;
- no unsupported deck cards or undeclared nodes.

The measured (u_x) is compared with (FL/(EA)) using the preregistered absolute-plus-relative tolerance. Unsupported or stale data are rejected; a parseable numeric mismatch is reported as FAIL. Even a passing reference comparison remains **physical_result_qualified=false**.

### Reaction force

A separate deck emits reaction forces only, allowing the existing restricted-RF parser to inspect one unambiguous support-RF section. A request/deck node map binds the support and load endpoints. The expected global-x reaction is (-1000\,\mathrm{N}). The workflow checks the normalized reaction and force-balance residual; it does not substitute an exit code or plausible-looking number for exact deck/output binding.

### Mesh trend

The (N=1,2,4) cases represent the same analytic straight bar. The workflow records tip displacement at each level, relative error against the closed-form oracle, level-to-level spread, element count and executable/build identity. The preregistered level-to-level displacement spread limit is (10^{-12}\,\mathrm{m}).

This is a **small analytical mesh-trend check**, not a general adaptive mesh refinement study and not a calculated Grid Convergence Index. Do not infer broad mesh-convergence or continuum-field accuracy from this case.

### Negative controls and failure evidence

- Mutate output bytes without updating the manifest digest: exact-byte verification must reject it.
- Malformed, duplicated, non-finite or wrong-node displacement rows: must not pass.
- Unsupported deck cards and mismatched build identities: reject.
- Incorrect but syntactically valid displacement: return a numeric FAIL.
- Non-zero or timed-out solver process: preserve status and logs and fail the exact-head gate.
- Missing output: preserve the failure and do not map missing values to zero.

Artifacts are collected with an always-run step, including partial build/run logs when an earlier step fails. A missing artifact or skipped execution step never counts as a pass.

## Gates and benchmark outputs

- Rust formatter, unit tests and strict Clippy on the isolated physical-solver contract crate.
- Official archive checksum and source-tree equality check.
- Pinned build recipe and explicit no-proprietary/no-optional-solver profile.
- Time-bounded real solver invocation for three displacement meshes, one RF case, and a repeated one-element displacement case.
- Exact-byte displacement report for each mesh level.
- Separate restricted RF normalization and force-balance check.
- Repeated-run numerical agreement, three-level analytical mesh trend and tamper negative control.
- Raw solver outputs, build manifest and hash, per-process receipts, numerical reports, summary report, and exact PR source commit.

Target: **zero false PASS** for malformed, stale, unsupported, timed-out, non-finite or over-tolerance cases. The summary must retain **physical_result_qualified=false**, **real_world_validation_status=UNRESOLVED**, and **engineering_use_authorized=false**.

## Claim boundary

A pass can support the statement that a pinned CCX build executed this specific small-strain linear-static axial-bar fixture and that its displacement and support reaction agreed with independent closed-form expectations within declared tolerances, for the exact request/deck/output/build-manifest bytes.

It does not validate measured physical behaviour, prove all CCX capabilities, qualify shells/contact/plasticity/buckling/nonlinear or dynamic mechanics, authorize construction or demolition, or establish that Symtropy is faster or more accurate than other FEM software.

## Upstream anchors

- [Symtropy external FEM qualification issue #2321](https://github.com/Luminous-Dynamics/symtropy/issues/2321)
- [NAFEMS standard benchmarks](https://www.nafems.org/publications/resource_center/p18/)
- [Pinned CalculiX build-recipe provenance](https://github.com/OmnibusCloud/CalculiX/blob/b7977a4e90b40fd28fa149fc69f1647661d10f56/PROVENANCE.md)
- [Ubuntu snapshot-service documentation](https://ubuntu.com/server/docs/how-to/software/snapshot-service/)
