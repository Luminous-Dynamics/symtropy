# Radiance subprocess runner v0.2

**Status:** process/parser contract only. No official executable provenance, actual Radiance run, optical accuracy, product compliance or energy-saving result is established.

## Purpose

`tools/lighting-benchmark/radiance-runner.sh` supervises a bounded `rtrace` process and emits artifacts for independent review. It is additive to the strict Rust parser and the non-rankable failure-receipt contract. It never emits an energy score; `scoring_eligible` remains false even when process and parsing succeed.

## Locked output contract

The legacy-RGB profile is exactly:

```text
rtrace <whitelisted numeric settings> -h- -I+ -ov -cs 3 -x 1 scene.oct
```

`-h-` explicitly disables header output; `-I+` explicitly enables irradiance for point/orientation inputs; `-ov` echoes input origin and RGB irradiance; `-cs 3` fixes three output channels; `-x 1` flushes each input record. This gives six output values per ray: origin x/y/z and irradiance R/G/B. `-opv` would emit origin, intersection point and value and therefore does not match the parser's six-field arity. See the [official rtrace manual](https://radsite.lbl.gov/radiance/man_html/rtrace.1.html) and [Radiance 6.0 release notes](https://www.radiance-online.org/download-install/radiance-source-code/latest-release/ReleaseNotes.txt).

## Stage, hash, invoke

1. The caller supplies the executable, strict parser, octree, case, design, property data and adapter configuration. The output directory must not exist.
2. The runner creates a private directory under `umask 077`, copies inputs and candidate tools, removes write permissions, and hashes the staged bytes. All subsequent execution uses the staged executable/parser and staged scene.
3. The expected executable/parser SHA-256 values must be 64 lowercase hex characters and match the staged bytes. The runner rechecks staged tool digests immediately before and after execution.
4. The adapter configuration must declare `schema_version: 0.1.0`, `profile: legacy-rgb-v1`, and unique whitelisted numeric option/value pairs. Runner-owned output flags and mode controls cannot be overridden. The exact flattened argv and minimal child environment are preserved.
5. GNU `timeout` supervises the process; native stdout/stderr and supervisor timeout diagnostics are captured separately. Exit status, elapsed duration and timeout status are distinct.
6. Strict parser success is all-or-nothing. The normalized ordered sample grid and parser diagnostics receive separate hashes. Native streams are always preserved if invocation occurs.

## Report and failure semantics

`run-report.json` includes case/tool labels, source and staged paths, expected and actual staged tool hashes, scene/input digests, command/argv evidence, raw stream/normalized sample/diagnostics digests and process outcome. Missing or invalid executable, pin mismatch, invalid configuration, timeout, nonzero exit and parser rejection are non-rankable. The report's service-energy metric is always null.

The caller-provided engine version label is descriptive; the executable digest only proves identity to the caller-supplied pin. It does not establish that the bytes came from an official build. Never use a freshly computed digest of an unknown binary as proof that it is an official Radiance release.

## Verification

The mock harness includes successful parsing, nonzero exit, supervisor timeout, child exit 124, executable digest mismatch, missing executable, invalid/reserved settings, duplicate options, parser digest mismatch and a permanent non-rankable claim ceiling. In CI, it runs once with a mock parser and once with the actual Rust parser CLI, using a mock solver for both.

These tests validate shell/process/report contracts only. They do not validate actual native Radiance output, scene compilation, the RGB-to-lux formula against a physical source, photometric calibration, product quality or any energy-saving outcome.

## Blockers before actual solver evidence

- Retrieve the selected official source archive bytes and compute the archive hash from those actual bytes; record source URL, archive size, license, build image/toolchain, dependencies and executable hash.
- Build the scene/octree and luminaire model from reviewed fixtures and pinned source data.
- Choose solver settings with the separate convergence sweep, freeze one settings policy across all candidate algorithms and charge all high-fidelity calls to a matched budget.
- Run analytical known-answer cases where the assumptions permit them, inspect native output and independently verify the parser.
- Compare against calibrated illuminance and power measurements with a declared uncertainty budget. Keep optical correctness, service-design performance and product/laboratory claims as separate evidence tiers.

No setting in mock fixtures is a recommended production value. No SOTA or real-world energy-saving claim is made by this runner.
