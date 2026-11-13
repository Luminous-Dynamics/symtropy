# ROBOTICS-STANDARDS-EVIDENCE-CROSSWALK-001

Status: initial executable reference for version-pinned public standards scope and evidence-domain separation.

Parent issue: Symtropy #5016.

## Public scope register

| Standard | Published scope used by this profile | Explicit boundary |
|---|---|---|
| ISO 18646-5:2026 | Legged service-robot locomotion performance criteria and related test methods; complete-machine tests, sample/qualification/check-and-accept use | Not intended to verify/validate safety requirements |
| ISO 21423:2026 | Communications/interoperability between industrial mobile robots, fleet managers and other equipment/applications in shared industrial environments | Not safety-related; navigation is excluded |
| ISO 10218-1:2025 | Industrial robot as a partly completed machine / robot-product safety | Not the integrated application/cell assessment |
| ISO 10218-2:2025 | Industrial robot applications and cells; integration, commissioning, operation, maintenance and decommissioning | Not a substitute for robot-product evidence or other deployment classes |
| ISO/TS 15066:2016 | Collaborative industrial robot systems and work environment | Supplements industrial ISO 10218 collaborative application guidance; not a generic service-robot standard |

Official public sources:
- https://www.iso.org/standard/86850.html
- https://www.iso.org/standard/86749.html
- https://www.iso.org/standard/73933.html
- https://www.iso.org/standard/73934.html
- https://www.iso.org/standard/62996.html

The register records high-level public scope only. It is not a clause-by-clause requirements analysis. Do not reproduce copyrighted clauses; clause-level mappings require legitimate access to the official text and independent review.

## Evidence domains deliberately kept separate

- Performance.
- Robot-product safety.
- Application/cell safety.
- Collaborative-application safety.
- Communications interoperability.
- Navigation (explicitly not covered by ISO 21423).

The reference refuses to promote:
- ISO 18646-5 performance evidence into safety validation;
- ISO 21423 communication evidence into navigation or safety validation;
- ISO 10218-1 robot-product evidence into application/cell evidence, or the reverse;
- ISO/TS 15066 collaborative-application evidence into generic robot-product safety.

Applicability is a recorded decision by an accountable assessor. The code does not determine legal scope from a robot-type label.

## Output ceiling

A complete profile says only that each Applicable standard/domain has a scope-matched evidence receipt whose owner reports PassWithinDeclaredScope. It is not a standards-compliance certificate, conformity assessment, safety assessment or actuation permission. A passing software fixture validates only deterministic scope-check semantics.

## Run

    cargo fmt --manifest-path research/robotics-standards-evidence-crosswalk-001/reference/Cargo.toml -- --check
    cargo test --locked --manifest-path research/robotics-standards-evidence-crosswalk-001/reference/Cargo.toml
    cargo clippy --locked --manifest-path research/robotics-standards-evidence-crosswalk-001/reference/Cargo.toml --all-targets -- -D warnings

The exact-head Rust 1.96 workflow is the qualification gate.
