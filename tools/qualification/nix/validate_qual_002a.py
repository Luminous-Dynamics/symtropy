#!/usr/bin/env python3
"""Fail-closed validator for QUAL-002 capsule policy v1."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any

SCHEMA_ID = "luminous.qual-002-capsule-policy.v1"
PROFILE_ID = "nix-pure-sandbox-qualified-v1"

EXPECTED_POLICY: dict[str, Any] = {
    "schema_id": SCHEMA_ID,
    "profile_id": PROFILE_ID,
    "execution_class": "PureDerivation",
    "supported_systems": ["x86_64-linux"],
    "nix_policy": {
        "sandbox": True,
        "sandbox_fallback": False,
        "pure_eval": True,
        "restrict_eval": True,
        "allow_import_from_derivation": False,
        "accept_flake_config": False,
        "allow_unsafe_native_code_during_evaluation": False,
        "require_sigs": True,
        "substitute_theorem_result": False,
        "remote_builders_allowed": False,
        "remote_store_allowed": False,
        "plugin_files_allowed": False,
        "pre_build_hook_allowed": False,
        "post_build_hook_allowed": False,
        "diff_hook_allowed": False,
        "fixed_output_theorem_allowed": False,
        "impure_theorem_allowed": False,
        "no_chroot_theorem_allowed": False,
        "network_theorem_allowed": False,
    },
    "preparation_policy": {
        "network_allowed_for_content_pinned_fetches": True,
        "subject_code_execution_allowed": False,
        "all_remote_inputs_content_pinned": True,
    },
    "environment_policy": {
        "mode": "deny-ambient-allow-reviewed-suite",
        "base_set": {
            "HOME": "/homeless-shelter",
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "NIX_CONFIG": "",
            "NIX_PATH": "",
            "NIX_USER_CONF_FILES": "/dev/null",
            "SOURCE_DATE_EPOCH": "1",
            "TZ": "UTC",
        },
        "ambient_clear": [
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_HOME",
            "CC",
            "CXX",
            "LD_LIBRARY_PATH",
            "NIX_CACHE_HOME",
            "NIX_CONF_DIR",
            "NIX_CONFIG_HOME",
            "NIX_REMOTE",
            "NIX_SSL_CERT_FILE",
            "NIX_STATE_HOME",
            "PKG_CONFIG_PATH",
            "RUSTC_WRAPPER",
            "RUSTFLAGS",
            "RUSTUP_HOME",
            "SSL_CERT_FILE",
        ],
        "suite_owned_keys": [
            "CARGO_HOME",
            "CC",
            "CXX",
            "LD_LIBRARY_PATH",
            "PKG_CONFIG_PATH",
        ],
    },
    "source_identity_required": [
        "exact_parent",
        "manifest_digest",
        "owned_paths",
        "required_commit_count",
        "subject_commit_sha",
        "subject_repository",
        "subject_tree_sha",
    ],
    "evidence_required": [
        "builders_config",
        "capsule_policy_digest",
        "derivation_json_digest",
        "derivation_path",
        "effective_environment",
        "effective_nix_config",
        "hook_config",
        "nix_version",
        "output_path",
        "plugin_config",
        "sandbox_config",
        "source_nar_hash",
        "source_store_path",
        "store_config",
        "subject_identity",
        "substituter_config",
        "suite_id",
        "suite_steps",
        "system",
        "trust_config",
    ],
}

EXPECTED_ROOT_KEYS = set(EXPECTED_POLICY)


class PolicyError(ValueError):
    """Raised when the policy is malformed or weaker than v1 permits."""


def _reject_constant(value: str) -> None:
    raise PolicyError(f"non-finite JSON number is forbidden: {value}")


def _pairs_no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for key, value in pairs:
        if key in out:
            raise PolicyError(f"duplicate JSON key: {key}")
        out[key] = value
    return out


def _validate_value_types(value: Any, path: str) -> None:
    if isinstance(value, bool) or isinstance(value, str):
        return
    if isinstance(value, dict):
        for key, child in value.items():
            if not isinstance(key, str):
                raise PolicyError(f"{path}: object key must be a string")
            _validate_value_types(child, f"{path}.{key}")
        return
    if isinstance(value, list):
        for index, child in enumerate(value):
            _validate_value_types(child, f"{path}[{index}]")
        return
    raise PolicyError(
        f"{path}: only strings, booleans, arrays, and objects are permitted"
    )


def load_json_strict(path: Path) -> tuple[dict[str, Any], bytes]:
    raw = path.read_bytes()
    if raw.startswith(b"\xef\xbb\xbf"):
        raise PolicyError("UTF-8 BOM is forbidden")
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise PolicyError("policy must be valid UTF-8") from exc

    try:
        value = json.loads(
            text,
            object_pairs_hook=_pairs_no_duplicates,
            parse_constant=_reject_constant,
        )
    except json.JSONDecodeError as exc:
        raise PolicyError(f"invalid JSON: {exc.msg}") from exc

    if not isinstance(value, dict):
        raise PolicyError("root must be an object")
    _validate_value_types(value, "$")
    return value, raw


def validate_policy(policy: dict[str, Any]) -> None:
    _validate_value_types(policy, "$")
    if policy != EXPECTED_POLICY:
        raise PolicyError("policy must equal the frozen QUAL-002A v1 profile")

    env = policy["environment_policy"]
    if not set(env["suite_owned_keys"]).issubset(env["ambient_clear"]):
        raise PolicyError("suite-owned keys must first be cleared from ambient state")


def validate_file(path: Path) -> tuple[str, int]:
    policy, raw = load_json_strict(path)
    validate_policy(policy)
    return hashlib.sha256(raw).hexdigest(), len(raw)


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"usage: {argv[0]} POLICY.json", file=sys.stderr)
        return 2
    try:
        digest, byte_len = validate_file(Path(argv[1]))
    except (OSError, PolicyError) as exc:
        print(f"QUAL-002A policy rejected: {exc}", file=sys.stderr)
        return 1

    print(f"QUAL-002A policy valid sha256={digest} bytes={byte_len}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
