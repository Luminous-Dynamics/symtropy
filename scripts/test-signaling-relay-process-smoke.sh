#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/.." && pwd)"
cd "${repo_root}"

artifact_dir="${SYMTROPY_SMOKE_ARTIFACT_DIR:-${repo_root}/target/signaling-relay-smoke}"
mkdir -p "${artifact_dir}"
rm -f "${artifact_dir}/server.log" "${artifact_dir}/alpha.log" "${artifact_dir}/beta.log" "${artifact_dir}/summary.txt"

server_pid=""
cleanup() {
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

echo "[smoke] building server and independent smoke client"
cargo build --locked -p symtropy-net --no-default-features --features ws-relay \
  --bin symtropy-signaling-server --bin symtropy-signaling-smoke-client

target_dir="${CARGO_TARGET_DIR:-${repo_root}/target}"
server_bin="${target_dir}/debug/symtropy-signaling-server"
client_bin="${target_dir}/debug/symtropy-signaling-smoke-client"
if [[ ! -x "${server_bin}" || ! -x "${client_bin}" ]]; then
  echo "[smoke] expected binaries were not produced" >&2
  exit 1
fi

token="$(openssl rand -base64 36 | tr '+/' '-_' | tr -d '=\n')"
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
room="symtropy-smoke-${GITHUB_RUN_ID:-local}-${RANDOM}-$$"
signal_url="ws://127.0.0.1:${port}"

SYMTROPY_RELAY_TOKEN="${token}" \
SYMTROPY_RELAY_BIND="127.0.0.1:${port}" \
  "${server_bin}" >"${artifact_dir}/server.log" 2>&1 &
server_pid=$!

ready=0
for _ in $(seq 1 100); do
  if ! kill -0 "${server_pid}" 2>/dev/null; then
    echo "[smoke] server exited before becoming ready" >&2
    cat "${artifact_dir}/server.log" >&2 || true
    exit 1
  fi
  if (exec 3<>"/dev/tcp/127.0.0.1/${port}") 2>/dev/null; then
    ready=1
    break
  fi
  sleep 0.1
done
if [[ "${ready}" != "1" ]]; then
  echo "[smoke] server failed to accept a local TCP connection" >&2
  cat "${artifact_dir}/server.log" >&2 || true
  exit 1
fi

echo "[smoke] launching alpha and beta as separate OS processes"
SYMTROPY_SIGNAL_URL="${signal_url}" \
SYMTROPY_SIGNAL_TOKEN="${token}" \
SYMTROPY_SMOKE_ROLE=alpha \
SYMTROPY_SMOKE_ROOM="${room}" \
  "${client_bin}" >"${artifact_dir}/alpha.log" 2>&1 &
alpha_pid=$!

SYMTROPY_SIGNAL_URL="${signal_url}" \
SYMTROPY_SIGNAL_TOKEN="${token}" \
SYMTROPY_SMOKE_ROLE=beta \
SYMTROPY_SMOKE_ROOM="${room}" \
  "${client_bin}" >"${artifact_dir}/beta.log" 2>&1 &
beta_pid=$!

set +e
wait "${alpha_pid}"
alpha_status=$?
wait "${beta_pid}"
beta_status=$?
set -e

if [[ "${alpha_status}" != "0" || "${beta_status}" != "0" ]]; then
  echo "[smoke] failed: alpha_status=${alpha_status} beta_status=${beta_status}" >&2
  for log in server alpha beta; do
    echo "===== ${log}.log =====" >&2
    cat "${artifact_dir}/${log}.log" >&2 || true
  done
  echo "[smoke] logs retained at ${artifact_dir}" >&2
  exit 1
fi

{
  echo "result=pass"
  echo "test=two-process-signaling-relay-bidirectional-payload"
  echo "workflow_sha=${GITHUB_SHA:-local-unrecorded}"
  echo "clients=2 independent OS processes"
  echo "transport=localhost WebSocket to experimental server binary"
  echo "observed_directions=alpha-to-beta,beta-to-alpha"
  echo "public_service_interoperability=not_tested"
  echo "production_identity_security=not_tested"
} >"${artifact_dir}/summary.txt"

cat "${artifact_dir}/alpha.log"
cat "${artifact_dir}/beta.log"
echo "[smoke] PASS: two independent clients exchanged expected payloads in both directions"
echo "[smoke] evidence retained at ${artifact_dir}"
