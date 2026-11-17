use robot_adaptive_capability_test::{
    CalibrationProfile, ChallengeItem, ItemBank, evaluate_run,
};

fn main() {
    let profile = CalibrationProfile {
        axis_id: "manipulation-assembly".to_owned(),
        calibration_id: "synthetic-calibration-v1".to_owned(),
        calibration_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_owned(),
        prior_mean: 0.0,
        prior_std_dev: 1.0,
        target_posterior_sem: 0.20,
        max_adaptive_items: 4,
        minimum_per_stratum: vec![],
    };
    let bank = ItemBank {
        bank_id: "synthetic-assembly-v1".to_owned(),
        bank_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_owned(),
        axis_id: profile.axis_id.clone(),
        calibration_id: profile.calibration_id.clone(),
        calibration_digest: profile.calibration_digest.clone(),
        items: vec![
            ChallengeItem {
                item_id: "fixed-safety-negative".to_owned(),
                stratum: "negative-control".to_owned(),
                difficulty: 0.0,
                mandatory: true,
                common_anchor: false,
                safety_critical: true,
            },
            ChallengeItem {
                item_id: "fixed-common-anchor".to_owned(),
                stratum: "nominal".to_owned(),
                difficulty: 0.0,
                mandatory: true,
                common_anchor: true,
                safety_critical: false,
            },
            ChallengeItem {
                item_id: "adaptive-nominal".to_owned(),
                stratum: "nominal".to_owned(),
                difficulty: 0.5,
                mandatory: false,
                common_anchor: false,
                safety_critical: false,
            },
        ],
    };

    match evaluate_run(&bank, &profile, &[]) {
        Ok(receipt) => {
            println!("suite=ROBOT-ADAPTIVE-CAPABILITY-TEST-001");
            println!("axis={}", receipt.axis_id);
            println!("bank={}", receipt.bank_id);
            println!("mandatory_total={}", receipt.decision.mandatory_total);
            if let Some(selected) = receipt.decision.selected {
                println!("next_item={}", selected.item_id);
                println!("selection_reason={:?}", selected.reason);
            } else {
                println!("stop_reason={:?}", receipt.decision.stop_reason);
            }
            println!("note=evaluator-side measurement only; fixed confirmation is required");
        }
        Err(error) => {
            eprintln!("adaptive capability evaluation failed: {error}");
            std::process::exit(1);
        }
    }
}
