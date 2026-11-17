# Muon Cooling Demonstrator Data Audit

This Rust CLI audits the publicly versioned Muon Collider WG4 BDSIM userfile without pretending to run BDSIM or G4Beamline.

## Pinned upstream source

- Repository: `MuonCollider-WG4/cooling_demonstrator`
- Ref: `2026-11-09-prerelease`
- Commit: `d453b0ce0efd018ffb8d32927ac74cb597a02d26`
- Data: `cooling/bdsim/lowemittancebeam.dat`
- Git blob SHA-1: `9e31ff9e1e65ca31aa082f685a2f5fc0b81a7217`
- Expected byte length: 1,257,234
- Expected records: 10,001

The BDSIM source declares a `mu-` userfile with columns:

`x[mm]:xp[rad]:y[mm]:yp[rad]:z[mm]:Ek[MeV]:t[ns]`

The parser preserves the original native coordinate basis. BDSIM defines xp/yp as angles, not canonical momenta. No normalized emittance, momentum conversion, particle IDs or weights are inferred.

## Run against the exact upstream snapshot

```sh
git clone https://github.com/MuonCollider-WG4/cooling_demonstrator.git
git -C cooling_demonstrator checkout --detach d453b0ce0efd018ffb8d32927ac74cb597a02d26
cargo run --manifest-path tools/muon-cooling-data-audit/Cargo.toml -- \
  --manifest docs/research/muon-collider/wg4-cooling-demonstrator-v0.1.json \
  --input cooling_demonstrator/cooling/bdsim/lowemittancebeam.dat
```

The CLI verifies the raw byte length and Git blob object ID, and reports a local SHA-256, seven-column native statistics, and the 2D RMS covariance determinants in mm·rad.

## Source-model separation

The same source manifest records:
- BDSIM solenoid-sheet configuration: 102 coils, 0 absorbers, 0 RF cavities. That model cannot support ionization-cooling or RF-acceleration claims.
- G4Beamline cooling configuration: a separate 60-cell lattice with a lithium-hydride wedge and 30 MV/m RF. It is not declared equivalent to the BDSIM model.
- A G4Beamline log from a separate run: 10,000 events, 773 s, 51 output regions. Decay is disabled in that run, so muon survival is not evaluated. The report records unresolved runtime override provenance rather than guessing an invocation.

The upstream README itself warns that the BDSIM lattice is early and inconsistent with the G4Beamline lattice. Do not join outputs by similar filenames or call them cross-code comparisons without a geometry/physics equivalence study.

## Tests and claim ceiling

```sh
cargo test --manifest-path tools/muon-cooling-data-audit/Cargo.toml
cargo fmt --manifest-path tools/muon-cooling-data-audit/Cargo.toml -- --check
cargo clippy --manifest-path tools/muon-cooling-data-audit/Cargo.toml --all-targets -- -D warnings
```

Tests cover the seven-column parser, native covariance, byte identity, header/order/units, malformed data, and attempts to promote non-cooling or decay-disabled evidence.

A pass means only that the pinned public input file and its declared native format passed this audit. It is not a BDSIM/G4Beamline run, ionization-cooling result, transmission result or muon-survival qualification.
