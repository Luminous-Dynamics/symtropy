#!/usr/bin/env bash
set -euo pipefail

runner="${1:?runner binary path required}"
replay="${2:?replay binary path required}"
importer="${3:?SU2 history importer binary path required}"
convergence="${4:?SU2 convergence screen binary path required}"
root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT

cat > "$root/fake-su2-cfd" <<'FAKE'
#!/bin/sh
set -eu
[ "$#" -eq 2 ]
[ "$1" = "-f" ]
[ "$2" = "config.cfg" ]
[ -s config.cfg ]
[ -s mesh.su2 ]
printf 'Iteration,CL,CD,RMS_DENSITY,RMS_MOMENTUM-X\n0,0.4100,0.0210,-1,-1\n10,0.4200,0.0200,-2,-2\n20,0.4230,0.01820,-6.5,-6.5\n30,0.42305,0.01815,-6.8,-6.8\n40,0.42310,0.01810,-7.1,-7.1\n50,0.42310,0.01808,-7.4,-7.4\n60,0.42315,0.01806,-7.7,-7.7\n70,0.42316,0.01805,-8,-8\n' > history.csv
printf 'fixture force history\n' > forces_breakdown.dat
printf 'synthetic SU2-style fixture stdout\n'
printf 'synthetic SU2-style fixture stderr\n' >&2
FAKE
chmod +x "$root/fake-su2-cfd"

printf 'MESH_FILENAME=mesh.su2\nMACH_NUMBER=0.73\n' > "$root/config.cfg"
printf 'synthetic mesh fixture; not a physical mesh\n' > "$root/mesh.su2"
executable_sha256="$(sha256sum "$root/fake-su2-cfd" | awk '{print $1}')"

"$runner" \
  --run-id "ci-su2-process-fixture" \
  --solver-id "SU2_CFD" \
  --version-claim "synthetic-ci-fixture-not-a-SU2-version" \
  --executable "$root/fake-su2-cfd" \
  --expected-executable-sha256 "$executable_sha256" \
  --run-dir "$root/run" \
  --input "$root/config.cfg=config.cfg" \
  --input "$root/mesh.su2=mesh.su2" \
  --output "history.csv" \
  --output "forces_breakdown.dat" \
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
  --field RMS_DENSITY=log10(residual) \
  --field RMS_MOMENTUM-X=log10(residual) \
  --minimum-rows 8 \
  > "$root/su2-history-import.csv" 2> "$root/su2-history-import.txt"
test "$(wc -l < "$root/su2-history-import.csv")" -eq 9
grep -q 'su2_history_import=pass run_id=ci-su2-process-fixture points=8 selected_fields=4' "$root/su2-history-import.txt"
grep -q 'receipt_bound_history_import_only_not_solver_convergence_or_physical_validation' "$root/su2-history-import.txt"
python3 scripts/verify-su2-history-import-independent.py "$root/run" "$root/su2-history-import.csv"

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
  --field RMS_DENSITY=log10(residual) \
  --field RMS_MOMENTUM-X=log10(residual) \
  --residual-log10 'RMS_DENSITY|log10(residual)|6|-8|6|2' \
  --residual-log10 'RMS_MOMENTUM-X|log10(residual)|6|-8|6|2' \
  --monitor 'CL|dimensionless|6|0.0004|0.0003|0.02' \
  --monitor 'CD|dimensionless|6|0.0004|0.0003|0.02' \
  > "$root/su2-convergence-screen.csv" 2> "$root/su2-convergence-screen.txt"
test "$(wc -l < "$root/su2-convergence-screen.csv")" -eq 5
grep -q 'su2_convergence_screen=pass run_id=ci-su2-process-fixture passed=4 failed=0 insufficient=0' "$root/su2-convergence-screen.txt"
grep -q 'declared_residual_encoding_terminal_stability_and_monitor_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation' "$root/su2-convergence-screen.txt"
python3 scripts/verify-su2-convergence-screen-independent.py "$root/run" "$root/su2-history-import.csv" "$root/su2-convergence-screen.csv"
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
echo "external_solver_cli_known_answer=pass staged_inputs=2 expected_outputs=2 tamper_rejected=true"
