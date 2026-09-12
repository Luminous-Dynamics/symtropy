# Terrain post-Naga compile report

- Product head: `8172eb7a29997e566b88938b95e12fac0d4c9ded`
- Cargo.lock blob: `62c6eb098b3f6fffdea8a175a0f3afb33423b17b`
- Rust/Cargo: `1.96.0`

## lib

Exit code: `0`

## tests

Exit code: `101`

```text
[1m[92m    Checking[0m symtropy-terrain v0.1.0 (<repo>/crates/domains/symtropy-terrain)
[1m[91merror[E0432][0m[1m: unresolved imports `symtropy_terrain::EarthChunkLatticeCoord`, `symtropy_terrain::TerrainGeometrySnapshot`, `symtropy_terrain::TerrainVoxelBox`, `symtropy_terrain::TerrainVoxelIndex`[0m
 [1m[94m--> [0mcrates/domains/symtropy-terrain/tests/geometry_authority_golden.rs:5:17
  [1m[94m|[0m
[1m[94m5[0m [1m[94m|[0m     EarthChunk, EarthChunkLatticeCoord, SubstrateMaterial, TerrainGeometrySnapshot,
  [1m[94m|[0m                 [1m[91m^^^^^^^^^^^^^^^^^^^^^^[0m                     [1m[91m^^^^^^^^^^^^^^^^^^^^^^^[0m [1m[91mno `TerrainGeometrySnapshot` in the root[0m
  [1m[94m|[0m                 [1m[91m|[0m
  [1m[94m|[0m                 [1m[91mno `EarthChunkLatticeCoord` in the root[0m
[1m[94m6[0m [1m[94m|[0m     TerrainVoxelBox, TerrainVoxelIndex,
  [1m[94m|[0m     [1m[91m^^^^^^^^^^^^^^^[0m  [1m[91m^^^^^^^^^^^^^^^^^[0m [1m[91mno `TerrainVoxelIndex` in the root[0m
  [1m[94m|[0m     [1m[91m|[0m
  [1m[94m|[0m     [1m[91mno `TerrainVoxelBox` in the root[0m

[1mFor more information about this error, try `rustc --explain E0432`.[0m
[1m[91merror[0m: could not compile `symtropy-terrain` (test "geometry_authority_golden") due to 1 previous error
[1m[33mwarning[0m: build failed, waiting for other jobs to finish...

```

## examples

Exit code: `0`

## benches

Exit code: `0`

