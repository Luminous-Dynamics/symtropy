#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

import copy
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERIFIER = ROOT / "scripts" / "verify-retained-artifact.py"
REAL_REGISTRY = ROOT / "docs" / "release" / "evidence" / "retained-artifacts" / "universal-matter-v4.8.json"

PATCH = (
    b"diff --git a/a.txt b/a.txt\n"
    b"new file mode 100644\n"
    b"index 0000000..ce01362\n"
    b"--- /dev/null\n"
    b"+++ b/a.txt\n"
    b"@@ -0,0 +1 @@\n"
    b"+hello\n"
    b"diff --git a/b.txt b/b.txt\n"
    b"index 3367afd..3e75765 100644\n"
    b"--- a/b.txt\n"
    b"+++ b/b.txt\n"
    b"@@ -1 +1 @@\n"
    b"-old\n"
    b"+new\n"
)


def git_blob_sha1(data: bytes) -> str:
    h = hashlib.sha1()
    h.update(f"blob {len(data)}\0".encode("ascii"))
    h.update(data)
    return h.hexdigest()


def path_set_sha(paths: list[bytes]) -> str:
    return hashlib.sha256(b"\n".join(sorted(paths)) + b"\n").hexdigest()


def registry_for(data: bytes) -> dict:
    return {
        "schema": "symtropy.evidence.retained-artifact.v1",
        "artifact_id": "test-artifact",
        "artifact": {
            "logical_name": "fixture.patch",
            "media_type": "text/x-diff",
            "byte_authority": "sha256",
            "sha256": hashlib.sha256(data).hexdigest(),
            "byte_length": len(data),
            "git_blob_sha1_observation": git_blob_sha1(data),
        },
        "shape": {
            "profile": "unified-diff-same-path-v1",
            "newline_count": data.count(b"\n"),
            "diff_path_count": 2,
            "new_path_count": 1,
            "modified_path_count": 1,
            "deleted_path_count": 0,
            "renamed_path_count": 0,
            "copied_path_count": 0,
            "path_set_sha256": path_set_sha([b"a.txt", b"b.txt"]),
        },
        "availability": {
            "status": "EXTERNAL_RETAINED_BYTES",
            "github_blob_present": False,
            "execution_gate": "REQUIRE_EXACT_BYTE_MATCH",
            "known_external_filename": "fixture.patch",
        },
        "current_replay_context": {
            "preflight_pr": 1,
            "preflight_head": "1" * 40,
            "stage_a_parent_commit": "2" * 40,
            "stage_a_parent_tree": "3" * 40,
            "stage_a_run_id": "123",
            "stage_a_terminal_status": "NOT_NEEDED",
            "cargo_lock_repair": "NOT_AUTHORIZED_OR_REQUIRED",
        },
    }


def run(registry: dict, data: bytes | None = PATCH) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        reg = td / "registry.json"
        reg.write_text(json.dumps(registry, sort_keys=True) + "\n")
        cmd = [sys.executable, str(VERIFIER), "--registry", str(reg)]
        if data is not None:
            artifact = td / registry["artifact"]["logical_name"]
            artifact.write_bytes(data)
            cmd += ["--artifact", str(artifact)]
        return subprocess.run(cmd, text=True, capture_output=True, check=False)


def run_paths(registry_path: Path, artifact_path: Path | None = None) -> subprocess.CompletedProcess[str]:
    cmd = [sys.executable, str(VERIFIER), "--registry", str(registry_path)]
    if artifact_path is not None:
        cmd += ["--artifact", str(artifact_path)]
    return subprocess.run(cmd, text=True, capture_output=True, check=False)


def expect_ok(label: str, result: subprocess.CompletedProcess[str]) -> None:
    if result.returncode != 0:
        raise AssertionError(f"{label}: expected success\nstdout={result.stdout}\nstderr={result.stderr}")


def expect_fail(label: str, result: subprocess.CompletedProcess[str]) -> None:
    if result.returncode == 0:
        raise AssertionError(f"{label}: expected failure\nstdout={result.stdout}\nstderr={result.stderr}")


def main() -> int:
    base = registry_for(PATCH)

    expect_ok("valid synthetic artifact", run(base))
    expect_ok("registry-only validation", run(base, data=None))

    bad = copy.deepcopy(base)
    bad["artifact"]["sha256"] = "0" * 64
    expect_fail("wrong artifact sha", run(bad))

    bad = copy.deepcopy(base)
    bad["shape"]["path_set_sha256"] = "0" * 64
    expect_fail("wrong path-set sha", run(bad))

    bad = copy.deepcopy(base)
    bad["shape"]["new_path_count"] = 2
    bad["shape"]["modified_path_count"] = 0
    expect_fail("wrong shape counts", run(bad))

    bad = copy.deepcopy(base)
    bad["availability"]["github_blob_present"] = True
    expect_fail("availability contradiction", run(bad))

    bad = copy.deepcopy(base)
    bad["current_replay_context"]["stage_a_run_id"] = "run-123"
    expect_fail("nonnumeric run id", run(bad))

    bad = copy.deepcopy(base)
    bad["unexpected"] = True
    expect_fail("unknown registry key", run(bad))

    renamed_patch = PATCH.replace(b"diff --git a/b.txt b/b.txt", b"diff --git a/b.txt b/c.txt")
    renamed = registry_for(renamed_patch)
    renamed["shape"]["path_set_sha256"] = path_set_sha([b"a.txt", b"c.txt"])
    expect_fail("same-path profile rejects path-changing header", run(renamed, renamed_patch))

    with tempfile.TemporaryDirectory() as td_raw:
        td = Path(td_raw)
        reg = td / "registry.json"
        reg.write_text(json.dumps(base, sort_keys=True) + "\n")

        wrong_name = td / "wrong-name.patch"
        wrong_name.write_bytes(PATCH)
        expect_fail("artifact basename mismatch", run_paths(reg, wrong_name))

        target = td / "fixture.patch.real"
        target.write_bytes(PATCH)
        artifact_link = td / "fixture.patch"
        artifact_link.symlink_to(target.name)
        expect_fail("artifact symlink rejection", run_paths(reg, artifact_link))

        artifact_link.unlink()
        artifact_dir = td / "fixture.patch"
        artifact_dir.mkdir()
        expect_fail("artifact non-regular rejection", run_paths(reg, artifact_dir))

        registry_target = td / "registry.real.json"
        registry_target.write_text(json.dumps(base, sort_keys=True) + "\n")
        registry_link = td / "registry-link.json"
        registry_link.symlink_to(registry_target.name)
        expect_fail("registry symlink rejection", run_paths(registry_link))

    real = subprocess.run(
        [sys.executable, str(VERIFIER), "--registry", str(REAL_REGISTRY)],
        text=True, capture_output=True, check=False,
    )
    expect_ok("checked-in Universal Matter registry", real)

    print("PASS: retained artifact registry verifier regression suite (14/14)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
