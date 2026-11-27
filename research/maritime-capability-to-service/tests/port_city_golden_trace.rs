use std::collections::{BTreeMap, BTreeSet};

const OWNER_JSON: &str = include_str!(concat!(
    "../../../docs/research/maritime-capability-to-service/fixtures/",
    "mycelix-port-city-service-equivalence-v0.1.json"
));
const PROJECTION: &str = include_str!(concat!(
    "../../../docs/research/maritime-capability-to-service/fixtures/",
    "port-city-service-equivalence-projection-v0.1.tsv"
));

#[derive(Debug)]
struct Policy {
    import_kw: [f64; 4],
    peak_kw: f64,
    energy_mwh: f64,
    cap_exceedance_kw: f64,
    compute_energy_mwh: f64,
    compute_work_units: f64,
    cooling_mwh: f64,
    shore_mwh: f64,
    terminal_mwh: f64,
    fixed_port_mwh: f64,
    ro_mwh: f64,
    compute_kw: [f64; 4],
    cooling_kw: [f64; 4],
}

#[derive(Debug)]
struct PccProfile {
    limit_kw: f64,
    fixed_port_kw: [f64; 4],
    vessel_shore_kw: [f64; 4],
    terminal_kw: [f64; 4],
    ro_kw: [f64; 4],
    renewables_kw: [f64; 4],
}

#[derive(Debug)]
struct HeatPartition {
    it_heat_kw: f64,
    recovered_source_kw: f64,
    useful_sink_kw: f64,
    loss_kw: f64,
    direct_seawater_kw: f64,
    recovery_residual_kw: f64,
    it_residual_kw: f64,
}

struct Trace {
    cases: Vec<(String, String, String)>,
    policies: BTreeMap<String, Policy>,
    profile: PccProfile,
    heat: BTreeMap<String, HeatPartition>,
}

fn number(value: &str, label: &str) -> f64 {
    value
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("{label} must be numeric, got {value}"))
}

fn array4(fields: &[&str], start: usize, label: &str) -> [f64; 4] {
    [
        number(fields[start], label),
        number(fields[start + 1], label),
        number(fields[start + 2], label),
        number(fields[start + 3], label),
    ]
}

fn parse_projection(input: &str) -> Trace {
    let mut cases = Vec::new();
    let mut policies = BTreeMap::new();
    let mut profile = None;
    let mut heat = BTreeMap::new();

    for (line_index, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "CASE" => {
                assert_eq!(f.len(), 4, "CASE row {}", line_index + 1);
                cases.push((f[1].to_owned(), f[2].to_owned(), f[3].to_owned()));
            }
            "POLICY" => {
                assert_eq!(f.len(), 24, "POLICY row {}", line_index + 1);
                let policy = Policy {
                    import_kw: array4(&f, 2, "PCC import"),
                    peak_kw: number(f[6], "PCC peak"),
                    energy_mwh: number(f[7], "PCC energy"),
                    cap_exceedance_kw: number(f[8], "PCC exceedance"),
                    compute_energy_mwh: number(f[9], "compute energy"),
                    compute_work_units: number(f[10], "compute useful work"),
                    cooling_mwh: number(f[11], "cooling energy"),
                    shore_mwh: number(f[12], "shore energy"),
                    terminal_mwh: number(f[13], "terminal energy"),
                    fixed_port_mwh: number(f[14], "fixed-port energy"),
                    ro_mwh: number(f[15], "RO electric energy"),
                    compute_kw: array4(&f, 16, "compute power"),
                    cooling_kw: array4(&f, 20, "cooling power"),
                };
                assert!(policies.insert(f[1].to_owned(), policy).is_none());
            }
            "PROFILE" => {
                assert_eq!(f.len(), 22, "PROFILE row {}", line_index + 1);
                assert!(profile.is_none());
                profile = Some(PccProfile {
                    limit_kw: number(f[1], "PCC limit"),
                    fixed_port_kw: array4(&f, 2, "fixed-port power"),
                    vessel_shore_kw: array4(&f, 6, "vessel shore power"),
                    terminal_kw: array4(&f, 10, "terminal power"),
                    ro_kw: array4(&f, 14, "RO power"),
                    renewables_kw: array4(&f, 18, "renewable generation"),
                });
            }
            "HEAT" => {
                assert_eq!(f.len(), 9, "HEAT row {}", line_index + 1);
                let partition = HeatPartition {
                    it_heat_kw: number(f[2], "IT heat"),
                    recovered_source_kw: number(f[3], "recovered source heat"),
                    useful_sink_kw: number(f[4], "useful sink heat"),
                    loss_kw: number(f[5], "recovery loss"),
                    direct_seawater_kw: number(f[6], "direct seawater heat"),
                    recovery_residual_kw: number(f[7], "recovery residual"),
                    it_residual_kw: number(f[8], "IT heat residual"),
                };
                assert!(heat.insert(f[1].to_owned(), partition).is_none());
            }
            other => panic!("unknown projection record {other} at line {}", line_index + 1),
        }
    }

    Trace {
        cases,
        policies,
        profile: profile.expect("exactly one PCC profile"),
        heat,
    }
}

fn close(left: f64, right: f64) {
    assert!(
        (left - right).abs() <= 1e-9,
        "expected {right}, received {left}"
    );
}

#[test]
fn all_forty_eight_owner_dispositions_are_present_and_source_bound() {
    assert!(OWNER_JSON.contains(
        "\"fixture_id\": \"port-city-service-equivalence-4slot-v0-1\""
    ));
    assert!(OWNER_JSON.contains("\"case_count\": 48"));
    assert!(OWNER_JSON.contains("\"prohibited_scalar_winner\": true"));

    let trace = parse_projection(PROJECTION);
    assert_eq!(trace.cases.len(), 48);

    let mut ids = BTreeSet::new();
    let mut category_counts = BTreeMap::<String, usize>::new();
    let mut dispositions = BTreeSet::new();
    for (id, category, disposition) in &trace.cases {
        assert!(ids.insert(id), "duplicate case id {id}");
        *category_counts.entry(category.clone()).or_default() += 1;
        dispositions.insert(disposition.clone());

        let marker = format!("\"case_id\": \"{id}\"");
        let start = OWNER_JSON
            .find(&marker)
            .unwrap_or_else(|| panic!("owner JSON omitted case {id}"));
        let end = OWNER_JSON[start..]
            .find('}')
            .map(|offset| start + offset)
            .expect("case object closes");
        let source_case = &OWNER_JSON[start..end];
        assert!(
            source_case.contains(&format!("\"category\": \"{category}\"")),
            "owner case {id} category differs from projection"
        );
        assert!(
            source_case.contains(&format!("\"expected_disposition\": \"{disposition}\"")),
            "owner case {id} disposition differs from projection"
        );
    }
    assert_eq!(
        category_counts,
        BTreeMap::from([
            ("Conflict".to_owned(), 2),
            ("Negative".to_owned(), 21),
            ("Positive".to_owned(), 14),
            ("Unknown".to_owned(), 11),
        ])
    );
    assert!(dispositions.len() >= 20);
}

#[test]
fn policy_traces_recompute_the_same_pcc_power_and_energy_without_service_loss() {
    let trace = parse_projection(PROJECTION);
    assert_eq!(trace.policies.len(), 3);
    close(trace.profile.limit_kw, 8_200.0);

    for (id, policy) in &trace.policies {
        let mut recomputed_peak = 0.0_f64;
        let mut recomputed_energy_mwh = 0.0_f64;
        for interval in 0..4 {
            let pcc = trace.profile.fixed_port_kw[interval]
                + trace.profile.vessel_shore_kw[interval]
                + trace.profile.terminal_kw[interval]
                + policy.compute_kw[interval]
                + policy.cooling_kw[interval]
                + trace.profile.ro_kw[interval]
                - trace.profile.renewables_kw[interval];
            close(policy.import_kw[interval], pcc);
            recomputed_peak = recomputed_peak.max(pcc);
            recomputed_energy_mwh += pcc * 0.25 / 1_000.0;
        }
        close(policy.peak_kw, recomputed_peak);
        close(policy.energy_mwh, recomputed_energy_mwh);
        close(
            policy.cap_exceedance_kw,
            (recomputed_peak - trace.profile.limit_kw).max(0.0),
        );
        close(policy.compute_energy_mwh, 3.0);
        close(policy.compute_work_units, 100.0);
        close(policy.cooling_mwh, 0.60);
        close(policy.shore_mwh, 1.0);
        close(policy.terminal_mwh, 0.95);
        close(policy.fixed_port_mwh, 0.80);
        close(policy.ro_mwh, 0.70);
    }

    let baseline = &trace.policies["baseline-inflexible"];
    let naive = &trace.policies["naive-carbon-objective-only"];
    let admitted = &trace.policies["no-new-peak-admitted"];
    close(baseline.peak_kw, 7_400.0);
    close(naive.peak_kw, 9_200.0);
    close(naive.cap_exceedance_kw, 1_000.0);
    close(admitted.peak_kw, 7_200.0);
    close(admitted.cap_exceedance_kw, 0.0);
    close(baseline.energy_mwh, 6.125);
    close(naive.energy_mwh, baseline.energy_mwh);
    close(admitted.energy_mwh, baseline.energy_mwh);

    // The guarded schedule preserves the exact useful-work and service-energy
    // obligations. A peak reduction bought by dropped work is not admitted.
    close(naive.compute_work_units, baseline.compute_work_units);
    close(admitted.compute_work_units, baseline.compute_work_units);
    close(naive.shore_mwh, baseline.shore_mwh);
    close(admitted.shore_mwh, baseline.shore_mwh);
    close(naive.terminal_mwh, baseline.terminal_mwh);
    close(admitted.terminal_mwh, baseline.terminal_mwh);
}

#[test]
fn heat_partitions_close_but_never_create_water_product_credit() {
    let trace = parse_projection(PROJECTION);
    assert_eq!(trace.heat.len(), 3);
    for (id, heat) in &trace.heat {
        close(
            heat.recovered_source_kw,
            heat.useful_sink_kw + heat.loss_kw,
        );
        close(
            heat.it_heat_kw,
            heat.recovered_source_kw + heat.direct_seawater_kw,
        );
        close(heat.recovery_residual_kw, 0.0);
        close(heat.it_residual_kw, 0.0);
        assert!(trace.policies.contains_key(id));
    }

    assert!(OWNER_JSON.contains("\"qualified_product_water_m3\": null"));
    assert!(OWNER_JSON.contains("\"feedwater_quality_evidence\": \"not_supplied\""));
    assert!(OWNER_JSON.contains("\"environmental_discharge_limit\": null"));
    assert!(
        OWNER_JSON.contains("no RO SEC, water-quality or environmental compliance credit")
            || OWNER_JSON.contains("no RO, potable-water or environmental compliance credit")
    );
}

#[test]
fn cross_owner_contract_keeps_scorecard_vector_non_scalar_and_authority_separate() {
    for marker in [
        "\"port_schedule_verifier\": \"issue:#4582\"",
        "\"compute_flex\": \"issue:#6211\"",
        "\"water_heat_coupling\": \"issue:#6258\"",
        "\"common_cause_dependencies\": \"issue:#5762\"",
        "\"role\": \"propose_and_abstain_not_self_certify\"",
        "\"same_scenario_ids\": true",
        "\"same_compute_job_set\": true",
        "\"same_water_quality_evidence\": true",
    ] {
        assert!(OWNER_JSON.contains(marker), "owner contract lost {marker}");
    }
}
