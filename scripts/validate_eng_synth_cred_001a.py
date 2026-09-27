#!/usr/bin/env python3
import hashlib
import json
from pathlib import Path

PATH = Path("docs/release/evidence/eng-synth-cred-001a-engineering-deterministic-reference-v1.json")
EXPECTED_SHA256 = "891aa10b0040f6681d1288cce1243e6f85204800bb121f264aa45e45d0f93a7b"
EXPECTED_SCHEMA = "eng-synth-cred-001a-engineering-deterministic-reference-v1"
EXPECTED_AUTHORITY = "simulation_model_and_numerical_integrity_semantics_only_no_physical_validity_or_engineering_authority"
EXPECTED_BASE = "d56d6a6c1381dc131d493f0713b657f01c98cb80"
EXPECTED_BLOBS = {
    "crates/core/symtropy-physics/src/world.rs": "7dccf283f858c4874c96425fd7aa8aa3e176a557",
    "crates/core/symtropy-physics/src/integrator.rs": "c2e32f1bc14adf249f3b797e54209d501e491d9d",
    "crates/core/symtropy-physics/src/diagnostics.rs": "d4bab2ec7217471653315986eb77f0d13a895d44",
    "crates/core/symtropy-physics/src/bin/replay_cli.rs": "17673d1ab937ea3ccb282508a07b4b140add30e3",
    "crates/core/symtropy-math/src/halfspace.rs": "5ce17d01f793bc543037310bec737f463bb51e87",
}
EXPECTED_DISPOSITIONS = {
    "AdmissibleNoKnownNumericalAnomaly", "InvalidTimeStepBlocked",
    "NonFiniteInputBlocked", "NonFiniteStateBlocked",
    "LinearVelocityClampRetained", "AngularVelocityClampRetained",
    "PenetrationBiasClampRetained", "UndeclaredPhysicsCallbackBlocked",
    "DeclaredCallbackProfileBoundary", "UndeclaredMeshProviderBlocked",
    "DeclaredMeshProviderRetained", "RotationalInertiaApproximationRetained",
    "HalfSpacePairUnsupported", "HalfSpaceAnalyticalPath",
    "MeshFallbackApproximationRetained", "Q16ProfileDistinct",
    "ReplayConformantOnly", "ReplayMismatchBlocked", "InvariantDriftRetained",
    "UnsupportedPhenomenonBlocked", "ModelProfileMismatchBlocked",
    "TraceInvarianceBlocked",
}

def fail(msg):
    raise SystemExit("FAIL_ENG_SYNTH_CRED_001A_REFERENCE: " + msg)

def derive(x):
    if x["dt_state"] in {"Zero", "Negative", "NonFinite"}:
        return "InvalidTimeStepBlocked"
    if x["authored_numeric_input"] in {"NaNForce", "InfiniteTorque"}:
        return "NonFiniteInputBlocked"
    if x["post_state"] == "NonFinite":
        return "NonFiniteStateBlocked"
    if x["callback"] == "Undeclared":
        return "UndeclaredPhysicsCallbackBlocked"
    if x["mesh_provider"] == "Undeclared":
        return "UndeclaredMeshProviderBlocked"
    if x["requested_phenomenon"] == "Unsupported":
        return "UnsupportedPhenomenonBlocked"
    if x["model_profile_binding"] == "Mismatch":
        return "ModelProfileMismatchBlocked"
    if x["trace_behavior"] == "ChangedByTraceLevel":
        return "TraceInvarianceBlocked"
    if x["replay"] == "MismatchOrCorrupt":
        return "ReplayMismatchBlocked"
    if x["linear_velocity_guard"] == "Triggered":
        return "LinearVelocityClampRetained"
    if x["angular_velocity_guard"] == "Triggered":
        return "AngularVelocityClampRetained"
    if x["penetration_bias_guard"] == "Triggered":
        return "PenetrationBiasClampRetained"
    if x["callback"] == "DeclaredNonEngineeringProfile":
        return "DeclaredCallbackProfileBoundary"
    if x["mesh_provider"] == "Declared":
        return "DeclaredMeshProviderRetained"
    if x["contact_path"] == "HalfSpaceVsHalfSpace":
        return "HalfSpacePairUnsupported"
    if x["contact_path"] == "MeshWithoutProviderConvexFallback":
        return "MeshFallbackApproximationRetained"
    if x["contact_path"] == "ExactlyOneHalfSpaceAnalytical":
        return "HalfSpaceAnalyticalPath"
    if x["inertia_subject"] == "AsymmetricMeanApproximation":
        return "RotationalInertiaApproximationRetained"
    if x["determinism_profile"] == "Q16DeterministicNet":
        return "Q16ProfileDistinct"
    if x["replay"] == "ExactEnvironmentMatch":
        return "ReplayConformantOnly"
    if x["invariant_drift"] == "ExceededDeclaredDiagnosticEnvelope":
        return "InvariantDriftRetained"
    return "AdmissibleNoKnownNumericalAnomaly"

raw = PATH.read_bytes()
if hashlib.sha256(raw).hexdigest() != EXPECTED_SHA256:
    fail("corpus SHA-256 mismatch")
data = json.loads(raw)
canonical = (json.dumps(data, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()
if raw != canonical:
    fail("corpus bytes are not canonical compact sorted JSON plus newline")
if data.get("schema") != EXPECTED_SCHEMA or data.get("authority") != EXPECTED_AUTHORITY:
    fail("schema/authority mismatch")
if data.get("issue") != 1468 or data.get("source_repository") != "Luminous-Dynamics/symtropy":
    fail("issue/repository mismatch")
if data.get("source_base_head") != EXPECTED_BASE or data.get("source_audit_blobs") != EXPECTED_BLOBS:
    fail("source identity mismatch")
profile = data.get("profile", {})
if profile.get("stage") != "contract_only_no_runtime_enforcement":
    fail("contract stage was promoted")
if profile.get("physical_authority") is not False or profile.get("evidence_admission_authority") is not False:
    fail("authority inflation")
if set(data.get("dispositions", [])) != EXPECTED_DISPOSITIONS or len(data.get("dispositions", [])) != 22:
    fail("disposition vocabulary mismatch")
if data.get("determinism_profiles") != ["NativeFloatExactEnvironment", "Q16DeterministicNet", "PortableSemanticTolerance"]:
    fail("determinism profiles mismatch")
vocab = data.get("input_vocabularies", {})
defaults = data.get("case_defaults", {})
if set(defaults) != set(vocab):
    fail("default/vocabulary keys mismatch")
for k, v in defaults.items():
    if v not in vocab[k]:
        fail(f"default {k} outside vocabulary")
cases = data.get("cases", [])
if len(cases) != 26:
    fail(f"expected 26 cases, got {len(cases)}")
ids = [c.get("id") for c in cases]
if len(set(ids)) != 26 or ids != [f"C{i:02d}" for i in range(1, 27)]:
    fail("case IDs must be unique C01..C26")
seen = set()
for c in cases:
    if set(c) != {"id", "overrides", "expected"}:
        fail(f"{c.get('id')} unexpected case shape")
    x = dict(defaults)
    for k, v in c["overrides"].items():
        if k not in vocab or v not in vocab[k]:
            fail(f"{c['id']} override outside vocabulary")
        x[k] = v
    got = derive(x)
    if got != c["expected"]:
        fail(f"{c['id']} independent derivation {got} != {c['expected']}")
    seen.add(got)
if seen != EXPECTED_DISPOSITIONS:
    fail(f"outcome coverage incomplete: {sorted(EXPECTED_DISPOSITIONS-seen)}")
findings = {f["id"]: f["fact"] for f in data.get("current_engine_findings", [])}
if len(findings) != 14 or set(findings) != {f"F{i:02d}" for i in range(1, 15)}:
    fail("source finding census mismatch")
if "does not fall through" not in findings["F09"] or "HalfSpace-vs-HalfSpace remains unsupported" not in findings["F09"]:
    fail("current HalfSpace behavior not frozen")
if "do not increment NAN_ZEROED_COUNT" not in findings["F02"]:
    fail("uncounted input sanitization finding missing")
if "ignored ten-box stack" not in findings["F14"]:
    fail("solver-scaling negative finding missing")
if "not an engineering evidence commitment" not in findings["F13"]:
    fail("SYMTAPE hash authority boundary missing")
nonclaims = " | ".join(data.get("nonclaims", [])).lower()
for required in ["physical model validity", "engineering requirement satisfaction", "article safety or certification", "design acceptance", "physical execution or actuation authority"]:
    if required not in nonclaims:
        fail("missing nonclaim: " + required)
for override, expected in [
    ({"dt_state":"Zero"}, "InvalidTimeStepBlocked"),
    ({"callback":"Undeclared", "linear_velocity_guard":"Triggered"}, "UndeclaredPhysicsCallbackBlocked"),
    ({"contact_path":"ExactlyOneHalfSpaceAnalytical"}, "HalfSpaceAnalyticalPath"),
    ({"contact_path":"HalfSpaceVsHalfSpace"}, "HalfSpacePairUnsupported"),
    ({"requested_phenomenon":"Unsupported", "replay":"ExactEnvironmentMatch"}, "UnsupportedPhenomenonBlocked"),
    ({"trace_behavior":"ChangedByTraceLevel"}, "TraceInvarianceBlocked"),
]:
    x = dict(defaults); x.update(override)
    if derive(x) != expected:
        fail(f"hostile probe failed: {override}")
print(f"PASS_ENG_SYNTH_CRED_001A_REFERENCE digest={EXPECTED_SHA256} cases={len(cases)} outcomes={len(seen)} findings={len(findings)}")
