#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Manual two-process smoke test for the Lightyear 0.28 localhost UDP mode.
# This is not a test of Iroh, relay signaling, NAT traversal, or internet play.
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "This smoke helper currently requires Linux (GNU timeout)." >&2
    exit 2
fi
if ! command -v timeout >/dev/null 2>&1; then
    echo "GNU timeout is required for this smoke helper." >&2
    exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build --locked -p symtropy-multiplayer-demo
binary="$repo_root/target/debug/symtropy-multiplayer-demo"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/symtropy-lightyear-smoke.XXXXXX")"
server_pid=""
passed=false

cleanup() {
    if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
        kill -TERM "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi

    if [[ "$passed" == "true" ]]; then
        rm -rf "$tmp_dir"
    else
        echo "Smoke logs preserved at: $tmp_dir" >&2
        if [[ -f "$tmp_dir/server.log" ]]; then
            echo "--- server.log ---" >&2
            cat "$tmp_dir/server.log" >&2
        fi
        if [[ -f "$tmp_dir/client.log" ]]; then
            echo "--- client.log ---" >&2
            cat "$tmp_dir/client.log" >&2
        fi
    fi
}
trap cleanup EXIT

"$binary" --network-server >"$tmp_dir/server.log" 2>&1 &
server_pid=$!

# Give the server time to initialize, then require its own startup log.
# This check alone is not success evidence; the server must later observe this
# test client connecting and the client must observe replicated updates.
sleep 1
if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "Server process exited before the client started." >&2
    exit 1
fi
if ! grep -q 'LIGHTYEAR_SMOKE server_start_requested' "$tmp_dir/server.log"; then
    echo "Server startup marker was not recorded." >&2
    exit 1
fi

set +e
timeout --signal=TERM 12s \
    "$binary" --network-client --player-id 1 >"$tmp_dir/client.log" 2>&1
client_status=$?
set -e

if [[ "$client_status" -ne 124 && "$client_status" -ne 143 ]]; then
    echo "Client exited unexpectedly with status $client_status." >&2
    exit 1
fi
if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "Server process exited during the client smoke run." >&2
    exit 1
fi

updates="$(grep -c 'LIGHTYEAR_SMOKE replication_update' "$tmp_dir/client.log" || true)"
unique_states="$(
    grep 'LIGHTYEAR_SMOKE replication_update' "$tmp_dir/client.log" \
        | sed -nE 's/.*position=\(([^)]*)\).*/\1/p' \
        | sort -u \
        | wc -l \
        | tr -d '[:space:]'
)"
if [[ "$updates" -lt 3 ]]; then
    echo "Expected at least 3 replicated-state updates; observed $updates." >&2
    exit 1
fi
if [[ "$unique_states" -lt 3 ]]; then
    echo "Expected at least 3 distinct replicated positions; observed $unique_states." >&2
    exit 1
fi
if ! grep -q 'LIGHTYEAR_SMOKE client_connected' "$tmp_dir/server.log"; then
    echo "The test server did not record this client's successful connection." >&2
    exit 1
fi

echo "PASS: the server observed the client connection and the client received $updates updates across $unique_states distinct replicated positions over localhost UDP."
echo "Evidence is derived from a separate server and client process."
passed=true
