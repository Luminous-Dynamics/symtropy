using Test
using Dates
using HDF5
using JSON3
using PRAS
using SHA
using Statistics

const PRAS_ENV_ROOT = normpath(joinpath(@__DIR__, ".."))
const FIXTURE_PATH = joinpath(dirname(PRAS_ENV_ROOT), "v1", "pras-interop-fixtures.json")
const FIXTURE_DOC = JSON3.read(read(FIXTURE_PATH, String))
const RESULT_ROWS = Dict{String,Any}[]
const SAMPLE_COUNT = 16
const BASE_SEED = 20261105

function time_range(start::DateTime, step::T, count::Int) where {T<:Period}
    count > 0 || throw(ArgumentError("time range must contain at least one timestep"))
    start_zoned = ZonedDateTime(start, tz"UTC")
    return start_zoned:step:(start_zoned + (count - 1) * step)
end

function one_region_system(
    load_mw::Vector{Int},
    generation_mw::Vector{Int};
    start::DateTime=DateTime(2020, 1, 1, 0),
    step::T=Hour(1),
    failure_probability::Union{Nothing,Vector{Float64}}=nothing,
    repair_probability::Union{Nothing,Vector{Float64}}=nothing,
) where {T<:Period}
    n = length(load_mw)
    n > 0 || throw(ArgumentError("load profile cannot be empty"))
    length(generation_mw) == n || throw(ArgumentError("generation/load time axes differ"))
    l = Dates.value(step)
    lambda = isnothing(failure_probability) ? zeros(Float64, n) : failure_probability
    mu = isnothing(repair_probability) ? ones(Float64, n) : repair_probability
    length(lambda) == n || throw(ArgumentError("failure probability time axis differs"))
    length(mu) == n || throw(ArgumentError("repair probability time axis differs"))

    generators = Generators{n,l,T,MW}(
        ["G1"],
        ["Thermal"],
        reshape(copy(generation_mw), 1, n),
        reshape(copy(lambda), 1, n),
        reshape(copy(mu), 1, n),
    )
    storages = Storages{n,l,T,MW,MWh}()
    generator_storages = GeneratorStorages{n,l,T,MW,MWh}()
    timestamps = time_range(start, step, n)
    return SystemModel(generators, storages, generator_storages, timestamps, copy(load_mw))
end

function two_region_system(
    load_a_mw::Vector{Int},
    load_b_mw::Vector{Int},
    generation_a_mw::Vector{Int},
    generation_b_mw::Vector{Int};
    interface_capacity_mw::Int=10,
    start::DateTime=DateTime(2020, 1, 1, 0),
    step::T=Hour(1),
) where {T<:Period}
    n = length(load_a_mw)
    n > 0 || throw(ArgumentError("two-region profile cannot be empty"))
    all(length(x) == n for x in (load_b_mw, generation_a_mw, generation_b_mw)) ||
        throw(ArgumentError("two-region time axes differ"))
    l = Dates.value(step)

    load = Matrix{Int}(undef, 2, n)
    load[1, :] .= load_a_mw
    load[2, :] .= load_b_mw
    regions = Regions{n,MW}(["A", "B"], load)

    interfaces = Interfaces{n,MW}(
        [1],
        [2],
        fill(interface_capacity_mw, 1, n),
        fill(interface_capacity_mw, 1, n),
    )

    capacities = Matrix{Int}(undef, 2, n)
    capacities[1, :] .= generation_a_mw
    capacities[2, :] .= generation_b_mw
    generators = Generators{n,l,T,MW}(
        ["GA", "GB"],
        ["Thermal", "Thermal"],
        capacities,
        zeros(Float64, 2, n),
        ones(Float64, 2, n),
    )

    storages = Storages{n,l,T,MW,MWh}()
    generator_storages = GeneratorStorages{n,l,T,MW,MWh}()
    lines = Lines{n,l,T,MW}(
        ["AB"],
        ["Transmission"],
        fill(interface_capacity_mw, 1, n),
        fill(interface_capacity_mw, 1, n),
        zeros(Float64, 1, n),
        ones(Float64, 1, n),
    )
    timestamps = time_range(start, step, n)

    return SystemModel(
        regions,
        interfaces,
        generators,
        [1:1, 2:2],
        storages,
        [1:0, 1:0],
        generator_storages,
        [1:0, 1:0],
        lines,
        [1:1],
        timestamps,
    )
end

function measure(system::SystemModel; samples::Int=SAMPLE_COUNT, seed::Int=BASE_SEED)
    shortfall, = assess(
        system,
        SequentialMonteCarlo(samples=samples, seed=seed, threaded=false),
        ShortfallSamples(),
    )
    eue_metric = EUE(shortfall)
    lole_metric = LOLE(shortfall)
    lold_metric = LOLD(shortfall)
    cvar_metric = CVAR(:energy, shortfall, 0.95)
    return (
        shortfall=shortfall,
        sample_eue=shortfall[],
        eue=val(eue_metric),
        eue_stderr=stderror(eue_metric),
        lole_periods=val(lole_metric),
        lole_stderr=stderror(lole_metric),
        lold_days=val(lold_metric),
        lold_stderr=stderror(lold_metric),
        energy_cvar_95=val(cvar_metric),
        energy_cvar_95_stderr=stderror(cvar_metric),
    )
end

function fixture_case(case_id::String)
    found = findfirst(c -> String(c.case_id) == case_id, FIXTURE_DOC.cases)
    isnothing(found) && error("fixture case not found: $case_id")
    return FIXTURE_DOC.cases[found]
end

function push_result!(case_id::String, fields::Dict{String,Any})
    push!(RESULT_ROWS, merge(Dict("case_id" => case_id), fields))
    return
end

function interval_hours(step::Hour)
    return Float64(Dates.value(step))
end

function interval_hours(step::Minute)
    return Float64(Dates.value(step)) / 60.0
end

@testset "PRAS 0.9.0 interoperability" begin
    @test string(Base.pkgversion(PRAS)) == "0.9.0"
    @test String(FIXTURE_DOC.schema_version) == "symtropy.pras-interop-fixtures.v1"
    @test String(FIXTURE_DOC.claim_class) == "SYNTHETIC_KNOWN_TRUTH"

    @testset "hourly single-region metric mapping" begin
        case = fixture_case("pras-hourly-single-region-metrics")
        system = one_region_system([110, 120, 110, 100], fill(100, 4))
        result = measure(system)
        expected = case.expected

        @test result.sample_eue == fill(Float64(expected.eue_mwh), SAMPLE_COUNT)
        @test result.lole_periods == Float64(expected.lole_event_periods)
        @test result.lold_days == Float64(expected.lold_event_days)
        @test result.eue == Float64(expected.eue_mwh)
        @test result.lole_periods * interval_hours(Hour(1)) == 3.0

        # PRAS 0.9.0 averages sample EUE values strictly above the sample
        # quantile. An all-tied deterministic sample therefore returns zero.
        @test result.energy_cvar_95 == Float64(expected.energy_cvar_95_mwh)
        @test result.energy_cvar_95 == 0.0

        push_result!("pras-hourly-single-region-metrics", Dict(
            "lole_event_periods" => result.lole_periods,
            "lold_event_days" => result.lold_days,
            "lolh_hours" => result.lole_periods,
            "eue_mwh" => result.eue,
            "energy_cvar_95_pras_v0_9_mwh" => result.energy_cvar_95,
            "sample_count" => SAMPLE_COUNT,
        ))
    end

    @testset "quarter-hour event-period versus elapsed-hour semantics" begin
        case = fixture_case("pras-quarter-hour-period-versus-hour-metric")
        system = one_region_system(
            [110, 110, 100],
            fill(100, 3);
            step=Minute(15),
        )
        result = measure(system)

        @test result.lole_periods == Float64(case.expected.lole_event_periods)
        @test result.lold_days == Float64(case.expected.lold_event_days)
        @test result.eue == Float64(case.expected.eue_mwh)
        @test result.lole_periods != 0.5
        @test result.lole_periods * interval_hours(Minute(15)) ==
              Float64(case.expected.lolh_hours)

        push_result!("pras-quarter-hour-period-versus-hour-metric", Dict(
            "lole_event_periods" => result.lole_periods,
            "lold_event_days" => result.lold_days,
            "lolh_hours" => result.lole_periods * interval_hours(Minute(15)),
            "eue_mwh" => result.eue,
        ))
    end

    @testset "midnight-crossing event-day accounting" begin
        case = fixture_case("pras-midnight-crossing-event-days")
        system = one_region_system(
            [110, 110, 100],
            fill(100, 3);
            start=DateTime(2020, 1, 1, 23),
            step=Hour(1),
        )
        result = measure(system)

        @test result.lole_periods == Float64(case.expected.lole_event_periods)
        @test result.lold_days == Float64(case.expected.lold_event_days)
        @test result.eue == Float64(case.expected.eue_mwh)
        @test result.lole_periods == 2.0
        @test result.lold_days == 2.0

        push_result!("pras-midnight-crossing-event-days", Dict(
            "lole_event_periods" => result.lole_periods,
            "lold_event_days" => result.lold_days,
            "lolh_hours" => result.lole_periods,
            "eue_mwh" => result.eue,
        ))
    end

    @testset "signed workload trajectory, rebound, and EUE capacity sweep" begin
        case = fixture_case("pras-paired-signed-workload-trajectory-elcc")
        baseline = one_region_system([110, 80, 100], fill(100, 3))
        flexible_load = Int.(case.expected.flexible_load_mw_by_interval)
        flexible = one_region_system(flexible_load, fill(100, 3))
        baseline_result = measure(baseline)
        flexible_result = measure(flexible)

        @test baseline_result.eue == Float64(case.expected.baseline_eue_mwh)
        @test flexible_result.eue == Float64(case.expected.flexible_eue_mwh)

        # PRAS ELCC adds a constant quantity to every study interval. This
        # integer sweep is a deterministic oracle for the pinned EUE criterion.
        at_five = one_region_system(flexible_load .+ 5, fill(100, 3))
        at_six = one_region_system(flexible_load .+ 6, fill(100, 3))
        eue_at_five = measure(at_five).eue
        eue_at_six = measure(at_six).eue

        @test eue_at_five == baseline_result.eue
        @test eue_at_six > baseline_result.eue
        @test eue_at_five == 10.0
        @test eue_at_six == 12.0

        push_result!("pras-paired-signed-workload-trajectory-elcc", Dict(
            "baseline_eue_mwh" => baseline_result.eue,
            "flexible_eue_mwh" => flexible_result.eue,
            "eue_after_plus_5mw_mwh" => eue_at_five,
            "eue_after_plus_6mw_mwh" => eue_at_six,
            "known_truth_eue_elcc_mw" => 5,
            "method" => "deterministic_integer_capacity_sweep_using_PRAS_EUE",
        ))
    end

    @testset "full-horizon rebound trap" begin
        case = fixture_case("pras-rebound-worsens-shortfall")
        baseline = one_region_system([100, 100], [90, 80])
        flexible_load = Int.(case.expected.flexible_load_mw_by_interval)
        flexible = one_region_system(flexible_load, [90, 80])
        baseline_result = measure(baseline)
        flexible_result = measure(flexible)

        @test baseline_result.eue == Float64(case.expected.baseline_eue_mwh)
        @test flexible_result.eue == Float64(case.expected.flexible_eue_mwh)
        @test flexible_result.eue > baseline_result.eue

        push_result!("pras-rebound-worsens-shortfall", Dict(
            "baseline_eue_mwh" => baseline_result.eue,
            "flexible_eue_mwh" => flexible_result.eue,
            "net_eue_change_mwh" => flexible_result.eue - baseline_result.eue,
        ))
    end

    @testset "two-region transfer-limited location dependence" begin
        case = fixture_case("pras-two-region-location-dependence")
        generation_a = [110]
        generation_b = [100]
        baseline = two_region_system([90], [120], generation_a, generation_b)
        relief_at_b = two_region_system([90], [115], generation_a, generation_b)
        relief_at_a = two_region_system([85], [120], generation_a, generation_b)

        baseline_result = measure(baseline)
        result_b = measure(relief_at_b)
        result_a = measure(relief_at_a)

        @test baseline_result.eue == Float64(case.expected.baseline_eue_mwh)
        @test result_b.eue == Float64(case.expected.eue_after_5mw_relief_in_B_mwh)
        @test result_a.eue == Float64(case.expected.eue_after_5mw_relief_in_A_mwh)

        push_result!("pras-two-region-location-dependence", Dict(
            "baseline_eue_mwh" => baseline_result.eue,
            "eue_after_5mw_relief_in_B_mwh" => result_b.eue,
            "eue_after_5mw_relief_in_A_mwh" => result_a.eue,
        ))
    end

    @testset "non-degenerate sample-level energy-CVAR contract" begin
        system = one_region_system(
            fill(110, 8),
            fill(100, 8);
            failure_probability=fill(0.35, 8),
            repair_probability=fill(0.55, 8),
        )
        result = measure(system; samples=256, seed=BASE_SEED + 1)
        sample_eue = result.sample_eue
        threshold = quantile(sample_eue, 0.95)
        strict_tail = sample_eue[sample_eue .> threshold]
        expected_pras_cvar = isempty(strict_tail) ? 0.0 : mean(strict_tail)

        @test length(unique(sample_eue)) > 1
        @test result.energy_cvar_95 == expected_pras_cvar

        push_result!("pras-nondegenerate-energy-cvar-sample-semantics", Dict(
            "sample_count" => length(sample_eue),
            "sample_eue_mwh" => sample_eue,
            "empirical_quantile_95_mwh" => threshold,
            "strictly_above_quantile_sample_count" => length(strict_tail),
            "pras_energy_cvar_95_mwh" => result.energy_cvar_95,
            "convention" => "mean of sample EUE values strictly greater than empirical 95th percentile",
        ))
    end

    @testset "native HDF5 SystemModel round-trip and time-region orientation" begin
        system = two_region_system(
            [91, 93, 95],
            [121, 123, 125],
            [110, 110, 110],
            [100, 100, 100],
        )
        tempdir = mktempdir()
        model_path = joinpath(tempdir, "pras-interop-roundtrip.pras")
        try
            savemodel(system, model_path)
            attrs = h5open(model_path, "r") do file
                root = attributes(file)
                Dict(
                    "pras_dataversion" => String(read(root["pras_dataversion"])),
                    "timestep_count" => Int(read(root["timestep_count"])),
                    "timestep_length" => Int(read(root["timestep_length"])),
                    "timestep_unit" => String(read(root["timestep_unit"])),
                    "power_unit" => String(read(root["power_unit"])),
                    "energy_unit" => String(read(root["energy_unit"])),
                    "region_load" => read(file["regions/load"]),
                )
            end
            loaded = SystemModel(model_path)

            @test loaded == system
            @test attrs["pras_dataversion"] == "v0.9.0"
            @test attrs["timestep_count"] == 3
            @test attrs["timestep_length"] == 1
            @test attrs["timestep_unit"] == "h"
            @test attrs["power_unit"] == "MW"
            @test attrs["energy_unit"] == "MWh"
            @test attrs["region_load"] == [91 93 95; 121 123 125]

            push_result!("native-pras-hdf5-round-trip", Dict(
                "pras_dataversion" => attrs["pras_dataversion"],
                "timestep_count" => attrs["timestep_count"],
                "time_region_matrix_round_trip" => true,
                "model_file_sha256" => bytes2hex(sha256(read(model_path))),
            ))
        finally
            rm(tempdir; recursive=true, force=true)
        end
    end
end

manifest_path = joinpath(PRAS_ENV_ROOT, "Manifest.toml")
project_path = joinpath(PRAS_ENV_ROOT, "Project.toml")
runner_path = joinpath(PRAS_ENV_ROOT, "run_pras_interop.jl")
receipt = Dict(
    "schema_version" => "symtropy.pras-interop-execution-receipt.v1",
    "claim_class" => "EXECUTED_REFERENCE_HARNESS",
    "pras_version" => string(Base.pkgversion(PRAS)),
    "julia_version" => string(VERSION),
    "project_sha256" => bytes2hex(sha256(read(project_path))),
    "manifest_sha256" => isfile(manifest_path) ? bytes2hex(sha256(read(manifest_path))) : nothing,
    "fixture_sha256" => bytes2hex(sha256(read(FIXTURE_PATH))),
    "runner_sha256" => bytes2hex(sha256(read(runner_path))),
    "seed_base" => BASE_SEED,
    "case_results" => RESULT_ROWS,
    "claim_boundary" => "PRAS 0.9.0 reference harness; synthetic fixtures only; no NTCSA reproduction, site qualification, market accreditation, or field claim.",
)

receipt_path = get(
    ENV,
    "PRAS_INTEROP_RESULT_PATH",
    joinpath(PRAS_ENV_ROOT, ".artifacts", "pras-interop-results.json"),
)
mkpath(dirname(receipt_path))
open(receipt_path, "w") do io
    JSON3.write(io, receipt)
end
println("PRAS interoperability receipt: ", receipt_path)
