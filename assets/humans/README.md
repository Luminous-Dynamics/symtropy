# Optional human avatar smoke-test asset

The human avatar example does not download assets automatically and no third-party binary is committed here.

## Candidate smoke fixture

The Innerscene [Maya 3D Person — Locomotion Collection](https://www.innerscene.com/tools/library/3d-parts/locomotion-collection-maya-3d-person-animated-1499b2fd) page currently lists a 20.3 MB GLB under CC0 / public-domain terms, based on MakeHuman/MPFB CC0 assets. The listing describes named idle, relaxed/brisk walk, jog, run, carrying and sit/stand animations. Its publisher explicitly says these are authored periodic-pose recipes, not motion-capture recordings.

Re-check the [asset page](https://www.innerscene.com/tools/library/3d-parts/locomotion-collection-maya-3d-person-animated-1499b2fd) and [library terms](https://www.innerscene.com/tools/library/terms) at the time of download. This note records the current listing but does not replace the publisher's terms or a provenance review.

1. Download the GLB using the page's **Download GLB** link.
2. Save it as `assets/humans/person.glb`.
3. Record the downloaded file hash and source/terms snapshot:

   ```sh
   sha256sum assets/humans/person.glb
   ```

4. Run from the repository root:

   ```sh
   cargo run -p symtropy-bevy --features human-animation --example human_avatar
   ```

   Alternatively, set `SYMTROPY_HUMAN_AVATAR` to a different asset-relative path under `assets/`.

5. Capture a screenshot and record the commit, asset SHA-256, observed clip names, hardware, resolution, and P50/P95 frame times before treating the smoke run as evidence.

Use this asset only to test loading, skinning, material rendering, clip discovery, looping and transitions. Do not use its authored loops as mocap ground truth, a behavioral realism benchmark, or a training dataset with assumed rights beyond the published license. The JSON trace fixture under `docs/research/fixtures/` is synthetic and separate from this asset.
