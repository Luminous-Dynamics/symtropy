(* BHPT Teukolsky to Symtropy radial fixture exporter.
   This is a reference-data adapter, not production Symtropy code. *)

ClearAll[BHPTComplexObject, BHPTExportTeukolskyRadialFixture];

BHPTComplexObject[z_?NumericQ, digits_Integer] := <|
  "re" -> N[Re[z], digits],
  "im" -> N[Im[z], digits]
|>;

BHPTExportTeukolskyRadialFixture[config_Association] := Module[
  {
    required, missing, s, ell, m, a, omega, massKg, bits, digits,
    nodes, branchId, fourier, engineVersion, sourceRevision, sourceId,
    horizon, radial, inFunction, upFunction, lambda, samples,
    inValue, inDerivative, upValue, upDerivative, digest, timestamp,
    fixture, outputPath
  },

  required = {
    "FixtureId", "OutputPath", "MassKg", "SpinWeight", "Ell", "M",
    "SpinOverMass", "OmegaM", "PrecisionBits", "RadialNodesOverM",
    "AngularBranchId", "FourierConvention", "TeukolskyVersion"
  };
  missing = Select[required, !KeyExistsQ[config, #] &];
  If[missing =!= {}, Return[Failure["MissingConfiguration", <|"Keys" -> missing|>]]];

  s = config["SpinWeight"];
  ell = config["Ell"];
  m = config["M"];
  a = config["SpinOverMass"];
  omega = config["OmegaM"];
  massKg = config["MassKg"];
  bits = config["PrecisionBits"];
  digits = Max[16, Ceiling[bits/Log[2, 10]]];
  nodes = config["RadialNodesOverM"];
  branchId = config["AngularBranchId"];
  fourier = config["FourierConvention"];
  engineVersion = config["TeukolskyVersion"];
  sourceRevision = Lookup[config, "SourceRevision", Null];
  sourceId = Lookup[config, "SourceId", Null];
  outputPath = config["OutputPath"];

  If[!IntegerQ[s] || !IntegerQ[ell] || !IntegerQ[m] ||
     ell < Abs[s] || Abs[m] > ell,
    Return[Failure["InvalidModeNumbers", <||>]]
  ];
  If[!NumericQ[massKg] || !TrueQ[massKg > 0],
    Return[Failure["InvalidMassKg", <||>]]
  ];
  If[!NumericQ[a] || !TrueQ[Abs[a] < 1],
    Return[Failure["InvalidSpin", <||>]]
  ];
  If[!NumericQ[omega],
    Return[Failure["InvalidFrequency", <||>]]
  ];
  If[!IntegerQ[bits] || bits < 53,
    Return[Failure["InvalidPrecisionBits", <||>]]
  ];
  If[!VectorQ[nodes, NumericQ] || Length[nodes] < 1,
    Return[Failure["InvalidRadialNodes", <||>]]
  ];
  If[!And @@ (TrueQ[#2 > #1] & @@@ Partition[nodes, 2, 1]),
    Return[Failure["RadialNodesNotStrictlyIncreasing", <||>]]
  ];
  If[!StringQ[branchId] || StringTrim[branchId] === "",
    Return[Failure["MissingAngularBranchId", <||>]]
  ];
  If[!MemberQ[{"e_minus_i_omega_t", "e_plus_i_omega_t"}, fourier],
    Return[Failure["UnsupportedFourierConvention", <||>]]
  ];
  If[!StringQ[engineVersion] || StringTrim[engineVersion] === "",
    Return[Failure["MissingTeukolskyVersion", <||>]]
  ];
  If[bits > 53 && (Precision[a] === MachinePrecision || Precision[omega] === MachinePrecision),
    Return[Failure[
      "MachinePrecisionInput",
      <|"Message" -> "Use exact or arbitrary-precision spin/frequency input; lost digits cannot be restored."|>
    ]]
  ];

  a = N[a, digits];
  omega = N[omega, digits];
  nodes = N[nodes, digits];
  horizon = 1 + Sqrt[1 - a^2];
  If[!And @@ (TrueQ[# > horizon] & /@ nodes),
    Return[Failure["RadialNodeAtOrInsideHorizon", <|"OuterHorizonOverM" -> horizon|>]]
  ];

  (* BHPT documents this value as the Teukolsky spheroidal lambda. *)
  lambda = SpinWeightedSpheroidalEigenvalue[s, ell, m, a omega];

  radial = TeukolskyRadial[s, ell, m, a, omega];
  inFunction = radial["In"];
  upFunction = radial["Up"];

  samples = Table[
    inValue = N[inFunction[r], digits];
    inDerivative = N[Derivative[1][inFunction][r], digits];
    upValue = N[upFunction[r], digits];
    upDerivative = N[Derivative[1][upFunction][r], digits];
    If[!And @@ (NumericQ /@ {inValue, inDerivative, upValue, upDerivative}),
      Return[Failure["NonNumericRadialEvaluation", <|"RadiusOverM" -> r|>]]
    ];
    <|
      "r_over_m" -> r,
      "in_value" -> BHPTComplexObject[inValue, digits],
      "in_derivative" -> BHPTComplexObject[inDerivative, digits],
      "up_value" -> BHPTComplexObject[upValue, digits],
      "up_derivative" -> BHPTComplexObject[upDerivative, digits]
    |>,
    {r, nodes}
  ];

  digest = If[
    StringQ[$InputFileName] && FileExistsQ[$InputFileName],
    IntegerString[FileHash[$InputFileName, "SHA256"], 16, 64],
    Null
  ];
  timestamp = DateString[
    Now,
    {"Year", "-", "Month", "-", "Day", "T", "Hour", ":", "Minute", ":", "Second"},
    TimeZone -> 0
  ] <> "Z";

  fixture = <|
    "schema_version" -> 1,
    "fixture_id" -> config["FixtureId"],
    "provenance" -> <|
      "engine" -> "Black Hole Perturbation Toolkit Teukolsky",
      "engine_version" -> engineVersion,
      "source_revision" -> sourceRevision,
      "producer_artifact_sha256" -> digest,
      "generated_at_utc" -> timestamp,
      "precision_bits" -> bits
    |>,
    "problem" -> <|
      "mass_kg" -> N[massKg, digits],
      "spin_over_mass" -> a,
      "spin_weight" -> s,
      "ell" -> ell,
      "m" -> m,
      "omega_m" -> BHPTComplexObject[omega, digits],
      "angular_eigenvalue" -> BHPTComplexObject[lambda, digits],
      "angular_eigenvalue_convention" -> "teukolsky_radial_lambda",
      "angular_eigenvalue_definition" -> Null,
      "angular_branch_id" -> branchId,
      "source_id" -> sourceId
    |>,
    "fourier_convention" -> fourier,
    "radial_coordinate" -> "boyer_lindquist_r_over_m",
    "frequency_normalization" -> "dimensionless_m_omega",
    "normalization" -> <|
      "in_basis" -> "BHPT TeukolskyRadial In solution with package retarded horizon boundary condition and returned amplitude normalization",
      "up_basis" -> "BHPT TeukolskyRadial Up solution with package retarded infinity boundary condition and returned amplitude normalization",
      "radial_function_definition" -> "Spin-weight-s Teukolsky radial function R_s_lm in BHPT package convention",
      "derivative_definition" -> "d/d(r/M), with dimensionless M=1 radial coordinate"
    |>,
    "data" -> <|
      "radial_samples" -> samples,
      "diagnostics" -> Null
    |>
  |>;

  Export[outputPath, fixture, "JSON"];
  <|
    "status" -> "exported",
    "fixture_id" -> config["FixtureId"],
    "output_path" -> outputPath,
    "radial_samples" -> Length[samples],
    "angular_eigenvalue_convention" -> "teukolsky_radial_lambda",
    "precision_bits" -> bits,
    "producer_artifact_sha256" -> digest
  |>
];

(* Example after loading both BHPT paclets:
   Needs["SpinWeightedSpheroidalHarmonics" <> FromCharacterCode[96]];
   Needs["Teukolsky" <> FromCharacterCode[96]];
   BHPTExportTeukolskyRadialFixture[<|
     "FixtureId" -> "bhpt-s-2-l2-m2-a03-w05",
     "OutputPath" -> "bhpt-s-2-l2-m2-a03-w05.json",
     "MassKg" -> 1.98847*^30,
     "SpinWeight" -> -2, "Ell" -> 2, "M" -> 2,
     "SpinOverMass" -> SetPrecision[3/10, 100],
     "OmegaM" -> SetPrecision[1/2, 100],
     "PrecisionBits" -> 300,
     "RadialNodesOverM" -> SetPrecision[{3, 4, 6, 10, 20}, 100],
     "AngularBranchId" -> "bhpt-real-gamma-l2-branch",
     "FourierConvention" -> "e_minus_i_omega_t",
     "TeukolskyVersion" -> "1.2.1",
     "SourceRevision" -> "pinned-BHPT-paclet-source"
   |>]
*)
