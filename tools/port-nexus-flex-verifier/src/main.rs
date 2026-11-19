use port_nexus_flex_verifier::{
    verify_admission, AdmissionContract, FeederEnvelope, JobPlacement, WorkloadJob,
};

fn placements(slots: [usize; 4]) -> Vec<JobPlacement> {
    slots
        .into_iter()
        .enumerate()
        .map(|(index, slot)| JobPlacement {
            job_id: format!("job-{}", index + 1),
            start_slot: slot,
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = vec![14.0, 15.0, 18.0, 17.0, 10.0, 8.0, 12.0, 13.0];
    let contract = AdmissionContract {
        study_id: "synthetic-port-nexus-v0-1".to_owned(),
        pcc_boundary_id: "synthetic-pcc-01".to_owned(),
        interval_hours: 1.0,
        base_pcc_net_import_kw: base.clone(),
        pcc_max_import_kw: vec![None; 8],
        pcc_max_abs_ramp_kw: vec![None; 7],
        feeders: vec![FeederEnvelope {
            feeder_id: "port-feeder-a".to_owned(),
            base_net_import_kw: base,
            max_import_kw: vec![None; 8],
        }],
        require_no_new_peak: true,
        limit_tolerance_kw: 1e-9,
    };
    let jobs: Vec<_> = (1..=4)
        .map(|index| WorkloadJob {
            job_id: format!("job-{index}"),
            feeder_id: "port-feeder-a".to_owned(),
            power_kw: 2.0,
            duration_slots: 1,
            release_slot: 0,
            deadline_exclusive: 8,
            useful_work_units: 1.0,
        })
        .collect();
    let baseline: Vec<_> = (1..=4)
        .map(|index| JobPlacement {
            job_id: format!("job-{index}"),
            start_slot: 4,
        })
        .collect();

    let cases = [
        ("TOU unguarded", [0, 0, 0, 0]),
        ("TOU no-new-peak", [0, 0, 1, 5]),
        ("Carbon unguarded", [1, 1, 1, 1]),
        ("Carbon no-new-peak", [1, 5, 5, 5]),
        ("Post-window recovery", [7, 7, 7, 7]),
    ];

    println!("Port Nexus workload-shift admission verifier v0.1.0");
    println!("study={} pcc={} baseline_peak=18.00 kW", contract.study_id, contract.pcc_boundary_id);
    for (name, slots) in cases {
        let report = verify_admission(&contract, &jobs, &baseline, &placements(slots))?;
        let status = if report.admitted { "ADMIT" } else { "REJECT" };
        let reason_codes: Vec<_> = report
            .violations
            .iter()
            .map(|violation| violation.code.to_string())
            .collect();
        println!(
            "{name}: {status}; peak={:.2} kW; PCC_import={:.2} kWh; flexible_work={:.2} kWh; useful_work={:.2}; violations={:?}",
            report.candidate_peak_import_kw,
            report.candidate_pcc_import_energy_kwh,
            report.flexible_work_energy_kwh,
            report.useful_work_units,
            reason_codes
        );
    }
    Ok(())
}
