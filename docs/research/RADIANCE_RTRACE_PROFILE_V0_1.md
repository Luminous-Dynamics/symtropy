# Radiance `rtrace` workplane output profile v0.1

Status: **design contract, not an executed Radiance integration**. The companion Rust module `tools/lighting-benchmark/src/radiance.rs` implements the strict input/output parsing rules and pure conversion tests. It does not launch Radiance, build an octree, interpret IES files, calibrate a source model, or validate physical accuracy.

## Release identity and reference lane

For the reproducible v0.1 reference lane, prefer the official Radiance 6.0 P1 source archive rather than a moving HEAD/nightly build:

- Official source release page: https://www.radiance-online.org/download-install/radiance-source-code/latest-release
- Archive: `rad6R0P1all.tar.gz`, listed as 245.18 MB and last modified 2025-08-22: https://www.radiance-online.org/download-install/radiance-source-code/latest-release/rad6R0P1all.tar.gz/view
- Official release notes identify version 6.0 as released 2025-07-22: https://www.radiance-online.org/download-install/radiance-source-code/latest-release/ReleaseNotes.txt
- The LBNL-ETA GitHub mirror lists 6.1a builds as pre-releases, including a 2026-05-05 build: https://github.com/LBNL-ETA/Radiance/releases

The archive's SHA-256 has **not** been established by this document. Before the adapter can claim a pinned toolchain, download the actual bytes from the official source, calculate and independently check the SHA-256, record the URL/size/hash, pin the build environment and dependency identities, and record the built executable digest. The official `latest-release` URL is a locator, not a content pin. If 6.1a is later tested, give it a different explicit lane; never silently replace the reference backend.

Radiance 6.0 added spectral rendering facilities while retaining legacy RGB rendering as the default for compatibility (with documented exceptions and format changes). The reference profile below explicitly requests three-channel RGB mode; a spectral lane needs its own output arity, spectral-to-photopic conversion and calibration path. Do not label these legacy RGB values as measured-spectrum results.

## Input contract

For each expected grid point, send exactly one ASCII row to `rtrace -I`:

```text
x_m y_m z_m normal_x normal_y normal_z
```

For this fixed horizontal task plane, the normal is `0 0 1`. Rows are emitted in frozen case order (x changes fastest, then y). No point may be silently omitted, duplicated or reordered. Coordinates use metres and must be derived from the versioned case fixture, not from optimizer-provided point counts.

The parser helper `render_rtrace_irradiance_input` writes each coordinate deterministically at f64 round-trip precision. It rejects an empty grid, non-finite coordinates and non-sequential indices.

## Output contract

Use a command profile equivalent to:

```text
rtrace -h -I -opv -cs 3 <fixed solver settings> scene.oct
```

Command arguments must be passed as an argv array, not concatenated into a shell command. The exact solver settings, octree and every source input must be separately hashed. `-h` suppresses the informational header, `-I` requests irradiance at point/orientation inputs, and `-opv` requests point coordinates plus the RGB value. The parser expects **exactly six finite numeric fields per line**:

```text
reported_x reported_y reported_z irradiance_R irradiance_G irradiance_B
```

This six-field output profile is grounded in the Radiance `rtrace` manual and its workplane-illuminance examples:

- `rtrace` manual: https://radsite.lbl.gov/radiance/man_html/rtrace.1.html
- Workplane illuminance examples: https://radsite.lbl.gov/radiance/refer/analysis.pdf

The decoder rejects header text, blank records, wrong field counts, non-numeric/non-finite values, negative irradiance channels, coordinate drift, reordered points, and both short and long output. It is all-or-nothing: no partial prefix is returned as a usable grid. Reported coordinates are retained in each decoded sample alongside expected coordinates so the verifier can inspect them independently.

## Legacy RGB-to-lux conversion

The v0.1 RGB profile names this conversion exactly:

```text
lux = 47.4 * R + 120.0 * G + 11.6 * B
conversion_id = radiance_rgb_legacy_47_4_120_11_6_v1
```

The coefficients follow the RGB-to-illuminance conversion shown in the Radiance `rtrace` manual: https://radsite.lbl.gov/radiance/man_html/rtrace.1.html . They are a software convention for this legacy RGB profile, not a certificate of measured photometric accuracy. Changing color primaries, running in spectral mode, changing the conversion formula or introducing calibrated spectral measurements requires a separately versioned conversion profile and new known-answer tests.

Known-answer unit tests check the arithmetic (R=G=B=1 gives 179 lx under this formula; each isolated channel gives its fixed named coefficient). These validate only the deterministic conversion function, not the solver, the spectral power distribution of a real light source, or agreement with a calibrated lux meter.

## Fail-closed acceptance

The full adapter should only produce a candidate observation if all of these hold:

1. The selected executable is the pinned build; source archive, build environment, executable, scene, material/property files and settings digests match the run receipt.
2. The process exits successfully without timeout. Preserve stdout, stderr, duration, exact argv and native byte hashes.
3. The output uses this explicit legacy RGB profile; a spectral or differently formatted result is rejected rather than inferred.
4. Every expected sample produces one and only one record, with six finite values and coordinates mapping to the same grid point within the declared tolerance.
5. Illuminance is derived using the recorded conversion ID, and the independent evaluator recomputes aggregate metrics from all normalized samples.
6. Any failure produces a v0.1 non-rankable failure receipt, not a candidate score.

## Evidence boundary and next step

Current status is a parser contract plus authored unit tests. No Radiance process has run through this module, no archive digest has been pinned, and no optical accuracy is established. Next implement the supervised subprocess wrapper (timeout/kill, exact argv, captured raw streams), build the frozen scene with pinned `oconv`, exercise parser negative controls against actual native output, and then compare the parsed grid with an independently measured task-light baseline. Keep process correctness, numerical photometric accuracy and physical product performance as separate evidence tiers.
