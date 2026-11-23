# Orbital Surface Support Bench

Status: synthetic 3D support-level sensing-envelope and conservative occlusion prototype.
Parent research issue: Symtropy #7136.
Consumes: the typed 3D voxel/candidate model from #7020.
Claim ceiling: synthetic 3D support-envelope certification only.

This crate does not duplicate voxelization, exterior flood-fill, surface-face extraction, candidate generation or centre-visibility evaluation. It consumes the typed GsdCase, CandidateView, and per-feature SurfaceSupport exported by orbital-gsd-kcoverage-bench.

The upstream support exporter preserves opposing thin-wall faces as separate planar supports; non-cancelling unions of exterior faces as a full-cell AABB with a normal cone enclosing each constituent face normal; and outward-rounded support extents and normal-cone error margins for quantization.

For each source-centre-visible candidate/support pair, this crate bounds support distance using a bounding sphere around the declared support AABB; ray-direction variation using asin(r / d0); FOV using centre angle plus support angular radius plus optical-axis quantization margin; incidence using centre angle plus support angular radius plus declared normal cone; and GSD using upper distance and upper incidence angle.

Occlusion uses the axis-aligned box enclosing every line segment from the sensor to the support AABB. If any non-target occupied voxel intersects this enclosure, the result is INDETERMINATE, not blocked. If no non-target occupied voxel intersects, the entire set of support rays is clear under this voxel occupancy model. The enclosing box may intersect cells that no actual ray touches; those are conservative abstentions, not false blocks.

Only CERTIFIED_VISIBLE contributes to full-support coverage. Centre-invisible pairs are reported separately as CENTRE_NOT_VISIBLE; centre-visible but inconclusive pairs remain INDETERMINATE. v0.1 makes no PARTIAL or CERTIFIED_BLOCKED claim because those require constructive proof of a non-zero failing support subset or of the entire support being blocked.

Run:

    cargo test --locked --manifest-path tools/orbital-surface-support-bench/Cargo.toml
    cargo run --quiet --locked --manifest-path tools/orbital-surface-support-bench/Cargo.toml --bin orbital-surface-support-bench -- docs/research/orbital-surface-support/fixtures/support-isolated-voxel-v0_1.json <40-hex-source-commit>
    python3 tools/orbital-surface-support-bench/tests/support_reference_oracle.py docs/research/orbital-surface-support/fixtures/support-isolated-voxel-v0_1.json <40-hex-source-commit>

The independent oracle reuses the already-qualified Python 3D candidate/centre-visibility oracle, then separately reconstructs planar and union supports, support bounds, range/FOV/incidence/GSD certificates and the conservative occupied-cell broad phase. Full Rust and Python receipts must match.

Frozen controls:
- SUPPORT-001-ISOLATED-VOXEL: an isolated single-voxel surface under a permissive synthetic camera model; source-centre views should include whole-support certificates.
- SUPPORT-002-CENTRE-PASS-SUPPORT-BOUND-ABSTAIN: the same voxel with insufficient pixel count so the source centre can still pass but the sensor lies within the support bounding sphere; those pairs must be INDETERMINATE.

The result remains synthetic geometric evidence. It does not establish camera calibration, real image quality, defect findability, NDE/POD, metrology accuracy, robot reachability, structural acceptance or flight readiness.
