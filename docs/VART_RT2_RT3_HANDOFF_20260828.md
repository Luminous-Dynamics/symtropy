# VART RT2/RT3 Handoff — 2026-08-28

Status: implementation landed on `art/realtime-studio-v2`; Rust/Nix qualification pending.

## RT2 — single-frame off-screen GPU observation

`art_offscreen` now provides a real Bevy `Image` render-target/readback path:

1. validate an `ArtCaptureRequest` against an independently supplied `ArtRenderStamp`;
2. allocate a dedicated render-target `Image` with `COPY_SRC` enabled;
3. temporarily bind one camera to that image;
4. after exactly one host render pass, restore the camera;
5. queue asynchronous `Readback::texture` on the now-detached image;
6. emit bounded raw bytes plus an `ArtCaptureReceipt` and byte digest;
7. despawn the readback entity after completion so it cannot silently sample future frames.

The dedicated-image rule is important: shared continuously rendered targets can be overwritten between request and asynchronous readback. A single-use target makes frame evidence much easier to reason about.

Current RT2 scope is intentionally color-only. Depth, normals, object ID, and motion require dedicated render/prepass provenance rather than pretending those channels already exist.

## RT3 — isolated canonical preview scene

`art_preview_scene` clones only canonical `ArtSceneRecord` values. It owns no main Bevy `World`, `Commands`, or committed revision mutator. Preview transform, visibility, and material changes therefore alter only the branch copy.

Each preview starts by verifying the claimed committed scene hash. After arbitrary preview edits, callers can re-hash the actual committed records and prove they remain identical to the base.

This is the deterministic intermediate form for proposal ghosts. A later renderer may materialize these records into a dedicated render layer or secondary world, but the artistic proposal is already isolated from the committed scene.

## Qualification

Run on the target Nix/Rust environment:

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings
```

Then add a live Bevy conformance test that:

- renders a known scene into a dedicated target;
- verifies a non-empty asynchronous GPU readback arrives;
- verifies camera target restoration;
- verifies the committed semantic scene hash is identical before/after capture;
- renders N isolated preview variants and proves the committed hash never changes;
- compares baseline/candidate captures only when revision, frame, scene hash, camera, and fidelity match.

## Next visible milestone

Create one small scene with one stable camera and three proposal variants plus abstention. Produce four aligned off-screen captures, feed them through one common perception adapter, and preserve the multidimensional consequence evidence. Only a separately authorized commit may advance the real scene.
