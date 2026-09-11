#!/usr/bin/env python3
"""Verify the exact Symtropy v0.11r3 semantic-oracle pilot evidence.

Authority inputs are intentionally minimal:
  * the exact canonical pilot-result artifact, and
  * GitHub/Sigstore verification performed by this process.

The verifier first authenticates the artifact and derives the exact GitHub run
ID / run attempt from the attestation certificate. It then queries GitHub
directly for that exact workflow-run attempt, attempt-specific jobs, and the
current experimental base ref. Caller-supplied provider JSON is not accepted.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable

VERSION = "0.13.4"
REPOSITORY = "Luminous-Dynamics/symtropy"
BASE_REF = "exp/promotion-oracle-base-v0.11r3"
WORKFLOW_PATH = "Luminous-Dynamics/symtropy/.github/workflows/promotion-oracle-execution-trusted-v011.yml"
JOB_NAME = "Trusted target-base semantic oracle / Trusted target-base game-state oracle"
RUNNER_LABEL = "ubuntu-24.04"
PREDICATE_TYPE = "https://slsa.dev/provenance/v1"
API_VERSION = "2026-03-10"
COMMAND_ID = "symtropy-game-state-promotion-oracle-v1"
SUT_PATH = "crates/domains/symtropy-game-state/src/lib.rs"
ORACLE_PATH = "crates/domains/symtropy-game-state/tests/promotion_oracle.rs"
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
DIGEST_RE = re.compile(r"^[0-9a-f]{64}$")
RUN_URI_RE = re.compile(
    r"^https://github\.com/Luminous-Dynamics/symtropy/actions/runs/([1-9][0-9]*)/attempts/([1-9][0-9]*)$"
)


class PilotEvidenceInvalid(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PilotEvidenceInvalid(message)


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8") + b"\n"


def digest_json(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_json_bytes(raw: bytes, label: str) -> Any:
    try:
        return json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PilotEvidenceInvalid(f"{label} must be UTF-8 JSON: {exc}") from exc


def valid_sha(name: str, value: Any) -> str:
    require(isinstance(value, str) and SHA_RE.fullmatch(value) is not None,
            f"{name} must be lowercase 40-hex SHA")
    return value


def valid_digest(name: str, value: Any) -> str:
    require(isinstance(value, str) and DIGEST_RE.fullmatch(value) is not None,
            f"{name} must be lowercase 64-hex SHA-256")
    return value


def valid_identity(name: str, value: Any, *, with_path: bool) -> dict[str, Any]:
    require(isinstance(value, dict), f"{name} must be object")
    fields = {"git_blob_sha", "raw_sha256"} | ({"path"} if with_path else set())
    require(set(value) == fields, f"{name} fields must exactly match schema")
    valid_sha(f"{name}.git_blob_sha", value["git_blob_sha"])
    valid_digest(f"{name}.raw_sha256", value["raw_sha256"])
    if with_path:
        require(isinstance(value["path"], str) and value["path"], f"{name}.path missing")
    return dict(value)


def validate_pilot_result(raw: bytes) -> dict[str, Any]:
    value = load_json_bytes(raw, "pilot result")
    require(canonical_json(value) == raw, "pilot result bytes must be canonical JSON with one trailing newline")
    require(isinstance(value, dict), "pilot result must be object")
    fields = {
        "schema_version", "kind", "repository", "decision", "oracle_exit_code",
        "base_sha", "head_sha", "trusted_workflow_sha", "command_id",
        "system_under_test", "authority_oracle", "trusted_build_inputs", "candidate_shadow",
    }
    require(set(value) == fields, "pilot result fields must exactly match schema v1")
    require(value["schema_version"] == 1 and value["kind"] == "promotion-semantic-oracle-execution-pilot",
            "unexpected pilot result generation")
    require(value["repository"] == REPOSITORY, "pilot result repository mismatch")
    require(value["decision"] in {"PASS", "FAIL"}, "invalid pilot decision")
    require(isinstance(value["oracle_exit_code"], int) and not isinstance(value["oracle_exit_code"], bool)
            and value["oracle_exit_code"] >= 0,
            "oracle_exit_code must be non-negative integer")
    require((value["decision"] == "PASS") == (value["oracle_exit_code"] == 0),
            "PASS iff oracle_exit_code is zero")
    for name in ("base_sha", "head_sha", "trusted_workflow_sha"):
        valid_sha(name, value[name])
    require(value["trusted_workflow_sha"] == value["base_sha"],
            "trusted workflow generation must equal exact target-base authority SHA")
    require(value["command_id"] == COMMAND_ID, "unexpected command_id")

    sut = valid_identity("system_under_test", value["system_under_test"], with_path=True)
    oracle = valid_identity("authority_oracle", value["authority_oracle"], with_path=True)
    require(sut["path"] == SUT_PATH, "unexpected system_under_test path")
    require(oracle["path"] == ORACLE_PATH, "unexpected authority_oracle path")

    build = value["trusted_build_inputs"]
    require(isinstance(build, dict) and set(build) == {"manifest", "lockfile", "toolchain"},
            "trusted_build_inputs fields mismatch")
    for name in ("manifest", "lockfile", "toolchain"):
        valid_identity(f"trusted_build_inputs.{name}", build[name], with_path=False)

    shadow = value["candidate_shadow"]
    require(isinstance(shadow, dict) and set(shadow) == {"oracle_raw_sha256", "differs_from_authority"},
            "candidate_shadow fields mismatch")
    valid_digest("candidate_shadow.oracle_raw_sha256", shadow["oracle_raw_sha256"])
    require(shadow["differs_from_authority"] is True, "candidate oracle must differ from authority")
    require(shadow["oracle_raw_sha256"] != oracle["raw_sha256"],
            "candidate shadow digest must differ from authority oracle digest")
    return dict(value)


def default_gh_runner(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, check=False, capture_output=True, text=True)


def certificate_from_verification(item: Any) -> dict[str, Any]:
    require(isinstance(item, dict), "verified attestation entry must be object")
    vr = item.get("verificationResult")
    require(isinstance(vr, dict), "verified attestation missing verificationResult")
    signature = vr.get("signature")
    require(isinstance(signature, dict), "verified attestation missing signature")
    cert = signature.get("certificate")
    require(isinstance(cert, dict), "verified attestation missing parsed certificate")
    return cert


def verify_attestation(
    artifact_path: Path,
    *,
    workflow_sha: str,
    gh_path: str,
    runner: Callable[[list[str]], subprocess.CompletedProcess[str]] = default_gh_runner,
) -> tuple[dict[str, Any], str, int, int]:
    cmd = [
        gh_path, "attestation", "verify", str(artifact_path),
        "--repo", REPOSITORY,
        "--signer-workflow", WORKFLOW_PATH,
        "--signer-digest", workflow_sha,
        "--predicate-type", PREDICATE_TYPE,
        "--deny-self-hosted-runners",
        "--format", "json",
    ]
    result = runner(cmd)
    require(result.returncode == 0, f"GitHub attestation verification failed: {result.stderr.strip()}")
    try:
        parsed = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise PilotEvidenceInvalid(f"gh attestation verify returned invalid JSON: {exc}") from exc
    require(isinstance(parsed, list) and parsed and all(isinstance(item, dict) for item in parsed),
            "gh attestation verify must return non-empty verification array")

    matches: list[tuple[dict[str, Any], int, int, str]] = []
    for item in parsed:
        cert = certificate_from_verification(item)
        uri = cert.get("runInvocationURI")
        match = RUN_URI_RE.fullmatch(uri) if isinstance(uri, str) else None
        if match is None:
            continue
        require(cert.get("runnerEnvironment") == "github-hosted",
                "matching attestation certificate must be github-hosted")
        require(cert.get("buildTrigger") == "pull_request",
                "matching attestation certificate must be triggered by pull_request")
        matches.append((item, int(match.group(1)), int(match.group(2)), uri))

    require(len(matches) == 1,
            f"expected exactly one unambiguous verified GitHub run attestation, got {len(matches)}")
    item, run_id, run_attempt, uri = matches[0]
    summary = {
        "verified_attestations": len(parsed),
        "matching_run_attestations": 1,
        "run_invocation_uri": uri,
        "run_id": run_id,
        "run_attempt": run_attempt,
    }
    return summary, digest_json(item), run_id, run_attempt


def gh_api_json(
    endpoint: str,
    *,
    gh_path: str,
    runner: Callable[[list[str]], subprocess.CompletedProcess[str]] = default_gh_runner,
) -> Any:
    cmd = [
        gh_path, "api", "--method", "GET",
        "-H", "Accept: application/vnd.github+json",
        "-H", f"X-GitHub-Api-Version: {API_VERSION}",
        endpoint,
    ]
    result = runner(cmd)
    require(result.returncode == 0, f"GitHub API query failed for {endpoint}: {result.stderr.strip()}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise PilotEvidenceInvalid(f"GitHub API returned invalid JSON for {endpoint}: {exc}") from exc


def fetch_provider_payloads(
    *,
    run_id: int,
    run_attempt: int,
    gh_path: str,
    runner: Callable[[list[str]], subprocess.CompletedProcess[str]] = default_gh_runner,
) -> tuple[Any, Any, Any]:
    run_endpoint = f"repos/{REPOSITORY}/actions/runs/{run_id}/attempts/{run_attempt}"
    jobs_endpoint = f"repos/{REPOSITORY}/actions/runs/{run_id}/attempts/{run_attempt}/jobs?per_page=100"
    ref_endpoint = f"repos/{REPOSITORY}/git/ref/heads/{BASE_REF}"
    return (
        gh_api_json(run_endpoint, gh_path=gh_path, runner=runner),
        gh_api_json(jobs_endpoint, gh_path=gh_path, runner=runner),
        gh_api_json(ref_endpoint, gh_path=gh_path, runner=runner),
    )


def select_exact_job(jobs_payload: Any, run_id: int) -> dict[str, Any]:
    require(isinstance(jobs_payload, dict), "attempt jobs payload must be object")
    jobs = jobs_payload.get("jobs")
    require(isinstance(jobs, list), "attempt jobs payload missing jobs list")
    total_count = jobs_payload.get("total_count")
    require(isinstance(total_count, int) and total_count >= 0, "attempt jobs total_count missing")
    require(total_count == len(jobs), "attempt jobs response is incomplete or paginated")
    matches = [
        item for item in jobs
        if isinstance(item, dict)
        and item.get("name") == JOB_NAME
        and item.get("run_id") == run_id
    ]
    require(len(matches) == 1, f"expected exactly one trusted pilot job, got {len(matches)}")
    return matches[0]


def validate_referenced_workflow(run_payload: dict[str, Any], workflow_sha: str) -> None:
    referenced = run_payload.get("referenced_workflows")
    require(isinstance(referenced, list), "attempt run missing referenced_workflows")
    expected_path = f"{WORKFLOW_PATH}@{workflow_sha}"
    matches = [
        item for item in referenced
        if isinstance(item, dict)
        and item.get("path") == expected_path
        and item.get("sha") == workflow_sha
    ]
    require(len(matches) == 1, "attempt run must reference exact trusted workflow generation once")


def derive_provider_observation(
    run_payload: Any,
    jobs_payload: Any,
    pilot: dict[str, Any],
    *,
    run_id: int,
    run_attempt: int,
) -> dict[str, Any]:
    require(isinstance(run_payload, dict), "attempt run payload must be object")
    require(run_payload.get("id") == run_id, "attempt run id mismatch")
    require(run_payload.get("run_attempt") == run_attempt, "attempt number mismatch")
    require(run_payload.get("event") == "pull_request", "pilot run must be pull_request event")
    require(run_payload.get("status") == "completed", "pilot run must be completed")
    repository = run_payload.get("repository")
    require(isinstance(repository, dict) and repository.get("full_name") == REPOSITORY,
            "attempt run repository mismatch")
    validate_referenced_workflow(run_payload, pilot["trusted_workflow_sha"])

    prs = run_payload.get("pull_requests")
    require(isinstance(prs, list) and len(prs) == 1, "pilot run requires exactly one pull request")
    pr = prs[0]
    require(isinstance(pr, dict), "pull request entry must be object")
    base = pr.get("base")
    head = pr.get("head")
    require(isinstance(base, dict) and isinstance(head, dict), "pull request base/head missing")
    require(base.get("sha") == pilot["base_sha"], "provider base SHA mismatch")
    require(head.get("sha") == pilot["head_sha"], "provider head SHA mismatch")

    job = select_exact_job(jobs_payload, run_id)
    require(job.get("status") == "completed", "trusted pilot job must be completed")
    conclusion = job.get("conclusion")
    expected_conclusion = "success" if pilot["decision"] == "PASS" else "failure"
    require(conclusion == expected_conclusion,
            f"{pilot['decision']} evidence requires provider job conclusion {expected_conclusion}")
    require(run_payload.get("conclusion") == expected_conclusion,
            f"{pilot['decision']} evidence requires provider run conclusion {expected_conclusion}")
    labels = job.get("labels")
    require(isinstance(labels, list) and RUNNER_LABEL in labels,
            f"trusted pilot job must execute on {RUNNER_LABEL}")
    runner_id = job.get("runner_id")
    require(isinstance(runner_id, int) and runner_id > 0, "trusted pilot job must have allocated hosted runner")
    steps = job.get("steps")
    require(isinstance(steps, list) and len(steps) > 0, "trusted pilot job must contain executed steps")

    return {
        "provider": "github-actions",
        "repository": REPOSITORY,
        "base_sha": pilot["base_sha"],
        "head_sha": pilot["head_sha"],
        "workflow_path": WORKFLOW_PATH,
        "workflow_sha": pilot["trusted_workflow_sha"],
        "runner_label": RUNNER_LABEL,
        "run_id": run_id,
        "run_attempt": run_attempt,
        "run_status": run_payload["status"],
        "run_conclusion": run_payload["conclusion"],
        "job_id": job["id"],
        "job_name": job["name"],
        "job_status": job["status"],
        "job_conclusion": conclusion,
        "job_materialized": True,
        "steps_materialized": True,
        "raw_run_digest": digest_json(run_payload),
        "raw_jobs_digest": digest_json(jobs_payload),
    }


def validate_current_base(value: Any, expected_base_sha: str) -> tuple[str, dict[str, str]]:
    require(isinstance(value, dict), "current base ref payload must be object")
    require(value.get("ref") == f"refs/heads/{BASE_REF}", "current base ref mismatch")
    obj = value.get("object")
    require(isinstance(obj, dict), "current base ref payload missing object")
    require(obj.get("type") == "commit", "current base ref must resolve to commit")
    current = valid_sha("current base commit sha", obj.get("sha"))
    require(current == expected_base_sha, "target base moved; pilot evidence is stale")
    return current, {"base_ref": BASE_REF, "sha": current}


def enrich(
    pilot_result_raw: bytes,
    *,
    artifact_path: Path,
    gh_path: str,
    runner: Callable[[list[str]], subprocess.CompletedProcess[str]] = default_gh_runner,
) -> dict[str, Any]:
    pilot = validate_pilot_result(pilot_result_raw)
    artifact_bytes = artifact_path.read_bytes()
    require(artifact_bytes == pilot_result_raw, "artifact file bytes differ from validated pilot result bytes")

    attestation_summary, attestation_digest, run_id, run_attempt = verify_attestation(
        artifact_path,
        workflow_sha=pilot["trusted_workflow_sha"],
        gh_path=gh_path,
        runner=runner,
    )
    run_payload, jobs_payload, current_base_payload = fetch_provider_payloads(
        run_id=run_id,
        run_attempt=run_attempt,
        gh_path=gh_path,
        runner=runner,
    )
    provider = derive_provider_observation(
        run_payload,
        jobs_payload,
        pilot,
        run_id=run_id,
        run_attempt=run_attempt,
    )
    current_sha, current_base_proof = validate_current_base(current_base_payload, provider["base_sha"])

    classifier_observation = {
        "schema_version": 1,
        "kind": "promotion-oracle-execution-observation",
        "provider": "github-actions",
        "repository": provider["repository"],
        "base_sha": provider["base_sha"],
        "head_sha": provider["head_sha"],
        "workflow_path": provider["workflow_path"],
        "workflow_sha": provider["workflow_sha"],
        "run_id": provider["run_id"],
        "run_attempt": provider["run_attempt"],
        "job_id": provider["job_id"],
        "provider_status": provider["job_status"],
        "provider_conclusion": provider["job_conclusion"],
        "job_materialized": provider["job_materialized"],
        "steps_materialized": provider["steps_materialized"],
        "semantic_result": pilot["decision"],
        "attestation_verified": True,
        "exact_subject_verified": True,
        "subject_current": True,
    }

    receipt = {
        "schema_version": 1,
        "kind": "promotion-oracle-pilot-evidence-verification",
        "adapter_version": VERSION,
        "repository": REPOSITORY,
        "base_ref": BASE_REF,
        "provider_observation_digest": digest_json(provider),
        "pilot_result_sha256": digest_bytes(pilot_result_raw),
        "current_base_proof_digest": digest_json(current_base_proof),
        "provider_query_digests": {
            "run_attempt": digest_json(run_payload),
            "jobs_attempt": digest_json(jobs_payload),
            "current_base": digest_json(current_base_payload),
        },
        "attestation_verification_digest": attestation_digest,
        "attestation_summary": attestation_summary,
        "semantic_result": pilot["decision"],
        "classifier_observation": classifier_observation,
    }
    receipt["verification_digest"] = digest_json(receipt)
    return receipt


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--version", action="version", version=VERSION)
    p.add_argument("--pilot-result", type=Path, required=True)
    p.add_argument("--gh", dest="gh_path")
    p.add_argument("--output", type=Path)
    return p


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    gh_path = args.gh_path or shutil.which("gh")
    if not gh_path:
        print("PILOT_EVIDENCE_INVALID: gh CLI is required for attestation and provider verification", file=sys.stderr)
        return 2
    try:
        raw = args.pilot_result.read_bytes()
        receipt = enrich(
            raw,
            artifact_path=args.pilot_result,
            gh_path=gh_path,
        )
        rendered = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 0
    except (OSError, json.JSONDecodeError, PilotEvidenceInvalid) as exc:
        print(f"PILOT_EVIDENCE_INVALID: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
