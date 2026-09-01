# Universal Matter v4.8 × CUF Preflight — Status — 2026-09-01

## State

Replay boundary verified; patch not applied or locally Rust-qualified in the connected authoring environment.

## Branch

`terrain/universal-matter-v4.8-preflight`

## Base

`world/cuf-v0.10-upstream-downstream-proof`

## Verified artifact

`SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE(1).patch`

SHA-256:

`23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637`

Shape:

- 51,420 patch lines;
- 275 file diffs;
- 269 new files;
- 6 modified files;
- 0 deleted files.

All six modified-file Git preimage prefixes match the current CUF v0.10 branch exactly.

## Added safeguards

- fail-closed patch checksum verification;
- exact patch-shape verification;
- clean-working-tree requirement;
- six exact Git preimage checks;
- absence check for all 269 new-file targets;
- `git apply --check` preflight;
- explicit `git apply --index` helper that does not commit;
- six postimage checks after replay;
- exact 275 staged-path check;
- combined Terrain + CUF qualification gate;
- canonical v4.8 × CUF authority/integration contract.

## Not claimed

- v4.8 is not applied on GitHub by this tranche;
- v4.8 Rust tests have not been run here;
- no v4.8 compile/clippy result is asserted;
- no CUF v0.11 adapter is claimed yet.

## Local sequence

1. `bash scripts/preflight-universal-matter-v4.8.sh /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch`
2. `bash scripts/apply-universal-matter-v4.8.sh /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch`
3. `nix develop --command bash scripts/qualify-universal-matter-v4.8-cuf.sh`
4. review the staged diff and qualification evidence;
5. commit only after all gates pass.

## Next milestone after qualification

CUF v0.11: read-only Universal Matter observation adapters that bind native Matter/Hydrology/SurfaceWater/Ecosystem authority identities into CUF evidence, followed by replacing the hand-authored C hydrology fixture in the v0.10 proof with a real Hydrology-owned publication.
