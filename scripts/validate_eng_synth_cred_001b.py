#!/usr/bin/env python3
"""Independent stdlib-only qualifier for ENG-SYNTH-CRED-001B frozen source reference.

This script validates a pinned source/data contract. It does not import or execute
Symtropy physics code and is not production runtime software.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

EXPECTED_COMMIT = "0b3b9a6f541b616a2132fee1493515f9bcf3d4c0"
REFERENCE_PATH = Path("docs/release/evidence/eng-synth-cred-001b-engine-capability-reference-v1.json")
EXPECTED_FIXTURE_COUNT = 30
EXPECTED_STATUSES = {
    "SupportedExactUnderProfile",
    "SupportedNumericalModel",
    "SupportedApproximation",
    "SupportedViaDeclaredProvider",
    "PartiallySupported",
    "Unsupported",
    "UnknownUncensused",
}

REQUIRED_FIXTURES = {
    "same_source_same_profile",
    "deterministic_net_feature_enabled",
    "halfspace_pair_generic_shape_trait",
    "exactly_one_halfspace",
    "rotated_4d_box_pair",
    "sphere_ccd_to_box",
    "ccd_speed_threshold",
    "mesh_provider_absent_vs_declared",
    "callback_api_vs_engineering_permission",
    "noop_vs_arbitrary_callback",
    "mean_inertia_stable_integration",
    "ignored_ten_box_stack",
    "healthy_predicate_with_energy_drift",
    "symtape_cli_as_library",
    "replay_tape_portability",
    "overlap_sensor_as_measurement",
    "raycast_as_perception",
    "joint_export_as_validation",
    "unknown_capability",
    "census_as_authority",
    "q16_coverage",
    "sleep_profile_change",
    "sleep_velocity_zeroing",
    "convex_hull_raycast",
    "replay_body_handle",
    "symtape_net_id",
    "health_with_model_limitation",
    "merge_toolhead_mass_properties",
    "solver_iteration_change",
    "callback_provider_determinism",
}


def fail(message: str) -> None:
    raise ValueError(message)


def git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        fail(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def git_show(root: Path, commit: str, path: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), "show", f"{commit}:{path}"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        fail(f"cannot read frozen source {path}: {result.stderr.strip()}")
    return result.stdout


def canonical_digest(reference: dict[str, Any]) -> str:
    unsigned = json.loads(json.dumps(reference))
    identity = unsigned.get("content_identity")
    if not isinstance(identity, dict):
        fail("content_identity must be an object")
    identity.pop("digest", None)
    canonical = json.dumps(
        unsigned,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=True,
    ).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def require_text(text: str, anchors: list[str], label: str) -> None:
    missing = [anchor for anchor in anchors if anchor not in text]
    if missing:
        fail(f"{label} missing source anchors: {missing}")


def qualify_source(root: Path, reference: dict[str, Any]) -> None:
    source = reference["source_identity"]
    commit = source["commit"]
    if commit != EXPECTED_COMMIT:
        fail(f"unexpected source commit: {commit}")
    git(root, "cat-file", "-e", f"{commit}^{{commit}}")

    locked_files = source["source_files"]
    if len(locked_files) != 25:
        fail(f"expected 25 locked source files, found {len(locked_files)}")

    texts: dict[str, str] = {}
    for path, expected_blob in sorted(locked_files.items()):
        actual_blob = git(root, "rev-parse", f"{commit}:{path}")
        if actual_blob != expected_blob:
            fail(
                f"source blob mismatch for {path}: expected {expected_blob}, "
                f"found {actual_blob}"
            )
        texts[path] = git_show(root, commit, path)

    cargo = texts["crates/core/symtropy-physics/Cargo.toml"]
    require_text(
        cargo,
        ['name = "symtropy-physics"', 'version = "0.2.1"', 'edition = "2024"', "deterministic-net = []"],
        "engine Cargo.toml",
    )

    world = texts["crates/core/symtropy-physics/src/world.rs"]
    require_text(
        world,
        [
            "quantize_q16_16(body.force_accumulator[i])",
            "quantize_q16_16(body.transform.translation.0[i])",
            "quantize_q16_16(body.linear_velocity[i])",
            "sleep_threshold: 0.5",
            "sleep_ticks: 60",
            "solver_iterations: 4",
            "CCD_SPEED_THRESHOLD",
            "downcast_ref::<Sphere<D>>()",
            "halfspace_pair_is_analytically_resolved",
            "box_box_pair_is_analytically_resolved",
            "step_with_callback",
            "NoOpCallback",
        ],
        "world.rs",
    )

    body = texts["crates/core/symtropy-physics/src/body.rs"]
    require_text(
        body,
        [
            "self.linear_velocity = SVector::zeros()",
            "self.angular_velocity = Bivector::zero()",
            "pub fn try_sleep",
            "0.4 * tool_mass * 0.1 * 0.1",
            "SVector::from_element(i_total)",
        ],
        "body.rs",
    )

    integrator = texts["crates/core/symtropy-physics/src/integrator.rs"]
    require_text(
        integrator,
        [
            "Semi-implicit Euler",
            "let inv_i_avg = body.inv_inertia.sum() / D as f64",
            "body.linear_velocity *= 1000.0 / speed",
            "body.angular_velocity.scale(100.0 / ang_speed)",
        ],
        "integrator.rs",
    )

    ccd = texts["crates/core/symtropy-physics/src/ccd.rs"]
    require_text(ccd, ["pub const CCD_SPEED_THRESHOLD: f64 = 5.0", "sphere_sphere", "sphere_halfspace"], "ccd.rs")

    raycast = texts["crates/core/symtropy-physics/src/raycast.rs"]
    require_text(
        raycast,
        [
            "raycast_body",
            "downcast_ref::<Sphere<D>>()",
            "downcast_ref::<HyperBox<D>>()",
            "downcast_ref::<Capsule<D>>()",
            "downcast_ref::<HalfSpace<D>>()",
            "bounding-sphere false positive",
        ],
        "raycast.rs",
    )

    diagnostics = texts["crates/core/symtropy-physics/src/diagnostics.rs"]
    health_start = diagnostics.index("pub fn is_numerically_healthy")
    health_end = diagnostics.index("/// Compare this snapshot", health_start)
    health = diagnostics[health_start:health_end]
    require_text(
        health,
        ["non_finite_body_count == 0", "max_rotation_orthogonality_error", "max_rotation_determinant_error"],
        "InvariantSnapshot::is_numerically_healthy",
    )
    if "mechanical_energy" in health or "linear_momentum" in health or "max_penetration_depth" in health:
        fail("numerical-health predicate unexpectedly includes wider diagnostics; refresh reference")

    replay = texts["crates/core/symtropy-physics/src/replay.rs"]
    cli = texts["crates/core/symtropy-physics/src/bin/replay_cli.rs"]
    require_text(replay, ["pub struct ReplayTape", "BodyHandle", "pub struct WorldSnapshot"], "replay.rs")
    require_text(cli, ["SYMTAPE\\0", "struct Fnv1a64", "NetId", "only 2/3/4 supported"], "replay_cli.rs")

    hinge = texts["crates/core/symtropy-physics/src/joints/hinge.rs"]
    hinge_motor_start = hinge.index("// Motor drive on the hinge rotation axis")
    hinge_motor_end = hinge.index("\n    }", hinge_motor_start)
    hinge_motor = hinge[hinge_motor_start:hinge_motor_end]
    require_text(
        hinge_motor,
        ["motor.target_vel", "let impulse = torque * _dt", "impulse * 0.5", "cb.record_work"],
        "hinge motor path",
    )
    if "motor.target_pos" in hinge_motor:
        fail("hinge target_pos appears in solver path; refresh reference")
    if "self.angle_limits" in texts["crates/core/symtropy-physics/src/joints/hinge.rs"]:
        # Declaration/build storage is expected; only solver consumption would be
        # a semantic change. Keep this test conservative and check solve sections.
        solve_text = texts["crates/core/symtropy-physics/src/joints/hinge.rs"]
        if "self.angle_limits" in solve_text[hinge_motor_start:]:
            fail("hinge solver now consumes angle_limits; refresh reference")

    contact = texts["crates/core/symtropy-physics/src/contact.rs"]
    manifold = texts["crates/core/symtropy-physics/src/manifold_gen.rs"]
    require_text(contact, ["pub const MAX_CONTACTS: usize = 8"], "contact.rs")
    require_text(manifold, ["let max_pts = MAX_CONTACTS.min(4)"], "manifold_gen.rs")

    shapes = texts["crates/core/symtropy-math/src/lib.rs"]
    require_text(
        shapes,
        ["pub use capsule::Capsule", "pub use compound::CompoundShape", "pub use convex_hull::ConvexHull",
         "pub use halfspace::HalfSpace", "pub use hyperbox::HyperBox", "pub use sphere::Sphere"],
        "symtropy-math exports",
    )


def qualify_manifest(reference: dict[str, Any]) -> None:
    if reference.get("schema") != "symtropy.engine_capability_census_reference.v1":
        fail("unexpected schema identifier")
    if reference.get("status") != "frozen_source_reference_pending_independent_qualification":
        fail("reference status must remain pending until this qualifier runs")
    if set(reference.get("capability_status_vocabulary", [])) != EXPECTED_STATUSES:
        fail("capability status vocabulary drifted")
    if reference["content_identity"]["algorithm"] != "SHA-256":
        fail("unsupported content identity algorithm")
    actual_digest = canonical_digest(reference)
    if actual_digest != reference["content_identity"]["digest"]:
        fail(
            "canonical content digest mismatch: "
            f"expected {reference['content_identity']['digest']}, found {actual_digest}"
        )

    fixtures = reference.get("fixtures", [])
    ids = [row.get("id") for row in fixtures]
    if len(fixtures) != EXPECTED_FIXTURE_COUNT:
        fail(f"expected {EXPECTED_FIXTURE_COUNT} fixtures, found {len(fixtures)}")
    if len(set(ids)) != len(ids):
        fail("duplicate fixture IDs")
    if set(ids) != REQUIRED_FIXTURES:
        fail(f"fixture ID set mismatch: missing={sorted(REQUIRED_FIXTURES-set(ids))}, unexpected={sorted(set(ids)-REQUIRED_FIXTURES)}")

    ccd = reference["ccd"]
    if ccd["caster"] != "dynamic, non-sleeping, non-sensor Sphere only":
        fail("CCD caster claim broadened or changed")
    if set(ccd["targets"]) != {"Sphere", "HalfSpace"}:
        fail("CCD target claim changed")
    q16 = reference["determinism"]["deterministic_net_q16_16"]
    if set(q16["quantized"]) != {
        "force_accumulator before integration",
        "translation after integration",
        "linear_velocity after integration",
    }:
        fail("Q16 quantization coverage changed")
    if "angular_velocity" not in q16["not_quantized"] or "rotation" not in q16["not_quantized"]:
        fail("Q16 unquantized angular coverage omitted")

    hinge = reference["joints"]["hinge"]
    if hinge["target_pos_consumed"] is not False or hinge["angle_limits_enforced"] is not False:
        fail("hinge target/limit claim changed")
    if reference["sleeping"]["default_ticks"] != 60 or reference["sleeping"]["default_threshold"] != 0.5:
        fail("sleep defaults changed")
    if reference["diagnostics"]["health_predicate_checks"] != [
        "non_finite_body_count == 0",
        "max_rotation_orthogonality_error <= supplied tolerance",
        "max_rotation_determinant_error <= supplied tolerance",
    ]:
        fail("numerical health predicate boundary changed")
    if reference["replay"]["symtape_cli"]["hash"] != "FNV-1a 64 non-cryptographic state hash":
        fail("SYMTAPE hash claim changed")
    if "physical-control authority" not in reference["claim_ceiling"]["does_not_establish"]:
        fail("claim ceiling incomplete")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument(
        "--reference",
        type=Path,
        default=REFERENCE_PATH,
        help="reference path relative to repo root unless absolute",
    )
    args = parser.parse_args()
    root = args.repo_root.resolve()
    ref_path = args.reference if args.reference.is_absolute() else root / args.reference

    try:
        reference = json.loads(ref_path.read_text(encoding="utf-8"))
        qualify_manifest(reference)
        qualify_source(root, reference)
    except (OSError, json.JSONDecodeError, KeyError, TypeError, ValueError) as exc:
        print(f"ENG-SYNTH-CRED-001B REJECT: {exc}", file=sys.stderr)
        return 1

    print(
        "ENG-SYNTH-CRED-001B REFERENCE QUALIFIED "
        f"source={reference['source_identity']['commit']} "
        f"blobs={len(reference['source_identity']['source_files'])} "
        f"fixtures={len(reference['fixtures'])} "
        f"sha256={reference['content_identity']['digest']}"
    )
    print("Claim ceiling: source capability declaration only; no physical validity or model-credibility claim.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
