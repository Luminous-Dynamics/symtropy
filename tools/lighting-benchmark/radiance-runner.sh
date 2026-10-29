#!/usr/bin/env bash
set -euo pipefail
# Bounded Radiance process harness; always emits a non-rankable process report.

fail() { echo "radiance-runner: FAIL: $*" >&2; exit 2; }
sha256() { sha256sum -- "$1" | awk '{print $1}'; }
valid_sha() { [[ "$1" =~ ^[a-f0-9]{64}$ ]]; }
usage() {
  echo "Usage: radiance-runner.sh --rtrace PATH --expected-rtrace-sha256 HEX --parser PATH --expected-parser-sha256 HEX --octree PATH --case-json PATH --design-json PATH --property-data PATH --adapter-config PATH --engine-version LABEL --timeout-seconds N --output-dir NEW_DIR"
}
RTRACE= EXPECTED_RTRACE_SHA= PARSER= EXPECTED_PARSER_SHA= OCTREE= CASE_JSON= DESIGN_JSON= PROPERTY_DATA= ADAPTER_CONFIG= ENGINE_VERSION= TIMEOUT_SECONDS= OUT_DIR=
while (($#)); do
  key=$1; shift
  case "$key" in
    --rtrace|--expected-rtrace-sha256|--parser|--expected-parser-sha256|--octree|--case-json|--design-json|--property-data|--adapter-config|--engine-version|--timeout-seconds|--output-dir)
      (($#)) || fail "missing value for $key"
      value=$1; shift
      case "$key" in
        --rtrace) RTRACE=$value;; --expected-rtrace-sha256) EXPECTED_RTRACE_SHA=$value;;
        --parser) PARSER=$value;; --expected-parser-sha256) EXPECTED_PARSER_SHA=$value;;
        --octree) OCTREE=$value;; --case-json) CASE_JSON=$value;; --design-json) DESIGN_JSON=$value;;
        --property-data) PROPERTY_DATA=$value;; --adapter-config) ADAPTER_CONFIG=$value;;
        --engine-version) ENGINE_VERSION=$value;; --timeout-seconds) TIMEOUT_SECONDS=$value;; --output-dir) OUT_DIR=$value;;
      esac ;;
    -h|--help) usage; exit 0;;
    *) usage >&2; fail "unknown argument $key";;
  esac
done
for value in "$RTRACE" "$EXPECTED_RTRACE_SHA" "$PARSER" "$EXPECTED_PARSER_SHA" "$OCTREE" "$CASE_JSON" "$DESIGN_JSON" "$PROPERTY_DATA" "$ADAPTER_CONFIG" "$ENGINE_VERSION" "$TIMEOUT_SECONDS" "$OUT_DIR"; do
  [[ -n "$value" ]] || { usage >&2; fail "all arguments are required"; }
done
valid_sha "$EXPECTED_RTRACE_SHA" || fail "expected rtrace digest must be 64 lowercase hex characters"
valid_sha "$EXPECTED_PARSER_SHA" || fail "expected parser digest must be 64 lowercase hex characters"
[[ "$TIMEOUT_SECONDS" =~ ^[0-9]+$ ]] && ((TIMEOUT_SECONDS>=1 && TIMEOUT_SECONDS<=86400)) || fail "timeout must be in [1,86400] seconds"
for tool in jq sha256sum timeout realpath awk date; do command -v "$tool" >/dev/null 2>&1 || fail "required command missing: $tool"; done
for f in "$OCTREE" "$CASE_JSON" "$DESIGN_JSON" "$PROPERTY_DATA" "$ADAPTER_CONFIG"; do
  [[ -f "$f" && -r "$f" ]] || fail "required input missing/unreadable: $f"
done
RTRACE_SOURCE=$RTRACE; PARSER_SOURCE=$PARSER
[[ ! -e "$RTRACE_SOURCE" ]] || RTRACE_SOURCE=$(realpath -- "$RTRACE_SOURCE")
[[ ! -e "$PARSER_SOURCE" ]] || PARSER_SOURCE=$(realpath -- "$PARSER_SOURCE")
OCTREE_SOURCE=$(realpath -- "$OCTREE"); CASE_SOURCE=$(realpath -- "$CASE_JSON")
DESIGN_SOURCE=$(realpath -- "$DESIGN_JSON"); PROPERTY_SOURCE=$(realpath -- "$PROPERTY_DATA")
CONFIG_SOURCE=$(realpath -- "$ADAPTER_CONFIG")
umask 077
mkdir -p -- "$(dirname -- "$OUT_DIR")"
mkdir -- "$OUT_DIR" 2>/dev/null || fail "output directory already exists or cannot be created"
OUT_DIR=$(realpath -- "$OUT_DIR")
cp --reflink=auto -- "$OCTREE_SOURCE" "$OUT_DIR/scene.oct"
cp --reflink=auto -- "$CASE_SOURCE" "$OUT_DIR/case.json"
cp --reflink=auto -- "$DESIGN_SOURCE" "$OUT_DIR/design.json"
cp --reflink=auto -- "$PROPERTY_SOURCE" "$OUT_DIR/property-data.bin"
cp --reflink=auto -- "$CONFIG_SOURCE" "$OUT_DIR/adapter-config.json"
chmod a-w -- "$OUT_DIR/scene.oct" "$OUT_DIR/case.json" "$OUT_DIR/design.json" "$OUT_DIR/property-data.bin" "$OUT_DIR/adapter-config.json"
RTRACE= PARSER= RTRACE_SHA= PARSER_SHA=
if [[ -f "$RTRACE_SOURCE" && -r "$RTRACE_SOURCE" && -x "$RTRACE_SOURCE" ]]; then
  cp --reflink=auto -- "$RTRACE_SOURCE" "$OUT_DIR/rtrace.pinned"; chmod 500 -- "$OUT_DIR/rtrace.pinned"
  RTRACE="$OUT_DIR/rtrace.pinned"; RTRACE_SHA=$(sha256 "$RTRACE")
fi
if [[ -f "$PARSER_SOURCE" && -r "$PARSER_SOURCE" && -x "$PARSER_SOURCE" ]]; then
  cp --reflink=auto -- "$PARSER_SOURCE" "$OUT_DIR/parser.pinned"; chmod 500 -- "$OUT_DIR/parser.pinned"
  PARSER="$OUT_DIR/parser.pinned"; PARSER_SHA=$(sha256 "$PARSER")
fi
OCTREE=$OUT_DIR/scene.oct; CASE_JSON=$OUT_DIR/case.json; DESIGN_JSON=$OUT_DIR/design.json
PROPERTY_DATA=$OUT_DIR/property-data.bin; ADAPTER_CONFIG=$OUT_DIR/adapter-config.json
OCTREE_SHA=$(sha256 "$OCTREE"); CASE_SHA=$(sha256 "$CASE_JSON"); DESIGN_SHA=$(sha256 "$DESIGN_JSON")
PROPERTY_SHA=$(sha256 "$PROPERTY_DATA"); CONFIG_SHA=$(sha256 "$ADAPTER_CONFIG")
RUN_STATUS=preflight; FAILURE_CODE=; FAILURE_DETAIL=; INVOCATION_ATTEMPTED=false; EXIT_CODE=null; TIMED_OUT=false; DURATION_MS=null
STDOUT_SHA=; STDERR_SHA=; SUPERVISOR_SHA=; SAMPLES_SHA=; DIAGNOSTICS_SHA=
GRID_JSON_SHA=; GRID_TSV_SHA=; RAYS_SHA=; ARGV_SHA=; ENV_SHA=

write_report() {
  jq -n \
    --arg case_id "$(jq -r '.case_id // "unknown"' "$CASE_JSON" 2>/dev/null || echo unknown)" \
    --arg engine "$ENGINE_VERSION" --arg status "$RUN_STATUS" --arg code "$FAILURE_CODE" --arg detail "$FAILURE_DETAIL" \
    --arg rsrc "$RTRACE_SOURCE" --arg psrc "$PARSER_SOURCE" --arg rstage "$RTRACE" --arg pstage "$PARSER" \
    --arg ersha "$EXPECTED_RTRACE_SHA" --arg rsha "$RTRACE_SHA" --arg epsha "$EXPECTED_PARSER_SHA" --arg psha "$PARSER_SHA" \
    --arg octsha "$OCTREE_SHA" --arg casesha "$CASE_SHA" --arg designsha "$DESIGN_SHA" --arg propsha "$PROPERTY_SHA" --arg cfgsha "$CONFIG_SHA" \
    --arg outsha "$STDOUT_SHA" --arg errsha "$STDERR_SHA" --arg supsha "$SUPERVISOR_SHA" --arg samsha "$SAMPLES_SHA" --arg diagsha "$DIAGNOSTICS_SHA" \
    --arg attempted "$INVOCATION_ATTEMPTED" --arg exitcode "$EXIT_CODE" --arg timeout "$TIMED_OUT" --arg duration "$DURATION_MS" --arg gridjsonsha "$GRID_JSON_SHA" --arg gridtsvsha "$GRID_TSV_SHA" --arg rayssha "$RAYS_SHA" --arg argvsha "$ARGV_SHA" --arg envsha "$ENV_SHA" \
    '{report_version:"0.2.0",report_kind:"radiance_process_attempt",case_id:$case_id,engine_version_label:$engine,run_status:$status,
      failure_code:(if $code=="" then null else $code end),failure_detail:(if $detail=="" then null else $detail end),
      process:{invocation_attempted:($attempted=="true"),exit_code:(if $exitcode=="null" then null else ($exitcode|tonumber) end),
        timed_out:($timeout=="true"),duration_ms:(if $duration=="null" then null else ($duration|tonumber) end)},
      tools:{rtrace_source_path:$rsrc,rtrace_staged_path:(if $rstage=="" then null else $rstage end),expected_rtrace_sha256:$ersha,staged_rtrace_sha256:(if $rsha=="" then null else $rsha end),
        parser_source_path:$psrc,parser_staged_path:(if $pstage=="" then null else $pstage end),expected_parser_sha256:$epsha,staged_parser_sha256:(if $psha=="" then null else $psha end)},
      inputs:{compiled_octree_sha256:$octsha,case_sha256:$casesha,design_sha256:$designsha,property_data_sha256:$propsha,adapter_config_sha256:$cfgsha},
      artifacts:{stdout_sha256:(if $outsha=="" then null else $outsha end),stderr_sha256:(if $errsha=="" then null else $errsha end),
        supervisor_stderr_sha256:(if $supsha=="" then null else $supsha end),normalized_samples_sha256:(if $samsha=="" then null else $samsha end),
        parser_diagnostics_sha256:(if $diagsha=="" then null else $diagsha end)},
      execution:{expected_grid_json_sha256:(if $gridjsonsha=="" then null else $gridjsonsha end),expected_grid_tsv_sha256:(if $gridtsvsha=="" then null else $gridtsvsha end),ray_input_sha256:(if $rayssha=="" then null else $rayssha end),command_argv_sha256:(if $argvsha=="" then null else $argvsha end),child_environment_sha256:(if $envsha=="" then null else $envsha end)},
      scoring_eligible:false,service_energy_metric_wh_per_compliant_task_hour:null}' > "$OUT_DIR/run-report.json"
}
report_failure() { RUN_STATUS=$1; FAILURE_CODE=$2; FAILURE_DETAIL=$3; write_report; echo "radiance-runner: $RUN_STATUS/$FAILURE_CODE; report=$OUT_DIR/run-report.json" >&2; exit 1; }

if ! jq -e '
  def allowed($o): (["-ab","-ad","-as","-ar","-aa","-ds","-dt","-dc","-dr","-dp","-dj","-ss","-st","-lw"]|index($o))!=null;
  .schema_version=="0.1.0" and .profile=="legacy-rgb-v1" and (.rtrace_args|type=="array" and length>=1 and length<=32)
  and ((.rtrace_args|map(.[0])|unique|length)==(.rtrace_args|length))
  and all(.rtrace_args[]; . as $p | type=="array" and length==2 and ($p[0]|type=="string" and allowed(.))
    and ($p[1]|type=="string" and test("^([0-9]+(\\.[0-9]*)?|\\.[0-9]+)([eE][+-]?[0-9]+)?$"))
    and (($p[1]|tonumber) as $v | ($v|isfinite) and $v>=0)
    and (if (["-aa","-ds","-dt","-dc","-dj","-st"]|index($p[0]))!=null then (($p[1]|tonumber)<=1) else true end)
    and (if (["-ab","-ad","-as","-ar","-dr","-dp"]|index($p[0]))!=null then (($p[1]|tonumber)==(($p[1]|tonumber)|floor)) else true end)
  )' "$ADAPTER_CONFIG" >/dev/null; then
  report_failure invalid_input invalid_adapter_config "schema/profile/setting allowlist failed; runner-owned output flags cannot be overridden."
fi
mapfile -t SETTINGS_ARGS < <(jq -r '.rtrace_args[]|.[]' "$ADAPTER_CONFIG")
[[ -n "$RTRACE" && -x "$RTRACE" ]] || report_failure unsupported missing_executable "rtrace unavailable; no invocation attempted."
[[ -n "$PARSER" && -x "$PARSER" ]] || report_failure unsupported parser_unavailable "strict parser unavailable; no invocation attempted."
[[ "$RTRACE_SHA" == "$EXPECTED_RTRACE_SHA" ]] || report_failure invalid_input rtrace_digest_mismatch "staged executable does not match supplied pin."
[[ "$PARSER_SHA" == "$EXPECTED_PARSER_SHA" ]] || report_failure invalid_input parser_digest_mismatch "staged parser does not match supplied pin."
jq -e '.case_id|type=="string" and length>0' "$CASE_JSON" >/dev/null || report_failure invalid_input invalid_case "case_id missing."
if ! jq -e '.task_surface as $t|$t.measurement_grid as $g|($g.spacing_m|type=="number" and .>0) as $ok|(($t.length_x_m/$g.spacing_m)|round) as $nx|(($t.width_y_m/$g.spacing_m)|round) as $ny|$ok and (($t.length_x_m/$g.spacing_m)-$nx|abs)<1e-9 and (($t.width_y_m/$g.spacing_m)-$ny|abs)<1e-9 and ($t.origin_xyz_m|length)==3 and all($t.origin_xyz_m[];type=="number" and isfinite) and ($g.expected_points==(($nx+1)*($ny+1)))' "$CASE_JSON" >/dev/null; then
  report_failure invalid_input invalid_case_grid "case geometry does not define a finite regular grid."
fi
jq -c '.task_surface as $t|$t.measurement_grid.spacing_m as $s|(($t.length_x_m/$s)|round) as $nx|(($t.width_y_m/$s)|round) as $ny|($nx+1) as $w|[range(0;$w*($ny+1)) as $i|{grid_index:$i,x_m:($t.origin_xyz_m[0]+(($i%$w)*$s)),y_m:($t.origin_xyz_m[1]+((($i/$w)|floor)*$s)),z_m:$t.origin_xyz_m[2]}]' "$CASE_JSON" > "$OUT_DIR/expected-grid.json"
jq -r '.[]|"\(.grid_index)\t\(.x_m)\t\(.y_m)\t\(.z_m)"' "$OUT_DIR/expected-grid.json" > "$OUT_DIR/expected-grid.tsv"
while IFS=$'\t' read -r idx x y z; do
  [[ "$idx" =~ ^[0-9]+$ ]] || report_failure invalid_input generated_grid_invalid "grid index invalid."
  printf '%s %s %s 0 0 1\n' "$(awk -v n="$x" 'BEGIN{printf "%.17e",n}')" "$(awk -v n="$y" 'BEGIN{printf "%.17e",n}')" "$(awk -v n="$z" 'BEGIN{printf "%.17e",n}')"
done < "$OUT_DIR/expected-grid.tsv" > "$OUT_DIR/rtrace-input.txt"
printf '%s\n' "$(jq -r '.rtrace_args[]|.[]' "$ADAPTER_CONFIG")" '-h-' '-I+' '-ov' '-cs' '3' '-x' '1' "$OUT_DIR/scene.oct" > "$OUT_DIR/command-argv.txt"
printf '%s\n' 'LC_ALL=C' 'LANG=C' 'PATH=/usr/bin:/bin' 'RAYPATH=.' > "$OUT_DIR/child-environment.txt"
GRID_JSON_SHA=$(sha256 "$OUT_DIR/expected-grid.json"); GRID_TSV_SHA=$(sha256 "$OUT_DIR/expected-grid.tsv")
RAYS_SHA=$(sha256 "$OUT_DIR/rtrace-input.txt"); ARGV_SHA=$(sha256 "$OUT_DIR/command-argv.txt"); ENV_SHA=$(sha256 "$OUT_DIR/child-environment.txt")
[[ "$(sha256 "$RTRACE")" == "$RTRACE_SHA" && "$(sha256 "$PARSER")" == "$PARSER_SHA" ]] || report_failure invalid_input staged_tool_changed_before_execution "staged tool bytes changed before run."
cd "$OUT_DIR"
START_NS=$(date +%s%N); INVOCATION_ATTEMPTED=true
set +e
LC_ALL=C timeout --verbose --foreground --signal=TERM --kill-after=2s "${TIMEOUT_SECONDS}s" \
  /bin/sh -c 'errfile=$1; shift; exec "$@" 2>"$errfile"' _ "$OUT_DIR/stderr.raw" \
  /usr/bin/env -i LC_ALL=C LANG=C PATH=/usr/bin:/bin RAYPATH=. "$RTRACE" "${SETTINGS_ARGS[@]}" -h- -I+ -ov -cs 3 -x 1 "$OUT_DIR/scene.oct" \
  < "$OUT_DIR/rtrace-input.txt" > "$OUT_DIR/stdout.raw" 2> "$OUT_DIR/supervisor.stderr"
EXIT_CODE=$?; set -e
END_NS=$(date +%s%N); DURATION_MS=$(((END_NS-START_NS)/1000000))
STDOUT_SHA=$(sha256 "$OUT_DIR/stdout.raw"); STDERR_SHA=$(sha256 "$OUT_DIR/stderr.raw"); SUPERVISOR_SHA=$(sha256 "$OUT_DIR/supervisor.stderr")
if grep -q 'sending signal' "$OUT_DIR/supervisor.stderr"; then TIMED_OUT=true; fi
[[ "$TIMED_OUT" == false ]] || report_failure timeout solver_timeout "timeout supervisor sent termination signal; status $EXIT_CODE."
[[ "$EXIT_CODE" == 0 ]] || report_failure invalid_output solver_nonzero_exit "rtrace exited with status $EXIT_CODE."
[[ "$(sha256 "$RTRACE")" == "$RTRACE_SHA" && "$(sha256 "$PARSER")" == "$PARSER_SHA" ]] || report_failure invalid_output staged_tool_changed_after_run "staged tool bytes changed during run."
if ! "$PARSER" --grid-tsv "$OUT_DIR/expected-grid.tsv" --raw-output "$OUT_DIR/stdout.raw" --normalized-output "$OUT_DIR/samples.json" --diagnostics-output "$OUT_DIR/parser-diagnostics.json" --coordinate-tolerance-m 1e-6 > "$OUT_DIR/parser.stdout" 2> "$OUT_DIR/parser.stderr"; then
  report_failure invalid_output parser_rejected_output "strict parser rejected native rtrace output."
fi
SAMPLES_SHA=$(sha256 "$OUT_DIR/samples.json"); DIAGNOSTICS_SHA=$(sha256 "$OUT_DIR/parser-diagnostics.json")
jq -e 'length>0 and all(.[]; .conversion_id=="radiance_rgb_legacy_47_4_120_11_6_v1" and (.irradiance_rgb|length)==3)' "$OUT_DIR/parser-diagnostics.json" >/dev/null || report_failure invalid_output parser_diagnostics_invalid "unexpected parser diagnostics/conversion."
RUN_STATUS=process_parse_ok; FAILURE_CODE=; FAILURE_DETAIL=
write_report
echo "radiance-runner: process and parser checks passed; score disabled; report=$OUT_DIR/run-report.json"
