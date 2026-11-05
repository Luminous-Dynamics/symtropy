# Optional PRAS 0.9.0 interoperability harness

This directory contains a Julia-only research/benchmark environment. It is not part of the Rust production runtime and does not add Julia as a Symtropy dependency.

## Run

From the repository root:

```sh
julia --project=benchmarks/ai-data-centre-energy/pras -e 'using Pkg; Pkg.instantiate()'
julia --project=benchmarks/ai-data-centre-energy/pras benchmarks/ai-data-centre-energy/pras/run_pras_interop.jl
```

The project environment requires PRAS exactly `0.9.0` and Julia 1.10 compatibility. The first instantiate resolves and writes a Julia `Manifest.toml`; the execution receipt records its digest when present. Commit and preserve that manifest when a qualified runner environment is available so future comparisons can be replayed against the same transitive dependency graph.

The test harness:
- constructs native PRAS single-region hourly and quarter-hour systems;
- checks LOLE event-periods, LOLD event-days and EUE against arithmetic fixtures;
- tests a complete signed workload trajectory, constant-load EUE restoration and a rebound-worsens-adequacy control;
- constructs a two-region line/interface case to check locational value under a transfer constraint;
- tests the exact PRAS 0.9.0 sample-EUE CVAR convention, including tied and non-degenerate distributions;
- saves and reloads a native HDF5 `.pras` model and checks a non-symmetric region-by-time load matrix for orientation errors;
- writes a JSON execution receipt with Julia/PRAS versions, source/fixture/environment digests, sample seed and case outputs.

## Claim boundary

These are synthetic known-truth model tests. They do not reproduce the official NTCSA model, certify the PRAS model's local topology, prove a data-centre site deliverable, or establish market accreditation. Passing this harness only qualifies the adapter's behavior against the selected PRAS 0.9.0 model contract.

This environment is optional and separate from all production Rust crates.
