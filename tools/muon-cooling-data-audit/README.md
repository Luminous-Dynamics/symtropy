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


## Published BDSIM cooling references — distinct from the pinned WG4 skeleton

The pinned WG4 102-coil BDSIM file is a solenoid-sheet model with no instantiated absorbers or RF cavities. It is not the complete BDSIM cooling model and is not admissible for ionization-cooling or RF-acceleration claims.

A separate public BDSIM implementation exists:

- Repository: bdsim-collaboration/bdsim.
- Exact source commit: d18134084b3960ba07bea81a7e2772114062f8b3 (14 February 2025).
- Example: examples/muoncooling/muoncooler.gmad; Git blob SHA-1 a71dbf70605d3dbed9b27fab43436a77a8171d6d; 2,489 bytes.
- The example instantiates a 5 m muoncooler component with six coils, one dipole, two cylindrical lithium-hydride absorbers and twelve RF cavities, for a 200 MeV/c mu-plus beam. It is a component-level example, not the full published channel. BDSIM is GPL-3.0; the source is referenced, not copied into this repository.

Two published studies provide stronger reproduction targets:

**MuCol D4.1 (25 February 2025):** the BDSIM report describes an 80 m rectilinear 6D cooling lattice. Its plotted transverse emittance decreases from approximately 2.5 mm to 1.45 mm; longitudinal emittance decreases from approximately 1.8 eV·s to 1.15 eV·s. The report identifies longitudinal mismatch, momentum oscillations and tail losses as unresolved issues, and describes component-level comparison with analytical and other code calculations. [Deliverable D4.1](https://indico.cern.ch/event/1517584/attachments/3020990/5329912/MuCol_D4_1.pdf).

**IPAC'26 MOP1048 (20 July 2026):** a separate 100 m BDSIM cooling study reports approximately 2.3× reduction in 6D emittance and approximately 93% transmission for a 200 MeV/c beam. Muon decays were disabled. The paper reports an amplitude–momentum correlation in the input distribution and identifies input matching and tolerance studies as follow-up work. [IPAC'26 paper](https://www.ipac26.org/prepress/pdf/MOP1048.pdf).

These are distinct evidence identities: the pinned WG4 102-coil skeleton, the 5 m BDSIM component example, the D4.1 80 m simulation, the IPAC'26 100 m simulation, and the separate G4Beamline 60-cell run. The exact identity of the IPAC'26 input distribution with the currently pinned 10,001-row WG4 file has not been verified. Do not claim reproduction or cross-code agreement from the source audit.
