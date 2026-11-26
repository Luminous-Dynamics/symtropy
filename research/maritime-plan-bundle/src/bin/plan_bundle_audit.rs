use maritime_plan_bundle::{
    evaluate_bundle, BundleDisposition, CommitmentProjectionProvider, CommitmentProjectionReceipt,
    CommitmentRequirement, CommitmentState, PhysicalDisposition, PlannedReservation, ProviderError,
    ResourcePlanReceipt,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

const FIXTURE: &str = include_str!("../../fixtures/bundles.tsv");

#[derive(Default)]
struct FixtureProvider {
    projections: BTreeMap<String, CommitmentProjectionReceipt>,
    missing: BTreeSet<String>,
    unavailable: BTreeSet<String>,
}

impl CommitmentProjectionProvider for FixtureProvider {
    fn project(
        &self,
        commitment_id: &str,
        _knowledge_cutoff: i64,
    ) -> Result<Option<CommitmentProjectionReceipt>, ProviderError> {
        if self.unavailable.contains(commitment_id) {
            return Err(ProviderError::Unavailable);
        }
        if self.missing.contains(commitment_id) {
            return Ok(None);
        }
        Ok(self.projections.get(commitment_id).cloned())
    }
}

struct FixtureCase {
    plan: ResourcePlanReceipt,
    requirements: Vec<CommitmentRequirement>,
    provider: FixtureProvider,
    cutoff: i64,
    expected_bundle: String,
    expected_requirements: Vec<String>,
}

fn parse_u32(value: &str, field: &str) -> Result<u32, Box<dyn Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_i64(value: &str, field: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_physical(value: &str) -> Result<PhysicalDisposition, Box<dyn Error>> {
    match value {
        "FEASIBLE" => Ok(PhysicalDisposition::Feasible),
        "INFEASIBLE" => Ok(PhysicalDisposition::Infeasible),
        "UNKNOWN" => Ok(PhysicalDisposition::Unknown),
        _ => Err(format!("invalid physical disposition: {value}").into()),
    }
}

fn parse_state(value: &str) -> Result<CommitmentState, Box<dyn Error>> {
    match value {
        "OFFERED" => Ok(CommitmentState::Offered),
        "ACCEPTED" => Ok(CommitmentState::Accepted),
        "REJECTED" => Ok(CommitmentState::Rejected),
        "REVOKED" => Ok(CommitmentState::Revoked),
        "EXPIRED" => Ok(CommitmentState::Expired),
        _ => Err(format!("invalid commitment state: {value}").into()),
    }
}

fn case_from_rows(rows: &[Vec<&str>]) -> Result<FixtureCase, Box<dyn Error>> {
    let first = rows.first().ok_or("empty bundle case")?;
    let cutoff = parse_i64(first[1], "knowledge_cutoff")?;
    let physical = parse_physical(first[2])?;
    let plan = ResourcePlanReceipt {
        schema_version: "maritime-resource-plan/0.1".to_owned(),
        plan_id: first[3].to_owned(),
        scenario_digest: first[4].to_owned(),
        plan_digest: first[5].to_owned(),
        physical_disposition: physical,
        reservations: Vec::new(),
    };
    let mut case = FixtureCase {
        plan,
        requirements: Vec::new(),
        provider: FixtureProvider::default(),
        cutoff,
        expected_bundle: first[37].to_owned(),
        expected_requirements: Vec::new(),
    };

    for fields in rows {
        if fields.len() != 38 {
            return Err(format!("{}: expected 38 fields, got {}", fields[0], fields.len()).into());
        }
        if fields[1] != first[1]
            || fields[2] != first[2]
            || fields[3] != first[3]
            || fields[4] != first[4]
            || fields[5] != first[5]
            || fields[37] != first[37]
        {
            return Err(format!("{}: group-level fields differ", fields[0]).into());
        }

        if fields[6] != "-" {
            case.plan.reservations.push(PlannedReservation {
                reservation_id: fields[6].to_owned(),
                resource_id: fields[7].to_owned(),
                slot: parse_u32(fields[8], "slot")?,
                quantity: parse_u64(fields[9], "quantity")?,
                unit: fields[10].to_owned(),
            });
        }

        if fields[11] != "-" {
            let requirement = CommitmentRequirement {
                reservation_id: fields[11].to_owned(),
                commitment_id: fields[12].to_owned(),
                revision: parse_u32(fields[13], "requirement_revision")?,
                controller_id: fields[14].to_owned(),
                accepting_participant_id: fields[15].to_owned(),
                resource_id: fields[16].to_owned(),
                slot: parse_u32(fields[17], "requirement_slot")?,
                quantity: parse_u64(fields[18], "requirement_quantity")?,
                unit: fields[19].to_owned(),
                terms_digest: fields[20].to_owned(),
            };
            case.requirements.push(requirement);
        }

        match fields[21] {
            "PRESENT" => {
                let commitment_id = fields[23].to_owned();
                case.provider.projections.insert(
                    commitment_id.clone(),
                    CommitmentProjectionReceipt {
                        schema_version: fields[22].to_owned(),
                        commitment_id,
                        revision: parse_u32(fields[24], "projection_revision")?,
                        controller_id: fields[25].to_owned(),
                        accepting_participant_id: fields[26].to_owned(),
                        resource_id: fields[27].to_owned(),
                        slot: parse_u32(fields[28], "projection_slot")?,
                        quantity: parse_u64(fields[29], "projection_quantity")?,
                        unit: fields[30].to_owned(),
                        expires_at: parse_i64(fields[31], "expires_at")?,
                        terms_digest: fields[32].to_owned(),
                        state: parse_state(fields[33])?,
                        knowledge_cutoff: parse_i64(fields[34], "projection_cutoff")?,
                        accepted_at: if fields[35] == "-" {
                            None
                        } else {
                            Some(parse_i64(fields[35], "accepted_at")?)
                        },
                    },
                );
            }
            "MISSING" => {
                if fields[12] != "-" {
                    case.provider.missing.insert(fields[12].to_owned());
                }
            }
            "UNAVAILABLE" => {
                if fields[12] != "-" {
                    case.provider.unavailable.insert(fields[12].to_owned());
                }
            }
            "-" => {}
            other => return Err(format!("invalid projection mode: {other}").into()),
        }

        if fields[36] != "-" {
            case.expected_requirements.push(fields[36].to_owned());
        }
    }

    Ok(case)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut groups: BTreeMap<String, Vec<Vec<&str>>> = BTreeMap::new();
    for (index, raw_line) in FIXTURE.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id\t") {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 38 {
            return Err(format!(
                "fixture line {}: expected 38 fields, got {}",
                index + 1,
                fields.len()
            )
            .into());
        }
        groups.entry(fields[0].to_owned()).or_default().push(fields);
    }

    println!("PLAN_BUNDLE_RECEIPT\t0.1");
    println!("case_id\tbundle_disposition\tresult_count");
    for (case_id, rows) in &groups {
        let case = case_from_rows(rows)?;
        let report = evaluate_bundle(
            &case.plan,
            &case.requirements,
            case.cutoff,
            &case.provider,
        )?;
        if report.disposition.as_str() != case.expected_bundle {
            return Err(format!(
                "{case_id}: expected bundle {}, got {}",
                case.expected_bundle,
                report.disposition.as_str()
            )
            .into());
        }

        let actual: Vec<String> = report
            .results
            .iter()
            .map(|result| result.disposition.as_str().to_owned())
            .collect();
        if actual != case.expected_requirements {
            return Err(format!(
                "{case_id}: expected requirement results {:?}, got {:?}",
                case.expected_requirements, actual
            )
            .into());
        }

        println!(
            "BUNDLE\t{}\t{}\t{}",
            case_id,
            report.disposition.as_str(),
            report.results.len()
        );
        for result in &report.results {
            println!(
                "RESULT\t{}\t{}\t{}\t{}",
                case_id,
                result.reservation_id,
                result.commitment_id.as_deref().unwrap_or("-"),
                result.disposition.as_str()
            );
        }
    }
    println!("SUMMARY\tPASS\t{}/{}", groups.len(), groups.len());
    Ok(())
}
