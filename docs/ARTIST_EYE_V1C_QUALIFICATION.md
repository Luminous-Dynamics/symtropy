# ARTIST-EYE-v1C Qualification Contract

ARTIST-EYE-v1C is not qualified by construction. A PASS requires both static
Rust gates and empirical render evidence.

## 1. Rust gates

Run in the project Nix development shell:

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings

cargo check -p symthaea-bevy-brain --features realtime-art-render --all-targets
cargo test -p symthaea-bevy-brain --features realtime-art-render
cargo clippy -p symthaea-bevy-brain --features realtime-art-render --all-targets -- -D warnings
```

Do not weaken warnings, remove tests, or change prospective empirical
thresholds in the same qualification lineage merely to obtain PASS.

## 2. Depth adapter static invariants

The reviewed implementation must continue to enforce:

- capture request declares `Depth`;
- standard perspective and orthographic projections are explicit;
- custom projections fail closed;
- perspective reverse-Z zero remains missing/infinite;
- depth source carries `COPY_SRC` only for the armed capture lifecycle;
- destination is a dedicated `Depth32Float` image;
- `Msaa::Off` is required for v1C depth evidence;
- completion queue is bounded and rejects rather than silently evicts;
- readback byte/row alignment is validated;
- linearized samples enter v1B as `LinearMeters` only after reconstruction.

## 3. VART-DEPTH-LIVE-001

Before looking at confirmatory outcomes, freeze:

- scene seed and exact geometry;
- camera identity and transform;
- projection type and near/far/culling parameters;
- resolution;
- GPU/driver/backend;
- absolute and relative metric-depth tolerances;
- expected background missingness behavior;
- trial count / independent run count.

Recommended analytic targets include planes centered at 0.5 m, 1 m, 2 m,
5 m, 10 m and 25 m when compatible with the frozen near/culling range.

For every expected capture record:

- capture ID;
- revision/frame/scene hash;
- camera stable ID;
- render epoch;
- raw artifact digest;
- projection provenance;
- reconstructed depth summary;
- expected versus observed metric error.

A missing capture, queue drop, projection mismatch, MSAA violation, or identity
mismatch invalidates the affected confirmatory unit.

## 4. Color/depth synchronization gate

For a synchronized artistic observation, color and depth must refer to the same:

```text
revision
studio frame
semantic scene hash
stable camera
resolution
render epoch / host render opportunity
```

The depth plane may have a distinct capture ID and artifact digest. It may not
silently inherit identity from a different color frame.

## 5. VART-TEMP-001

Freeze a maximum allowed frame gap and exact intervention schedule before the
confirmatory sequence.

Required controls:

1. static camera / static scene;
2. camera-only translation;
3. camera-only rotation;
4. scene-form motion with camera fixed;
5. occluder enters;
6. occluder leaves;
7. focal-region migration;
8. missing-frame negative control.

Required invariants:

- frame order strictly increases;
- cross-camera windows are rejected;
- gaps above the frozen bound are rejected;
- depth availability may not appear/disappear inside a transition;
- camera-pose evidence may not appear/disappear inside a transition;
- spatial pyramid levels must align;
- focal migration and camera motion remain separate evidence channels;
- descriptive rhythm retains independent dimensions.

## 6. Claim boundary

A v1C PASS supports:

> Symthaea's art studio can acquire qualified linear depth from Bevy and can
> measure revision-bound spatial/depth/camera changes across bounded temporal
> windows.

It does **not** by itself support:

- aesthetic competence;
- subjective visual experience;
- object-level motion attribution;
- camera-motion compensation;
- artistic value optimization;
- active mutation authority.

Those require separate evidence and gates.
