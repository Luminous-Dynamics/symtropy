#!/usr/bin/env bash
set -euo pipefail

runner="${1:?runner binary path required}"
replay="${2:?replay binary path required}"
importer="${3:?SU2 history importer binary path required}"
convergence="${4:?SU2 convergence screen binary path required}"
mesh_audit="${5:?SU2 mesh audit binary path required}"
preflight="${6:?SU2 field preflight audit binary path required}"
boundary="${7:?SU2 boundary marker audit binary path required}"
root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT

cat > "$root/fake-su2-cfd" <<'FAKE'
#!/bin/sh
set -eu
[ "$#" -eq 2 ]
[ "$1" = "-f" ]
[ "$2" = "config.cfg" ]
[ -s config.cfg ]
[ -s geometry.dat ]
[ -s mesh.su2 ]
printf 'Iteration,CL,CD,RMS_DENSITY,RMS_MOMENTUM-X\n0,0.4100,0.0210,-1,-1\n10,0.4200,0.0200,-2,-2\n20,0.4230,0.01820,-6.5,-6.5\n30,0.42305,0.01815,-6.8,-6.8\n40,0.42310,0.01810,-7.1,-7.1\n50,0.42310,0.01808,-7.4,-7.4\n60,0.42315,0.01806,-7.7,-7.7\n70,0.42316,0.01805,-8,-8\n' > history.csv
printf 'fixture force history\n' > forces_breakdown.dat
printf 'fixture surface field\n' > surface_flow.csv
printf 'synthetic SU2-style fixture stdout\n'
printf 'synthetic SU2-style fixture stderr\n' >&2
FAKE
chmod +x "$root/fake-su2-cfd"

printf 'MESH_FILENAME=mesh.su2\nMACH_NUMBER=0.73\n' > "$root/config.cfg"
printf 'synthetic geometry fixture; not an aircraft geometry\n' > "$root/geometry.dat"
cat > "$root/mesh.su2" <<'MESH'
NDIME= 2
NELEM= 2
5 0 1 2
5 0 2 3
NPOIN= 4
0.0 0.0
1.0 0.0
1.0 1.0
0.0 1.0
NMARK= 1
MARKER_TAG= wall
MARKER_ELEMS= 4
3 0 1
3 1 2
3 2 3
3 3 0
MESH
executable_sha256="$(sha256sum "$root/fake-su2-cfd" | awk '{print $1}')"

"$runner" \
  --run-id "ci-su2-process-fixture" \
  --solver-id "SU2_CFD" \
  --version-claim "synthetic-ci-fixture-not-a-SU2-version" \
  --executable "$root/fake-su2-cfd" \
  --expected-executable-sha256 "$executable_sha256" \
  --run-dir "$root/run" \
  --input "$root/config.cfg=config.cfg" \
  --input "$root/geometry.dat=geometry.dat" \
  --input "$root/mesh.su2=mesh.su2" \
  --output "history.csv" \
  --output "forces_breakdown.dat" \
  --output "surface_flow.csv" \
  --timeout-seconds 10 \
  --max-log-bytes 16384 \
  -- -f config.cfg

"$replay" "$root/run" | tee "$root/replay.txt"
grep -q 'external_solver_replay=pass' "$root/replay.txt"
grep -q 'process_status=process_exited_zero_expected_outputs_present' "$root/replay.txt"
grep -q 'evidence_level=process_receipt_only_caller_version_claim_unverified_no_convergence_or_physical_validation' "$root/replay.txt"

"$importer" \
  --run-dir "$root/run" \
  --history history.csv \
  --iteration-column Iteration \
  --header-column Iteration \
  --header-column CL \
  --header-column CD \
  --header-column RMS_DENSITY \
  --header-column RMS_MOMENTUM-X \
  --field CL=dimensionless \
  --field CD=dimensionless \
  --field 'RMS_DENSITY=log10(residual)' \
  --field 'RMS_MOMENTUM-X=log10(residual)' \
  --minimum-rows 8 \
  > "$root/su2-history-import.csv" 2> "$root/su2-history-import.txt"
test "$(wc -l < "$root/su2-history-import.csv")" -eq 9
grep -q 'su2_history_import=pass run_id=ci-su2-process-fixture points=8 selected_fields=4' "$root/su2-history-import.txt"
grep -q 'receipt_bound_history_import_only_not_solver_convergence_or_physical_validation' "$root/su2-history-import.txt"
python3 scripts/verify-su2-history-import-independent.py "$root/run" "$root/su2-history-import.csv"

"$mesh_audit" "$root/run/mesh.su2" > "$root/su2-mesh-summary.csv" 2> "$root/su2-mesh-summary.txt"
test "$(wc -l < "$root/su2-mesh-summary.csv")" -eq 2
grep -q 'su2_mesh_audit=pass dimension=2 points=4 volume_elements=2 markers=1 boundary_elements=4' "$root/su2-mesh-summary.txt"
grep -q 'su2-native-mesh-summary/0.1.0' "$root/su2-mesh-summary.csv"
python3 scripts/verify-su2-mesh-independent.py "$root/run/mesh.su2" "$root/su2-mesh-summary.csv"

"$convergence" \
  --run-dir "$root/run" \
  --history history.csv \
  --iteration-column Iteration \
  --header-column Iteration \
  --header-column CL \
  --header-column CD \
  --header-column RMS_DENSITY \
  --header-column RMS_MOMENTUM-X \
  --field CL=dimensionless \
  --field CD=dimensionless \
  --field 'RMS_DENSITY=log10(residual)' \
  --field 'RMS_MOMENTUM-X=log10(residual)' \
  --residual-log10 'RMS_DENSITY|log10(residual)|6|-8|6|2' \
  --residual-log10 'RMS_MOMENTUM-X|log10(residual)|6|-8|6|2' \
  --monitor 'CL|dimensionless|6|0.0004|0.0003|0.02' \
  --monitor 'CD|dimensionless|6|0.0004|0.0003|0.02' \
  --config-input config.cfg \
  --geometry-input geometry.dat \
  --mesh-input mesh.su2 \
  --additional-output forces_breakdown.dat \
  --additional-output surface_flow.csv \
  --lineage-output "$root/su2-run-lineage.csv" \
  > "$root/su2-convergence-screen.csv" 2> "$root/su2-convergence-screen.txt"
test "$(wc -l < "$root/su2-convergence-screen.csv")" -eq 5
grep -q 'su2_convergence_screen=pass run_id=ci-su2-process-fixture passed=4 failed=0 insufficient=0' "$root/su2-convergence-screen.txt"
grep -q 'declared_residual_encoding_terminal_stability_and_monitor_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation' "$root/su2-convergence-screen.txt"
python3 scripts/verify-su2-convergence-screen-independent.py "$root/run" "$root/su2-history-import.csv" "$root/su2-convergence-screen.csv"
test "$(wc -l < "$root/su2-run-lineage.csv")" -eq 7
grep -q 'su2_run_lineage=pass run_id=ci-su2-process-fixture outputs=2' "$root/su2-convergence-screen.txt"
grep -q 'su2-run-lineage-binding/0.1.0' "$root/su2-run-lineage.csv"
python3 scripts/verify-su2-run-lineage-independent.py "$root/run" "$root/su2-history-import.csv" "$root/su2-convergence-screen.csv" "$root/su2-run-lineage.csv" "$root/su2-mesh-summary.csv"
test -s "$root/run/invocation_manifest.txt"
test -s "$root/run/environment_manifest.txt"
test -s "$root/run/runner_receipt.csv"
test -s "$root/run/expected_outputs.csv"

cp -a "$root/run" "$root/tampered-run"
printf 'tampered\n' >> "$root/tampered-run/stdout.log"
if "$replay" "$root/tampered-run" > "$root/tampered-replay.txt" 2>&1; then
  echo "tampered runner receipt unexpectedly replayed" >&2
  exit 1
fi
grep -q 'persisted stdout.log differs from receipt' "$root/tampered-replay.txt"

# A process-only receipt is permitted solely for this explicit SU2_CFD -d preflight path.
cat > "$root/fake-su2-dry-run" <<'FAKE_DRY'
#!/bin/sh
set -eu
[ "$#" -eq 2 ]
[ "$1" = "-d" ]
[ "$2" = "preflight.cfg" ]
[ -s preflight.cfg ]
[ -s mesh.su2 ]
printf '%s\n' 'Available screen/history output fields for the current configuration:'
printf '%s\n' '| Field Name | Description | Group Name |'
printf '%s\n' '| ITER | Iteration count | ITER |'
printf '%s\n' '| INNER_ITER | Inner iteration | ITER |'
printf '%s\n' '| RMS_DENSITY | Density residual | RMS_RES |'
printf '%s\n' '| RMS_MOMENTUM-X | X momentum residual | RMS_RES |'
printf '%s\n' '| DRAG | Drag coefficient | AERO_COEFF |'
printf '%s\n' '| CD | Drag coefficient | AERO_COEFF |'
printf '%s\n' 'Available volume output fields for the current configuration:'
printf '%s\n' '| PRESSURE | Pressure | PRIMITIVE |'
FAKE_DRY
chmod +x "$root/fake-su2-dry-run"
cat > "$root/preflight.cfg" <<'CFG'
SOLVER= NAVIER_STOKES
CONV_FIELD= RMS_DENSITY
HISTORY_OUTPUT= (ITER, RMS_RES, AERO_COEFF)
TIME_DOMAIN= NO\nWINDOW_CAUCHY_CRIT= NO
MARKER_HEATFLUX= (wall, 0.0)
MARKER_MONITORING= (wall)
MESH_FILENAME= mesh.su2
CFG
preflight_executable_sha256="$(sha256sum "$root/fake-su2-dry-run" | awk '{print $1}')"

"$runner" \
  --run-id "ci-su2-dry-run-preflight" \
  --solver-id "SU2_CFD_DRY_RUN" \
  --version-claim "synthetic-ci-dry-run-not-a-SU2-version" \
  --executable "$root/fake-su2-dry-run" \
  --expected-executable-sha256 "$preflight_executable_sha256" \
  --run-dir "$root/preflight-run" \
  --input "$root/preflight.cfg=preflight.cfg" \
  --input "$root/mesh.su2=mesh.su2" \
  --timeout-seconds 10 \
  --max-log-bytes 16384 \
  -- -d preflight.cfg

"$replay" "$root/preflight-run" | tee "$root/preflight-replay.txt"
grep -q 'external_solver_replay=pass' "$root/preflight-replay.txt"
grep -q 'process_status=process_exited_zero_no_outputs_declared' "$root/preflight-replay.txt"

"$preflight" \
  --run-dir "$root/preflight-run" \
  --config preflight.cfg \
  --mesh mesh.su2 \
  --require-field RMS_DENSITY \
  --require-field RMS_MOMENTUM-X \
  --require-field DRAG \
  --require-field CD \
  > "$root/su2-field-preflight.csv" 2> "$root/su2-field-preflight.txt"
test "$(wc -l < "$root/su2-field-preflight.csv")" -eq 2
grep -q 'su2_field_preflight=pass run_id=ci-su2-dry-run-preflight solver_family=NAVIER_STOKES conv_field=RMS_DENSITY steady_fields=RMS_DENSITY time_domain=NO window_cauchy=NO window_fields= available_fields=6 required_fields=4' "$root/su2-field-preflight.txt"
grep -q 'receipt_bound_su2_dry_run_multi_field_steady_and_explicit_time_window_configuration_discovery_only_not_history_convergence_or_validation' "$root/su2-field-preflight.txt"
python3 scripts/verify-su2-field-preflight-independent.py "$root/preflight-run" "$root/su2-field-preflight.csv"

"$boundary" \
  --run-dir "$root/preflight-run" \
  --config preflight.cfg \
  --mesh mesh.su2 \
  --boundary-option MARKER_HEATFLUX=2:0 \
  --non-boundary-option MARKER_MONITORING \
  > "$root/su2-boundary-audit.csv" 2> "$root/su2-boundary-audit.txt"
test "$(wc -l < "$root/su2-boundary-audit.csv")" -eq 2
grep -q 'su2_boundary_audit=pass run_id=ci-su2-dry-run-preflight mesh_markers=1 boundary_assignments=1 boundary_options=1 non_boundary_options=1' "$root/su2-boundary-audit.txt"
grep -q 'receipt_bound_exact_mesh_marker_to_declared_config_boundary_option_mapping_not_boundary_physics_or_flow_validation' "$root/su2-boundary-audit.txt"
python3 scripts/verify-su2-boundary-audit-independent.py "$root/preflight-run" "$root/su2-boundary-audit.csv"
echo "external_solver_cli_known_answer=pass staged_inputs=3 expected_outputs=3 tamper_rejected=true dry_run_process_only=true field_preflight=pass marker_mapping=pass"
