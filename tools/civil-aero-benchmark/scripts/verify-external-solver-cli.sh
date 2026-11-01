#!/usr/bin/env bash
set -euo pipefail

runner="${1:?runner binary path required}"
replay="${2:?replay binary path required}"
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
printf 'Iteration,CL,CD\n0,0.42,0.018\n' > history.csv
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
