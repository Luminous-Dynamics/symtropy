# SU2 residual and monitor screening v0.1

## Purpose

\`su2_convergence_screen\` is a post-import threshold screen for selected SU2 history outputs. It runs only after \`su2_history_import\` has verified a successful \`SU2_CFD\` process receipt and the exact history output bytes. The screen's input type is then checked against the canonical import digest before any criterion is evaluated.

This is intentionally not advertised as a universal convergence detector. A passing result means only that every declared criterion over every declared channel met its caller-supplied numerical threshold. It does not demonstrate:
- all relevant governing-equation residuals were monitored;
- mass, momentum, or energy conservation;
- force/moment stationarity outside the selected finite window;
- spatial or temporal discretization convergence;
- turbulence-model adequacy, model-form credibility, or agreement with experiment;
- physical validity, flight suitability, or certification.

The evidence ceiling is:

\`declared_residual_and_monitor_threshold_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation\`

## CLI

The CLI imports the output from the receipt and evaluates explicit thresholds in one invocation:

\`\`\`sh
cargo run --release --locked \\
  --manifest-path tools/civil-aero-benchmark/Cargo.toml \\
  --bin su2_convergence_screen -- \\
  --run-dir ./artifacts/dpw8-v1a-periodic-l1-a136 \\
  --history history.csv \\
  --iteration-column Iteration \\
  --header-column Iteration \\
  --header-column CL \\
  --header-column CD \\
  --header-column RMS_RES \\
  --field CL=dimensionless \\
  --field CD=dimensionless \\
  --field RMS_RES=1 \\
  --residual RMS_RES=1e-8:1e-6 \\
  --monitor CL=500:1e-4 \\
  --monitor CD=500:1e-5 \\
  > ./artifacts/dpw8-v1a-periodic-l1-a136/convergence-screen.csv
\`\`\`

Those limits are illustrative syntax only, not recommended or pre-approved DPW8 convergence thresholds. Select thresholds from the chosen solver formulation, convergence protocol, observable sensitivity and numerical/engineering requirements. Do not copy thresholds across turbulence models, solvers, boundary conditions or unsteady regimes without justification.

\`--residual\` is repeatable and takes \`COLUMN=MAX_FINAL_ABS:MAX_FINAL_TO_INITIAL_RATIO\`. The column must be imported, the limits must be finite and non-negative, and the permitted reduction ratio is in [0,1]. The ratio is evaluated using absolute residual values at the first and last recorded rows.

\`--monitor\` is repeatable and takes \`COLUMN=WINDOW_ROWS:MAX_ABS_RANGE\`. The selected trailing window must contain at least two observations; fewer available rows produce \`insufficient_rows\` rather than a pass. The reported monitor range is max minus min over that trailing window.

Exit codes: 0 means all declared criteria met; 3 means one or more declared criteria failed or were incomplete; 2 means the run/history/criteria could not be audited. The CSV is emitted for both passing and non-passing screens (provided the audit itself could be performed).

## Evidence composition

The screen digest is a canonical hash over the run ID, runner receipt digest, history digest, import digest, overall status, every declared column/unit, each observed statistic, each threshold and each criterion status. The independent CI oracle reconstructs this digest and the arithmetic from the raw fixture history.

A screen must be followed by separate conservation checks, solver-specific stopping-condition review, GCI/grid-family assessment, validation uncertainty and domain-specific physical evidence. For unsteady buffet or dynamic cases, a finite trailing-range statistic is not a substitute for spectral/stationarity analysis or temporal refinement.

The CI fixture is synthetic and tests only process-to-history-to-screen data lineage and arithmetic. It is not SU2 output, an aerodynamic result, or evidence that any particular threshold is suitable for DPW8.
