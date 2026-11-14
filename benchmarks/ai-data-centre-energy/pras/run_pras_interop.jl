# Run the PRAS 0.9.0 interoperability known-truth suite.
#
# Usage:
#   julia --project=benchmarks/ai-data-centre-energy/pras benchmarks/ai-data-centre-energy/pras/run_pras_interop.jl
#
# This runner is an optional Julia benchmark harness, not a Symtropy
# production-runtime dependency.

include(joinpath(@__DIR__, "test", "runtests.jl"))
