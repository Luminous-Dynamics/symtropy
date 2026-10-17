# Thermodynamic allocation certificate verifier v0.1

This standalone, dependency-free Rust tool checks a proposed well-to-agent
allocation using integer quanta. It is intentionally separate from both the
production allocator and the finite-grid search oracle.

## Paradigm shift: solver and checker are different roles

A solver proposes an allocation. A small independent checker recomputes the
accounting from the edge flows rather than trusting the solver's totals. This
separates three questions that are too often conflated:

1. **Feasibility:** are supply, shared recipient caps and eligibility respected?
2. **Accounting:** do reported source debits, recipient credits and service profile
   match what the edge flows imply, with exact conservation?
3. **Optimality:** is this allocation actually leximin-optimal for the chosen
   objective?

This checker answers the first two. It deliberately never proves the third.
If a certificate sets CLAIMS_OPTIMALITY to true, the report emits a warning and
keeps optimality_proven=false. The finite-grid oracle or a separately checked
mathematical proof must establish optimality.

## Run

From the repository root:

    cargo fmt --manifest-path tools/thermo-allocation-certificate/Cargo.toml -- --check
    cargo test --manifest-path tools/thermo-allocation-certificate/Cargo.toml
    cargo run --manifest-path tools/thermo-allocation-certificate/Cargo.toml -- --verify tools/thermo-allocation-certificate/fixtures/overlapping-wells.tsv

The file format is tab-separated UTF-8. Blank lines and lines whose first
non-whitespace character is # are ignored. The first non-comment line must be:

    THERMO-ALLOCATION-CERT<TAB>1

Allowed rows, with exact field counts:

| Row | Fields after row name | Meaning |
|---|---|---|
| CASE | case_id | Stable scenario label |
| OBJECTIVE | absolute or capacity-normalized | Service profile definition |
| CLAIMS_OPTIMALITY | true or false | An explicit claim, not proof |
| WELL | id, supply | One source and its budget |
| AGENT | id, cap | One recipient and its shared cap |
| ELIGIBLE | well_id, agent_id | One allowed edge |
| FLOW | well_id, agent_id, amount | Proposed flow along an edge |
| SOURCE_CLAIM | well_id, debit | Claimed aggregate debit per source |
| CREDIT_CLAIM | agent_id, credit | Claimed aggregate credit per recipient |
| PROFILE_ABSOLUTE | amount | One sorted absolute-service profile element |
| PROFILE_RATIO | numerator, denominator | One reduced exact-rational profile element |

All identifiers and quantities are unsigned 64-bit integers. Values use a
single integer quantum chosen by the scenario author. Do not silently mix unit
systems in one certificate. A ratio must have a positive denominator; normalized
profile elements are reduced by greatest common divisor and sorted by exact
cross-multiplication, not floating-point division.

The checker fails closed on duplicate entity identifiers, duplicate edge rows,
unknown references, ineligible nonzero flows, source overspend, cap violation,
overflow, conservation discrepancies, missing/mismatched accounting claims and
a reported profile that differs from the recomputed profile. The CLI exits 0 for
a feasible certificate, 1 for an infeasible/invalid certificate, and 2 for
file/format errors. The focused workflow checks both the valid overlap fixture
and a separate negative-control file that must exit with code 1 and report
recipient_cap_violation.

## Exact overlap fixture

The fixture uses wells with supply 12 each and recipients with caps 9, 12, 12.
The edge flows are:

- well 10 -> agent 1: 4; well 10 -> agent 2: 8
- well 20 -> agent 1: 4; well 20 -> agent 3: 8

The exact accounting is debit 24, credit 24, and totals [8, 8, 8]. Agent 1's
shared cap is applied once to its combined receipt from both wells. The example
does not itself establish that an arbitrary production policy is fair or
optimal; it demonstrates what a checkable feasible decision record looks like.

## Benchmarks and acceptance gates

- Zero false accepts across targeted mutations of source supply, shared cap,
  eligibility, duplicate edge, unknown ID, claimed debit/credit and service profile.
- Zero accounting differences on valid certificates; all calculations are exact
  integer arithmetic, with exact rational comparisons for normalized ratios.
- Identical canonical reports when semantically identical inputs are permuted.
- Mutation score >=95% for targeted feasibility and accounting mutations, with
  surviving mutants enumerated.
- Measure checker time, peak memory, and throughput for 10, 100, 1,000, 10,000
  and 100,000 allocation edges. Record machine, OS, toolchain, input SHA and
  scenario seed. Do not gate CI on brittle wall-clock thresholds.
- Later extension: bind certificate to a canonical problem digest and add a
  proof/certificate format for optimality (for example, an independently
  checked exhaustive witness for tiny grids or a valid mathematical certificate).

These are research targets, not measured claims. Passing this checker validates
only the encoded certificate against its encoded problem. It does not validate
whether the problem model captured the intended game semantics.

## Related work

The need for a joint graph model rather than independent single-source
water-filling is consistent with Namyar et al., *Solving Max-Min Fair Resource
Allocations Quickly on Large Graphs*, NSDI 2024. Their work develops scalable
max-min fair resource allocation methods and explicit speed/quality trade-offs:
https://www.usenix.org/conference/nsdi24/presentation/namyar-solving

This local checker is complementary: it is a verification boundary for a given
answer, not a new allocation algorithm and not a claim to surpass the paper's
methods.
