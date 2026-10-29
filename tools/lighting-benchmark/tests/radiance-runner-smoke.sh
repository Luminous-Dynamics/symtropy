#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
RUNNER="$ROOT/radiance-runner.sh"
command -v jq >/dev/null
command -v sha256sum >/dev/null
TEST_ROOT=$(mktemp -d)
trap 'rm -rf "$TEST_ROOT"' EXIT
pass() { echo "radiance-runner smoke: PASS: $*"; }
fail() { echo "radiance-runner smoke: FAIL: $*" >&2; exit 1; }
make_case() {
  mkdir -p "$1"
  cat > "$1/case.json" <<'JSON'
{"case_id":"mock-room","task_surface":{"length_x_m":1.0,"width_y_m":0.0,"origin_xyz_m":[0,0,0],"measurement_grid":{"spacing_m":1.0,"expected_points":2}}}
JSON
  printf '{"design_id":"mock-design"}\n' > "$1/design.json"
  printf 'mock property data\n' > "$1/property.bin"
  printf '{"schema_version":"0.1.0","profile":"legacy-rgb-v1","rtrace_args":[["-ab","1"],["-ad","64"]]}\n' > "$1/config.json"
  : > "$1/scene.oct"
}
make_mock_parser() {
  cat > "$1" <<'PARSER'
#!/usr/bin/env python3
import json,sys
from pathlib import Path
args=sys.argv[1:]
def arg(name): return Path(args[args.index(name)+1])
grid=arg("--grid-tsv").read_text().splitlines()
raw=arg("--raw-output").read_text().splitlines()
tol=float(args[args.index("--coordinate-tolerance-m")+1])
if len(grid)!=len(raw) or not grid: sys.exit(3)
norm=[]; diag=[]
for g,line in zip(grid,raw):
    gf=g.split("\t"); vf=line.split()
    if len(gf)!=4 or len(vf)!=6: sys.exit(3)
    idx=int(gf[0]); pos=[float(v) for v in gf[1:]]
    val=[float(v) for v in vf]
    if any(abs(a-b)>tol for a,b in zip(val[:3],pos)): sys.exit(3)
    rgb=val[3:]
    if any(x<0 or not (x==x and abs(x)<float("inf")) for x in rgb): sys.exit(3)
    lux=47.4*rgb[0]+120.0*rgb[1]+11.6*rgb[2]
    if not (lux==lux and abs(lux)<float("inf")): sys.exit(3)
    norm.append({"grid_index":idx,"x_m":pos[0],"y_m":pos[1],"z_m":pos[2],"illuminance_lx":lux})
    diag.append({"grid_index":idx,"echoed_origin_m":val[:3],"irradiance_rgb":rgb,"conversion_id":"radiance_rgb_legacy_47_4_120_11_6_v1"})
arg("--normalized-output").write_text(json.dumps(norm,separators=(",",":"))+"\n")
arg("--diagnostics-output").write_text(json.dumps(diag,separators=(",",":"))+"\n")
PARSER
  chmod 700 "$1"
}
make_case "$TEST_ROOT/case"
make_mock_parser "$TEST_ROOT/parser-mock"
make_rtrace() {
  cat > "$1" <<'RTRACE'
#!/usr/bin/env bash
set -euo pipefail
while read -r x y z dx dy dz; do printf '%s %s %s 1 1 1\n' "$x" "$y" "$z"; done
RTRACE
  chmod 700 "$1"
}
make_rtrace "$TEST_ROOT/rtrace-ok"
PARSER="$TEST_ROOT/parser-mock"
if printenv LIGHTING_RTRACE_PARSER >/dev/null 2>&1; then PARSER=$(realpath "$(printenv LIGHTING_RTRACE_PARSER)"); fi
RTRACE_SHA=$(sha256sum "$TEST_ROOT/rtrace-ok"|awk '{print $1}')
PARSER_SHA=$(sha256sum "$PARSER"|awk '{print $1}')
run() {
  local name=$1 rtrace=$2 expected_rtrace=$3 parser=$4 expected_parser=$5 config=$6 timeout_s=$7
  bash "$RUNNER" --rtrace "$rtrace" --expected-rtrace-sha256 "$expected_rtrace" \
    --parser "$parser" --expected-parser-sha256 "$expected_parser" \
    --octree "$TEST_ROOT/case/scene.oct" --case-json "$TEST_ROOT/case/case.json" \
    --design-json "$TEST_ROOT/case/design.json" --property-data "$TEST_ROOT/case/property.bin" \
    --adapter-config "$config" --engine-version mock-radiance --timeout-seconds "$timeout_s" \
    --output-dir "$TEST_ROOT/out-$name"
}
run success "$TEST_ROOT/rtrace-ok" "$RTRACE_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/case/config.json" 5
jq -e '.run_status=="process_parse_ok" and .scoring_eligible==false and .service_energy_metric_wh_per_compliant_task_hour==null and .process.invocation_attempted==true and .artifacts.normalized_samples_sha256!=null and .execution.command_argv_sha256!=null and .execution.ray_input_sha256!=null and .execution.expected_grid_tsv_sha256!=null' "$TEST_ROOT/out-success/run-report.json" >/dev/null || fail "success report contract"
[[ $(jq 'length' "$TEST_ROOT/out-success/samples.json") -eq 2 ]] || fail "two normalized samples expected"
[[ $(grep -c -- '-I+' "$TEST_ROOT/out-success/command-argv.txt") -eq 1 ]] || fail "explicit irradiance flag not recorded"
[[ $(grep -c -- '-ov' "$TEST_ROOT/out-success/command-argv.txt") -eq 1 ]] || fail "origin+value output profile not recorded"
pass "successful mock process parses ordered samples but remains non-rankable"

cat > "$TEST_ROOT/rtrace-fail" <<'RTRACE'
#!/bin/sh
exit 7
RTRACE
chmod 700 "$TEST_ROOT/rtrace-fail"
FAIL_SHA=$(sha256sum "$TEST_ROOT/rtrace-fail"|awk '{print $1}')
if run nonzero "$TEST_ROOT/rtrace-fail" "$FAIL_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/case/config.json" 5; then fail "nonzero exit unexpectedly succeeded"; fi
jq -e '.run_status=="invalid_output" and .failure_code=="solver_nonzero_exit" and .process.exit_code==7 and .scoring_eligible==false' "$TEST_ROOT/out-nonzero/run-report.json" >/dev/null || fail "nonzero report"
pass "nonzero solver exit preserved and non-rankable"

cat > "$TEST_ROOT/rtrace-timeout" <<'RTRACE'
#!/bin/sh
sleep 5
RTRACE
chmod 700 "$TEST_ROOT/rtrace-timeout"
TIMEOUT_SHA=$(sha256sum "$TEST_ROOT/rtrace-timeout"|awk '{print $1}')
if run timeout "$TEST_ROOT/rtrace-timeout" "$TIMEOUT_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/case/config.json" 1; then fail "timeout unexpectedly succeeded"; fi
jq -e '.run_status=="timeout" and .process.timed_out==true and .scoring_eligible==false' "$TEST_ROOT/out-timeout/run-report.json" >/dev/null || fail "timeout report"
pass "timeout identified separately from a child exit code"

cat > "$TEST_ROOT/rtrace-exit124" <<'RTRACE'
#!/bin/sh
exit 124
RTRACE
chmod 700 "$TEST_ROOT/rtrace-exit124"
EXIT124_SHA=$(sha256sum "$TEST_ROOT/rtrace-exit124"|awk '{print $1}')
if run exit124 "$TEST_ROOT/rtrace-exit124" "$EXIT124_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/case/config.json" 5; then fail "child exit 124 unexpectedly succeeded"; fi
jq -e '.run_status=="invalid_output" and .process.exit_code==124 and .process.timed_out==false' "$TEST_ROOT/out-exit124/run-report.json" >/dev/null || fail "child 124 report"
pass "child exit 124 is not misclassified as supervisor timeout"

if run bad-pin "$TEST_ROOT/rtrace-ok" "$(printf '0%.0s' {1..64})" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/case/config.json" 5; then fail "wrong digest unexpectedly succeeded"; fi
jq -e '.run_status=="invalid_input" and .failure_code=="rtrace_digest_mismatch" and .process.invocation_attempted==false' "$TEST_ROOT/out-bad-pin/run-report.json" >/dev/null || fail "wrong digest report"
pass "rtrace digest mismatch rejected before invocation"

if run missing "$TEST_ROOT/no-such-rtrace" "$RTRACE_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/case/config.json" 5; then fail "missing binary unexpectedly succeeded"; fi
jq -e '.run_status=="unsupported" and .failure_code=="missing_executable" and .process.invocation_attempted==false and .scoring_eligible==false' "$TEST_ROOT/out-missing/run-report.json" >/dev/null || fail "missing binary report"
pass "missing executable produces explicit non-rankable report"

printf '{"schema_version":"0.1.0","profile":"legacy-rgb-v1","rtrace_args":[["-ov","1"]]}\n' > "$TEST_ROOT/config-reserved.json"
if run reserved "$TEST_ROOT/rtrace-ok" "$RTRACE_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/config-reserved.json" 5; then fail "reserved output option unexpectedly accepted"; fi
jq -e '.run_status=="invalid_input" and .failure_code=="invalid_adapter_config" and .scoring_eligible==false' "$TEST_ROOT/out-reserved/run-report.json" >/dev/null || fail "reserved option report"
pass "runner-owned output flags cannot be overridden by config"

printf '{"schema_version":"0.1.0","profile":"legacy-rgb-v1","rtrace_args":[["-ab","1"],["-ab","2"]]}\n' > "$TEST_ROOT/config-duplicate.json"
if run duplicate "$TEST_ROOT/rtrace-ok" "$RTRACE_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/config-duplicate.json" 5; then fail "duplicate option unexpectedly accepted"; fi
jq -e '.failure_code=="invalid_adapter_config" and .scoring_eligible==false' "$TEST_ROOT/out-duplicate/run-report.json" >/dev/null || fail "duplicate settings report"
pass "duplicate solver settings rejected before invocation"

if run bad-parser-pin "$TEST_ROOT/rtrace-ok" "$RTRACE_SHA" "$PARSER" "$(printf '0%.0s' {1..64})" "$TEST_ROOT/case/config.json" 5; then fail "wrong parser digest unexpectedly accepted"; fi
jq -e '.failure_code=="parser_digest_mismatch" and .process.invocation_attempted==false' "$TEST_ROOT/out-bad-parser-pin/run-report.json" >/dev/null || fail "parser digest report"
pass "parser digest mismatch rejected before invocation"

cat > "$TEST_ROOT/rtrace-mutable" <<'RTRACE'
#!/usr/bin/env bash
printf 'source replaced during invocation\n' > __MUTABLE_PATH__
while read -r x y z dx dy dz; do printf '%s %s %s 1 1 1\n' "$x" "$y" "$z"; done
RTRACE
sed -i "s|__MUTABLE_PATH__|$TEST_ROOT/rtrace-mutable|" "$TEST_ROOT/rtrace-mutable"
chmod 700 "$TEST_ROOT/rtrace-mutable"
MUTABLE_SHA=$(sha256sum "$TEST_ROOT/rtrace-mutable"|awk '{print $1}')
run mutation "$TEST_ROOT/rtrace-mutable" "$MUTABLE_SHA" "$PARSER" "$PARSER_SHA" "$TEST_ROOT/case/config.json" 5
jq -e --arg pin "$MUTABLE_SHA" '.run_status=="process_parse_ok" and .tools.staged_rtrace_sha256==$pin and .scoring_eligible==false and .execution.ray_input_sha256!=null' "$TEST_ROOT/out-mutation/run-report.json" >/dev/null || fail "staged source mutation report"
[[ "$(sha256sum "$TEST_ROOT/rtrace-mutable"|awk '{print $1}')" != "$MUTABLE_SHA" ]] || fail "mock did not replace original source path"
pass "execution uses the hashed staged copy despite source-path replacement"

echo "radiance-runner smoke: all scenarios passed; mocks only, no optical accuracy claim"
