# SU2 boundary-marker audit v0.1

## Purpose

A native SU2 mesh marker tag is only a named boundary entity. The solver configuration assigns its actual boundary condition through a corresponding configuration option. A byte-valid mesh and a byte-valid config therefore do not prove that the config references the same markers as the mesh.

The boundary audit runs against the same receipt-bound \`SU2_CFD -d CONFIG_PATH\` preflight used for field discovery. It verifies the staged config and native mesh bytes, extracts every mesh marker tag, and requires an exact mapping of each mesh marker to one declared boundary option. It rejects config references to absent mesh tags, duplicate assignments, incomplete mesh coverage, malformed tuple widths and unclassified \`MARKER_*\` options.

This is a structural consistency check. It does not prove the selected physical boundary condition is appropriate, that a periodic transform is correct, or that the resulting flow solution is accurate.

## Caller-declared tuple schema

The config option parser does not guess the cardinality of parameterized options. Supply a \`Su2BoundaryOptionSpec\` for each boundary condition in the case:

- \`option_name\`: exact SU2 config key, e.g. \`MARKER_SYM\`, \`MARKER_HEATFLUX\`, or \`MARKER_PERIODIC\`.
- \`record_width\`: number of comma-separated values per repeated boundary record.
- \`marker_positions\`: zero-based positions within each record that are mesh marker names.

Examples:

| Option | Record width | Marker positions | Interpretation |
|---|---:|---:|---|
| \`MARKER_EULER\` | 1 | 0 | one marker name per record |
| \`MARKER_SYM\` | 1 | 0 | one marker name per record |
| \`MARKER_FAR\` | 1 | 0 | one marker name per record |
| \`MARKER_HEATFLUX\` | 2 | 0 | marker plus heat flux value |
| \`MARKER_ISOTHERMAL\` | 2 | 0 | marker plus temperature |
| \`MARKER_HEATTRANSFER\` | 3 | 0 | marker plus coefficient and temperature |
| \`MARKER_PERIODIC\` | 11 | 0,1 | marker and donor marker plus nine transform values in a 3D tuple |

The caller must verify these layouts against the exact pinned SU2 version/solver documentation. The periodic example is a 3D tuple layout; do not apply it blindly to an alternate periodic syntax or solver mode. For options not represented in the caller schema, stop and add a versioned schema after checking the upstream parser.

Every other config marker option must be explicitly named in \`non_boundary_marker_options\`, e.g. \`MARKER_MONITORING\`, \`MARKER_PLOTTING\`, \`MARKER_DESIGNING\`, \`MARKER_ANALYZE\` or \`GEO_MARKER\`. Those options do not satisfy boundary coverage, but their marker references are also checked against the native mesh.

## CLI

After the successful dry-run receipt, invoke:

\`\`\`sh
cargo run --release --locked \\
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \\
  --bin su2_boundary_audit -- \\
  --run-dir ./artifacts/dpw8-v1a-periodic-l1-a136-dry-run \\
  --config config.cfg \\
  --mesh mesh.su2 \\
  --boundary-option MARKER_SYM=1:0 \\
  --boundary-option MARKER_FAR=1:0 \\
  --non-boundary-option MARKER_MONITORING \\
  --non-boundary-option MARKER_PLOTTING
\`\`\`

For this example, both mesh tags must appear exactly once across the two boundary assignment options, and any marker listed under monitoring or plotting must also exist in the mesh. The program fails closed on unknown marker options; classify every option rather than silently ignoring it.

CI includes a fake SU2 dry-run fixture with \`MARKER_HEATFLUX=(wall, 0.0)\` and \`MARKER_MONITORING=(wall)\`. An independent oracle reparses the mesh/config and recomputes the canonical digest. The fixture demonstrates parser behavior only; it is not a real SU2 run or boundary-physics validation.

## Evidence ceiling

The audit establishes that:
1. config and mesh bytes are bound to a replayable dry-run receipt;
2. every mesh marker is covered exactly once by the caller-declared boundary-option schema;
3. each configured BC marker exists in the mesh;
4. every other config marker option is explicitly classified and its marker list resolves to the mesh.

It does not establish that the specified option is physically appropriate for the condition, that marker orientation/normals are correct, that periodic transforms or inlet state values are right, that the solver uses every option as intended, or that the solution converges or validates against experiment.
