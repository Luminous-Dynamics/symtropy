# NASA CARA Reference Corpus — Source Inventory v0.1

This directory contains a **source manifest**, not copies of NASA CARA files.

Pinned upstream source:
- Repository: https://github.com/nasa/CARA_Analysis_Tools
- Commit: `43f0160f94d76a91805d86521c3e8fa0d09ecd68`
- Tree: `67cf62653228b6bebf2d17b34a57bf8249287697`
- Commit date: 2026-11-05

The manifest records candidate source paths, upstream Git blob SHA-1 object identifiers, sizes, and the reason each item is useful. These Git blob IDs are **not** SHA-256 digests of downloaded file bytes.

## Candidate comparison layers

| Layer | Pinned references |
|---|---|
| 2D Pc | CARA PcCircle / Foster-Estes |
| 2D and 3D collision count | Hall 2D-Nc and 3D-Nc |
| Direct simulation | CARA SDMC and Keplerian two-body Monte Carlo |
| Full ephemeris interval | EphemerisPc OEM example, Nc-rate output and PcMin/PcMax bounds |
| Covariance trust | CARA covariance-realism residual / scale-factor tools |
| Test corpus | Alfano, Frisbee and Omitron CDMs, including high-Pc, low-relative-velocity, non-PD covariance and 3D-Nc cases |

## Important comparison semantics

CARA's corpus README explains that CDM values may require state correction to the actual TCA because the published CDM TCA is rounded to milliseconds. Its multi-step entry point performs relevant state/TCA handling. The README also documents small numerical differences caused by MATLAB, BLAS/LAPACK, MKL, and operating-system versions.

CARA EphemerisPc evaluates the overlapping OEM interval and reports expected collision count / rate plus cumulative collision probability bounds. These outputs are not interchangeable with a single instantaneous TCA Pc.

The corpus is a reference implementation and comparison source, **not universal physical ground truth**. Use analytic synthetic cases and independent quadrature / high-budget estimators for known truth. Do not infer real-world calibration from agreement with another probability algorithm.

## Rights status

The upstream repository README lists several NASA Open Source Software Agreements. This inventory has not yet mapped individual algorithms, workbooks, sample CDMs and OEMs to their applicable agreements. The manifest therefore marks all candidates as rights-review-required and prohibits redistribution.

No source code, spreadsheet, CDM or OEM file is copied into this repository by this increment. Rights must be reviewed independently for:
- source-code execution;
- use of benchmark data;
- publication of derived numerical results;
- redistribution of original files.

## Next gates

1. Review the four pinned NOSA documents and identify the applicable agreement for each file family.
2. Confirm data provenance / restrictions for the sample CDMs and OEM example.
3. Retrieve only the first rights-cleared subset.
4. Compute local SHA-256 digests from exact downloaded bytes and create a frozen benchmark-case manifest.
5. Reproduce CARA reference outputs under a pinned platform and report tolerances before any performance comparison.

No SOTA or collision-risk qualification is claimed by this source inventory.
