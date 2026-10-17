use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const SERVICE_FLOOR_UNITS: u64 = 90;
const ORBITAL_BURDEN_BUDGET_SOBU: u64 = 15;

#[derive(Clone, Copy, Debug)]
struct OptionSpec {
    id: &'static str,
    private_lifecycle_cost_scu: u64,
    end_of_life_provision_scu: u64,
    qualified_service_units: u64,
    gross_new_burden_sobu: u64,
    claimed_prevention_sobu: u64,
    prevention_credit_verified: bool,
    claimed_removal_sobu: u64,
    removal_credit_verified: bool,
}

#[derive(Clone, Copy, Debug)]
struct Evaluation {
    id: &'static str,
    private_lifecycle_cost_scu: u64,
    end_of_life_provision_scu: u64,
    total_accounted_cost_scu: u64,
    qualified_service_units: u64,
    gross_new_burden_sobu: u64,
    claimed_prevention_sobu: u64,
    credited_prevention_sobu: u64,
    claimed_removal_sobu: u64,
    credited_removal_sobu: u64,
    residual_burden_sobu: u64,
    service_floor_pass: bool,
    burden_budget_pass: bool,
    feasible: bool,
}

const OPTIONS: [OptionSpec; 5] = [
    OptionSpec {
        id: "cheap_disposable",
        private_lifecycle_cost_scu: 60,
        end_of_life_provision_scu: 0,
        qualified_service_units: 100,
        gross_new_burden_sobu: 32,
        claimed_prevention_sobu: 0,
        prevention_credit_verified: false,
        claimed_removal_sobu: 0,
        removal_credit_verified: false,
    },
    OptionSpec {
        id: "design_for_removal",
        private_lifecycle_cost_scu: 76,
        end_of_life_provision_scu: 6,
        qualified_service_units: 100,
        gross_new_burden_sobu: 26,
        claimed_prevention_sobu: 10,
        prevention_credit_verified: true,
        claimed_removal_sobu: 3,
        removal_credit_verified: true,
    },
    OptionSpec {
        id: "service_and_reuse",
        private_lifecycle_cost_scu: 82,
        end_of_life_provision_scu: 6,
        qualified_service_units: 100,
        gross_new_burden_sobu: 18,
        claimed_prevention_sobu: 8,
        prevention_credit_verified: true,
        claimed_removal_sobu: 2,
        removal_credit_verified: true,
    },
    OptionSpec {
        id: "unverified_cleanup_claim",
        private_lifecycle_cost_scu: 58,
        end_of_life_provision_scu: 5,
        qualified_service_units: 100,
        gross_new_burden_sobu: 32,
        claimed_prevention_sobu: 0,
        prevention_credit_verified: false,
        claimed_removal_sobu: 20,
        removal_credit_verified: false,
    },
    OptionSpec {
        id: "under_service_floor",
        private_lifecycle_cost_scu: 55,
        end_of_life_provision_scu: 5,
        qualified_service_units: 80,
        gross_new_burden_sobu: 10,
        claimed_prevention_sobu: 2,
        prevention_credit_verified: true,
        claimed_removal_sobu: 0,
        removal_credit_verified: false,
    },
];

fn evaluate(spec: &OptionSpec) -> Result<Evaluation, String> {
    if spec.id.is_empty()
        || spec.private_lifecycle_cost_scu == 0
        || spec.qualified_service_units == 0
    {
        return Err(format!("{}: id, lifecycle cost and service must be positive", spec.id));
    }

    let credited_prevention = if spec.prevention_credit_verified {
        spec.claimed_prevention_sobu
    } else {
        0
    };
    if credited_prevention > spec.gross_new_burden_sobu {
        return Err(format!("{}: verified prevention cannot exceed gross burden", spec.id));
    }

    let burden_after_prevention = spec.gross_new_burden_sobu - credited_prevention;
    let credited_removal = if spec.removal_credit_verified {
        spec.claimed_removal_sobu
    } else {
        0
    };
    if credited_removal > burden_after_prevention {
        return Err(format!("{}: verified removal cannot exceed remaining burden", spec.id));
    }
    let residual_burden = burden_after_prevention - credited_removal;
    let total_cost = spec.private_lifecycle_cost_scu
        .checked_add(spec.end_of_life_provision_scu)
        .ok_or_else(|| format!("{}: lifecycle cost overflow", spec.id))?;
    let service_pass = spec.qualified_service_units >= SERVICE_FLOOR_UNITS;
    let burden_pass = residual_burden <= ORBITAL_BURDEN_BUDGET_SOBU;

    Ok(Evaluation {
        id: spec.id,
        private_lifecycle_cost_scu: spec.private_lifecycle_cost_scu,
        end_of_life_provision_scu: spec.end_of_life_provision_scu,
        total_accounted_cost_scu: total_cost,
        qualified_service_units: spec.qualified_service_units,
        gross_new_burden_sobu: spec.gross_new_burden_sobu,
        claimed_prevention_sobu: spec.claimed_prevention_sobu,
        credited_prevention_sobu: credited_prevention,
        claimed_removal_sobu: spec.claimed_removal_sobu,
        credited_removal_sobu: credited_removal,
        residual_burden_sobu: residual_burden,
        service_floor_pass: service_pass,
        burden_budget_pass: burden_pass,
        feasible: service_pass && burden_pass,
    })
}

// A portfolio dominates another only when it is no worse on every displayed
// objective: lower lifecycle cost, lower residual burden, and at least as much
// qualified service. Infeasible options are excluded from the Pareto frontier.
fn dominates(a: &Evaluation, b: &Evaluation) -> bool {
    a.feasible
        && b.feasible
        && a.total_accounted_cost_scu <= b.total_accounted_cost_scu
        && a.residual_burden_sobu <= b.residual_burden_sobu
        && a.qualified_service_units >= b.qualified_service_units
        && (a.total_accounted_cost_scu < b.total_accounted_cost_scu
            || a.residual_burden_sobu < b.residual_burden_sobu
            || a.qualified_service_units > b.qualified_service_units)
}

fn to_json(item: &Evaluation, evaluations: &[Evaluation]) -> Value {
    let dominated = evaluations.iter().any(|other| other.id != item.id && dominates(other, item));
    json!({
        "option_id": item.id,
        "private_lifecycle_cost_scu": item.private_lifecycle_cost_scu,
        "end_of_life_provision_scu": item.end_of_life_provision_scu,
        "total_accounted_cost_scu": item.total_accounted_cost_scu,
        "qualified_service_units": item.qualified_service_units,
        "gross_new_burden_sobu": item.gross_new_burden_sobu,
        "claimed_prevention_sobu": item.claimed_prevention_sobu,
        "credited_prevention_sobu": item.credited_prevention_sobu,
        "claimed_removal_sobu": item.claimed_removal_sobu,
        "credited_removal_sobu": item.credited_removal_sobu,
        "residual_burden_sobu": item.residual_burden_sobu,
        "service_floor_pass": item.service_floor_pass,
        "burden_budget_pass": item.burden_budget_pass,
        "feasible": item.feasible,
        "pareto_nondominated_among_feasible": item.feasible && !dominated,
        "reason_codes": [
            if !item.service_floor_pass { Some("SERVICE_FLOOR_MISSED") } else { None },
            if !item.burden_budget_pass { Some("ORBITAL_BURDEN_BUDGET_EXCEEDED") } else { None }
        ].into_iter().flatten().collect::<Vec<_>>(),
    })
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if !is_commit_sha(source_commit) {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }
    let evaluations: Vec<Evaluation> = OPTIONS.iter().map(evaluate).collect::<Result<_, _>>()?;
    let find = |id: &str| evaluations.iter().find(|item| item.id == id)
        .ok_or_else(|| format!("missing expected option {id}"));
    let cheap = find("cheap_disposable")?;
    let design = find("design_for_removal")?;
    let reuse = find("service_and_reuse")?;
    let unverified = find("unverified_cleanup_claim")?;
    let under_service = find("under_service_floor")?;

    if cheap.feasible || cheap.total_accounted_cost_scu >= design.total_accounted_cost_scu {
        return Err("cheap disposable negative control must be cheaper but fail the burden gate".to_owned());
    }
    if !design.feasible || design.residual_burden_sobu != 13 || design.total_accounted_cost_scu != 82 {
        return Err("design-for-removal oracle mismatch".to_owned());
    }
    if !reuse.feasible || reuse.residual_burden_sobu != 8 || reuse.total_accounted_cost_scu != 88 {
        return Err("service-and-reuse oracle mismatch".to_owned());
    }
    if unverified.credited_removal_sobu != 0 || unverified.residual_burden_sobu != 32 || unverified.feasible {
        return Err("unverified-removal negative control was credited or passed".to_owned());
    }
    if under_service.service_floor_pass || under_service.feasible {
        return Err("below-floor service negative control was accepted".to_owned());
    }
    let frontier: Vec<&str> = evaluations.iter()
        .filter(|item| item.feasible && !evaluations.iter().any(|other| other.id != item.id && dominates(other, item)))
        .map(|item| item.id)
        .collect();
    if frontier.len() != 2
        || !frontier.contains(&"design_for_removal")
        || !frontier.contains(&"service_and_reuse")
    {
        return Err("the reference must preserve both cost/burden Pareto alternatives".to_owned());
    }

    let source_digest = format!("{:x}", Sha256::digest(include_bytes!("orbital_externality_reference.rs")));
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "orbital-externality-common-pool-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_deterministic_policy_gate",
        "environment_scope": "synthetic_near_earth_orbit_case",
        "service_floor_units": SERVICE_FLOOR_UNITS,
        "orbital_burden_budget_sobu": ORBITAL_BURDEN_BUDGET_SOBU,
        "burden_unit_definition": "SOBU is a synthetic fixture unit, not a physical debris-risk unit, standards score, collision probability or regulatory threshold.",
        "hard_gate_order": ["qualified_service_floor", "verified_residual_burden_budget", "lifecycle_cost_and_pareto_comparison"],
        "pareto_frontier_option_ids": frontier,
        "options": evaluations.iter().map(|item| to_json(item, &evaluations)).collect::<Vec<_>>(),
        "claim_ceiling": "Synthetic near-Earth-orbit decision contract only; not an orbital dynamics simulation, collision-risk estimate, ISO/ESA/NASA compliance claim, lunar-orbit calibration or flight qualification."
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example orbital_externality_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("orbital_externality_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheap_disposable_option_fails_hard_burden_gate_before_cost_ranking() {
        let cheap = evaluate(&OPTIONS[0]).unwrap();
        let design = evaluate(&OPTIONS[1]).unwrap();
        assert!(cheap.total_accounted_cost_scu < design.total_accounted_cost_scu);
        assert!(!cheap.burden_budget_pass);
        assert!(!cheap.feasible);
    }

    #[test]
    fn verified_prevention_and_removal_are_accounted_separately() {
        let design = evaluate(&OPTIONS[1]).unwrap();
        assert_eq!(design.gross_new_burden_sobu, 26);
        assert_eq!(design.credited_prevention_sobu, 10);
        assert_eq!(design.credited_removal_sobu, 3);
        assert_eq!(design.residual_burden_sobu, 13);
        assert_eq!(design.total_accounted_cost_scu, 82);
    }

    #[test]
    fn unverified_cleanup_claim_does_not_reduce_residual_burden() {
        let unverified = evaluate(&OPTIONS[3]).unwrap();
        assert_eq!(unverified.claimed_removal_sobu, 20);
        assert_eq!(unverified.credited_removal_sobu, 0);
        assert_eq!(unverified.residual_burden_sobu, 32);
        assert!(!unverified.feasible);
    }

    #[test]
    fn service_floor_is_a_separate_hard_gate() {
        let under = evaluate(&OPTIONS[4]).unwrap();
        assert_eq!(under.qualified_service_units, 80);
        assert!(!under.service_floor_pass);
        assert!(!under.feasible);
    }

    #[test]
    fn valid_cost_and_burden_tradeoff_preserves_both_pareto_options() {
        let evals: Vec<Evaluation> = OPTIONS.iter().map(evaluate).collect::<Result<_, _>>().unwrap();
        let frontier: Vec<&str> = evals.iter()
            .filter(|item| item.feasible && !evals.iter().any(|other| other.id != item.id && dominates(other, item)))
            .map(|item| item.id)
            .collect();
        assert_eq!(frontier.len(), 2);
        assert!(frontier.contains(&"design_for_removal"));
        assert!(frontier.contains(&"service_and_reuse"));
    }

    #[test]
    fn verified_credits_cannot_exceed_the_available_burden() {
        let invalid = OptionSpec {
            id: "overcredited",
            private_lifecycle_cost_scu: 10,
            end_of_life_provision_scu: 1,
            qualified_service_units: 100,
            gross_new_burden_sobu: 5,
            claimed_prevention_sobu: 4,
            prevention_credit_verified: true,
            claimed_removal_sobu: 2,
            removal_credit_verified: true,
        };
        assert!(evaluate(&invalid).is_err());
    }

    #[test]
    fn rejects_invalid_source_commit_before_receipt() {
        assert!(execute("not-a-commit-sha").is_err());
    }

    #[test]
    fn exact_reference_oracle_and_claim_ceiling_are_present() {
        let report = execute(&"a".repeat(40)).unwrap();
        assert_eq!(report["result"], "pass");
        assert_eq!(report["options"].as_array().unwrap().len(), 5);
        assert_eq!(report["pareto_frontier_option_ids"].as_array().unwrap().len(), 2);
        assert!(report["claim_ceiling"].as_str().unwrap().contains("not an orbital dynamics simulation"));
    }
}
