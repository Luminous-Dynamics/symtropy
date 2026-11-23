# Orbital GSD k-Coverage Voxel Visibility Bench

Status: synthetic P0/P1 3D voxel-visibility prototype.
Parent issue: Symtropy #6845.
Dedicated reproduction: Symtropy #7002.
Claim ceiling: synthetic 3D voxel-surface candidate-view visibility only.

This crate independently implements a bounded subset of the 2026 GSD-constrained viewpoint-generation method:

- 3D occupancy grid and padded exterior-space flood fill;
- exterior face-derived surface normals, retaining multiple normals when opposing faces cancel;
- GSD-derived slant range and perpendicular camera distance;
- candidate viewpoints from each surface patch;
- voxel-centre coverage using range, surface-normal deviation, conic FOV and conservative voxel-grid ray occlusion;
- candidate-view and per-feature SBW receipts;
- fixed quantization for deterministic receipts and SHA-256 input binding.

It does not yet implement tilt/azimuth candidate augmentation, overlap pruning, the paper's globally informed greedy selector, ILP/MIP, physical camera modelling, route planning or reconstruction-quality metrics. Horizontal FOV and pixel width are required input fields; the implementation never guesses them. Fixture camera intrinsics are deliberately synthetic and are not asserted to be the paper's camera.

Run:

    cargo test --locked --manifest-path tools/orbital-gsd-kcoverage-bench/Cargo.toml
    cargo run --quiet --locked --manifest-path tools/orbital-gsd-kcoverage-bench/Cargo.toml --bin orbital-gsd-kcoverage-bench -- docs/research/orbital-gsd-kcoverage/fixtures/voxel-visibility-v0_1.json <40-hex-source-commit>
    python3 tools/orbital-gsd-kcoverage-bench/tests/reference_oracle.py docs/research/orbital-gsd-kcoverage/fixtures/voxel-visibility-v0_1.json <40-hex-source-commit>

The standard-library Python oracle independently reconstructs occupancy flood-fill, face normals, candidate views, FOV/GSD filters, voxel ray tests and receipt fields.

Paper: Bin Khalid and Yeum, arXiv:2610.36450 (27 October 2026). The paper evaluates a voxel-centre approximation and does not establish whole-surface sensor detection or route/flight feasibility. This crate is a synthetic algorithm prototype, not a source-faithful reproduction of their Angel/Rooster parameter sweep.


## Source admission: paper parameters are not camera intrinsics

The companion source-admission manifest is at `docs/research/orbital-gsd-kcoverage/source-admission-v0_1.json`. The paper provides its GSD equation, maximum obliquity and derived range, but the current admitted source record does not yet pin both horizontal FOV and horizontal pixel width, nor the exact mapping of its two rendered models to a versioned dataset file. The synthetic fixture therefore carries explicit test-only FOV/pixel values. A synthetic workflow PASS is not a source-faithful reproduction.

The upstream example repository is GPL-3.0; the linked Mendeley datasets separately state CC BY 4.0. This crate contains an independent implementation and no copied upstream code. Dataset file mapping and SHA-256 digests remain admission blockers until verified.


## Source-admissible tangent-frame augmentation

The candidate augmenter constructs deterministic local tangent frames for initially under-covered surface patches and explores the declared tilt/azimuth set. A requested angle is not sufficient to admit a candidate: the source patch must remain geometrically observable. Each generated direction is rejected when its angle to the source patch's outward normal exceeds `max_oblique_angle_rad`; candidates are also rejected if their source ray is occluded or the source patch does not survive the complete distance/FOV/GSD/occlusion visibility test.

The receipt accounts for every declared augmentation angle pair:

- `augmentation_candidate_attempt_count`
- `augmented_candidate_view_count`
- `augmentation_rejected_max_oblique_count`
- `augmentation_rejected_source_occlusion_count`
- `augmentation_rejected_source_visibility_count`

The sum of accepted plus rejected candidates must equal attempted candidates. The independent Python oracle reconstructs these counters and the workflow asserts that every accepted candidate sees its source feature and respects the obliquity limit. This prevents a “viewpoint for feature X” from entering the candidate pool when it cannot actually inspect X.
