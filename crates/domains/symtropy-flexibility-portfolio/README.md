# Symtropy Flexibility Portfolio Resilience

Empirical joint-delivery evaluation for a frozen portfolio of flexible resources.

## Why this is separate from fleet correlation

The fleet-correlation evaluator measures contemporaneous/lagged dependence in observed power traces. This crate evaluates a different object: whether the enrolled portfolio delivered its contracted lower-bound kW in complete dispatch-event panels under observed shared stress-domain conditions.

A fleet can have low ordinary-load covariance but still fail jointly when shared dependencies (telecom, control plane, utility, cloud, software release, cooling or regional weather) are impaired. Conversely, observing a domain failure does not prove that it caused every member asset's non-delivery.

## V0.1 evidence rules

- Portfolio identity pins product contract and asset/domain manifest digests.
- Assets have unique asset IDs and site/PCC pairs, positive offered capacity and declared failure-domain membership.
- Every event contains exactly one outcome for each enrolled asset and exactly one explicit state for each declared failure domain.
- Missing asset or domain observations fail closed. An explicit `Unknown` is retained as unknown; it is not silently mapped to nominal availability.
- Delivered lower-bound kW comes from a separately qualified response/metrology receipt. This crate does not re-measure PCC power or certify each site.
- Unavailable/unknown assets cannot claim nonzero delivered lower-bound capacity; delivered lower bound cannot exceed the asset's offered capacity.
- Portfolio delivery is summed at the portfolio boundary and compared with the event's required kW.
- Repeated events with one incident-group ID count as one incident group for group pass/fail. The group's worst event margin governs.
- Results are stratified by scenario and observed failure-domain state.
- Domain memberships may overlap. Per-domain member capacity is reported independently; domain shares are never summed. The largest single-domain offer share is reported.

## Metrics

- Event pass fraction and incident-group pass fraction.
- Worst incident-group margin and maximum event shortfall.
- Empirical upper-tail mean of incident-group maximum shortfalls (the tail fraction and rank convention are fixed in the report; descriptive only).
- Scenario-specific incident-group pass fractions.
- Per-domain state counts and portfolio outcomes conditional on each observed state.
- Member offered/delivered sums and largest shared-domain capacity share.

## Important assumptions

- Incident-group IDs must represent independent stress episodes. The evaluator cannot prove independence.
- Failure-domain membership and state are descriptive; no causal attribution is inferred.
- This is empirical evidence from the supplied panel, not a future failure probability. Confidence intervals and statistical generalization are future work.
- A complete event panel is required. Missing telemetry or missing site evidence is an error, not a zero delivered value.
- This is not network/nodal deliverability, PRAS/ELCC, utility acceptance, market accreditation, cybersecurity certification or IEEE P4200 conformity.

## Run

```sh
cargo test -p symtropy-flexibility-portfolio
cargo clippy -p symtropy-flexibility-portfolio --all-targets -- -D warnings
```

All current fixtures are synthetic known truth.

## Receipt/provenance preservation

Each event receipt carries one nested receipt per enrolled asset: asset/site/PCC identity, offered capacity, delivered lower-bound capacity, explicit availability state, upstream response receipt ID and source digest. It also carries every failure-domain state with the corresponding event evidence ID and source digest. The report is therefore replayable back to the evidence identifiers used for each per-site delivery and each observed shared-domain condition. The digests and IDs are references supplied by the upstream evidence owner; this crate does not authenticate the underlying bytes.

## Incident-group tail shortfall

For every independent incident group, the evaluator selects the worst shortfall among that group's dispatch episodes. These group-maximum shortfalls are sorted from largest to smallest. The empirical upper-tail mean uses the largest `ceil(N × alpha)` group shortfalls, where `N` is the number of independent incident groups and `alpha` is the contract's basis-point tail fraction. It is a descriptive stress-panel statistic, not a future CVaR probability estimate or a substitute for a validated statistical confidence bound.

## Overlapping failure domains

Failure-domain memberships can overlap (for example, a site may share both a utility region and a telecom carrier with other assets). The largest single-domain offered-capacity share is reported, and every event retains its domain membership/state receipts. Individual domain member-capacity totals are descriptive and may overlap; they are never summed into a second portfolio capacity total. V0.1 intentionally does not emit a Herfindahl index across overlapping domains because that would require an explicit, defensible allocation of each asset's capacity among its memberships.

The v0.1 implementation is intentionally an empirical evidence-panel evaluator; statistical extrapolation and scenario generation remain separate future work.
