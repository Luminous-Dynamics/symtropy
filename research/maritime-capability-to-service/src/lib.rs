use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
const FIXTURE: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/mcsc-counterfactual-tiny-v0.tsv"
);

#[derive(Clone, Debug)]
struct Shock {
    start_day: u64,
    end_day: u64,
}

#[derive(Clone, Debug)]
struct Dependency {
    available_from_day: u64,
    shocks: Vec<Shock>,
}

#[derive(Clone, Debug)]
struct Service {
    id: String,
    expected_units: u64,
    unit: String,
    deadline_day: u64,
}

#[derive(Clone, Debug)]
struct PortfolioOption {
    id: String,
    region: String,
    cost_units: u64,
    production_days: u64,
    inspection_days: u64,
    commissioning_days: u64,
    handoff_days: u64,
    emissions_units: u64,
    regional_value_units: u64,
    dependencies: Vec<String>,
}

#[derive(Clone, Debug)]
struct Instance {
    id: String,
    version: String,
    service: Service,
    dependencies: BTreeMap<String, Dependency>,
    options: BTreeMap<String, PortfolioOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome {
    pub option_id: String,
    pub region: String,
    pub receiver_day_without_shock: u64,
    pub receiver_day_with_shock: u64,
    pub expected_units: u64,
    pub on_time_units_with_shock: u64,
    pub cost_units: u64,
    pub emissions_units: u64,
    pub regional_value_units: u64,
}

fn parse_u64(value: &str, label: &str, line: usize) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("line {line}: invalid {label} '{value}': {error}"))
}

fn parse_instance(input: &str) -> Result<Instance, String> {
    let mut identity: Option<(String, String)> = None;
    let mut service: Option<Service> = None;
    let mut dependencies: BTreeMap<String, Dependency> = BTreeMap::new();
    let mut options: BTreeMap<String, PortfolioOption> = BTreeMap::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied().unwrap_or_default() {
            "INSTANCE" if fields.len() == 3 => {
                if identity.is_some() || fields[1].is_empty() || fields[2].is_empty() {
                    return Err(format!("line {line_number}: duplicate or empty INSTANCE"));
                }
                identity = Some((fields[1].to_owned(), fields[2].to_owned()));
            }
            "SERVICE" if fields.len() == 5 => {
                if service.is_some() || fields[1].is_empty() || fields[3].is_empty() {
                    return Err(format!("line {line_number}: duplicate or empty SERVICE"));
                }
                let expected_units = parse_u64(fields[2], "expected units", line_number)?;
                if expected_units == 0 {
                    return Err(format!("line {line_number}: expected units must be positive"));
                }
                service = Some(Service {
                    id: fields[1].to_owned(),
                    expected_units,
                    unit: fields[3].to_owned(),
                    deadline_day: parse_u64(fields[4], "deadline day", line_number)?,
                });
            }
            "DEPENDENCY" if fields.len() == 3 => {
                if fields[1].is_empty() {
                    return Err(format!("line {line_number}: empty dependency identity"));
                }
                let dependency = Dependency {
                    available_from_day: parse_u64(
                        fields[2],
                        "dependency availability day",
                        line_number,
                    )?,
                    shocks: Vec::new(),
                };
                if dependencies.insert(fields[1].to_owned(), dependency).is_some() {
                    return Err(format!(
                        "line {line_number}: duplicate dependency '{}'",
                        fields[1]
                    ));
                }
            }
            "SHOCK" if fields.len() == 4 => {
                if fields[1].is_empty() {
                    return Err(format!("line {line_number}: empty shock dependency"));
                }
                let start_day = parse_u64(fields[2], "shock start day", line_number)?;
                let end_day = parse_u64(fields[3], "shock end day", line_number)?;
                // This v0.1 arithmetic fixture only admits a disruption already active
                // at the common decision time (day 0). Later-starting shocks need an
                // event simulator and must not be silently approximated as start-time delay.
                if start_day != 0 || end_day <= start_day {
                    return Err(format!(
                        "line {line_number}: v0.1 shock must begin at day 0 and have positive duration"
                    ));
                }
                pending_shocks.push((
                    fields[1].to_owned(),
                    Shock { start_day, end_day },
                ));
            }
            "OPTION" if fields.len() == 11 => {
                if fields[1].is_empty() || fields[2].is_empty() {
                    return Err(format!("line {line_number}: empty option identity or region"));
                }
                let mut dependency_ids: Vec<String> = if fields[10] == "-" {
                    Vec::new()
                } else {
                    fields[10].split(',').map(str::to_owned).collect()
                };
                dependency_ids.sort();
                if dependency_ids.iter().any(String::is_empty) {
                    return Err(format!("line {line_number}: empty dependency in option"));
                }
                let unique: BTreeSet<&str> =
                    dependency_ids.iter().map(String::as_str).collect();
                if unique.len() != dependency_ids.len() {
                    return Err(format!(
                        "line {line_number}: duplicate dependency in option '{}'",
                        fields[1]
                    ));
                }
                let option = PortfolioOption {
                    id: fields[1].to_owned(),
                    region: fields[2].to_owned(),
                    cost_units: parse_u64(fields[3], "cost units", line_number)?,
                    production_days: parse_u64(fields[4], "production/supply days", line_number)?,
                    inspection_days: parse_u64(fields[5], "inspection days", line_number)?,
                    commissioning_days: parse_u64(fields[6], "commissioning days", line_number)?,
                    handoff_days: parse_u64(fields[7], "receiver handoff days", line_number)?,
                    emissions_units: parse_u64(fields[8], "emissions units", line_number)?,
                    regional_value_units: parse_u64(fields[9], "regional value units", line_number)?,
                    dependencies: dependency_ids,
                };
                if options.insert(option.id.clone(), option).is_some() {
                    return Err(format!(
                        "line {line_number}: duplicate option '{}'",
                        fields[1]
                    ));
                }
            }
            record => {
                return Err(format!(
                    "line {line_number}: unknown record or wrong field count for '{record}'"
                ));
            }
        }
    }

    // Resolve shocks after parsing so record ordering is semantically irrelevant.
    for (dependency_id, shock) in pending_shocks {
        let dependency = dependencies.get_mut(&dependency_id).ok_or_else(|| {
            format!("shock references undeclared dependency '{dependency_id}'")
        })?;
        dependency.shocks.push(shock);
    }

    let (id, version) = identity.ok_or_else(|| "missing INSTANCE record".to_owned())?;
    let service = service.ok_or_else(|| "missing SERVICE record".to_owned())?;
    if dependencies.is_empty() || options.is_empty() {
        return Err("fixture requires dependencies and portfolio options".to_owned());
    }

    for option in options.values() {
        for dependency_id in &option.dependencies {
            if !dependencies.contains_key(dependency_id) {
                return Err(format!(
                    "option '{}' references undeclared dependency '{}'",
                    option.id, dependency_id
                ));
            }
        }
    }

    Ok(Instance {
        id,
        version,
        service,
        dependencies,
        options,
    })
}

fn effective_dependency_day(dependency: &Dependency, include_shocks: bool) -> u64 {
    let mut available = dependency.available_from_day;
    if include_shocks {
        for shock in &dependency.shocks {
            if shock.start_day == 0 {
                available = available.max(shock.end_day);
            }
        }
    }
    available
}

fn receiver_day(
    instance: &Instance,
    option: &PortfolioOption,
    include_shocks: bool,
) -> Result<u64, String> {
    let ready_day = option
        .dependencies
        .iter()
        .map(|dependency_id| {
            instance
                .dependencies
                .get(dependency_id)
                .expect("parser validates every option dependency")
        })
        .map(|dependency| effective_dependency_day(dependency, include_shocks))
        .max()
        .unwrap_or(0);

    [
        option.production_days,
        option.inspection_days,
        option.commissioning_days,
        option.handoff_days,
    ]
    .into_iter()
    .try_fold(ready_day, |day, duration| {
        day.checked_add(duration)
            .ok_or_else(|| format!("receiver-day overflow for option '{}'", option.id))
    })
}

fn evaluate(instance: &Instance) -> Result<Vec<Outcome>, String> {
    instance
        .options
        .values()
        .map(|option| {
            let receiver_day_without_shock = receiver_day(instance, option, false)?;
            let receiver_day_with_shock = receiver_day(instance, option, true)?;
            let on_time_units_with_shock =
                if receiver_day_with_shock <= instance.service.deadline_day {
                    instance.service.expected_units
                } else {
                    0
                };
            Ok(Outcome {
                option_id: option.id.clone(),
                region: option.region.clone(),
                receiver_day_without_shock,
                receiver_day_with_shock,
                expected_units: instance.service.expected_units,
                on_time_units_with_shock,
                cost_units: option.cost_units,
                emissions_units: option.emissions_units,
                regional_value_units: option.regional_value_units,
            })
        })
        .collect()
}

fn render_receipt(instance: &Instance, outcomes: &[Outcome]) -> String {
    let mut lines = vec![
        "MCSC_RECEIPT\t0.1".to_owned(),
        format!("INSTANCE\t{}\t{}", instance.id, instance.version),
        format!(
            "SERVICE\t{}\t{}\t{}\t{}",
            instance.service.id,
            instance.service.expected_units,
            instance.service.unit,
            instance.service.deadline_day
        ),
    ];
    for (dependency_id, dependency) in &instance.dependencies {
        lines.push(format!(
            "DEPENDENCY\t{}\t{}",
            dependency_id, dependency.available_from_day
        ));
        for shock in &dependency.shocks {
            lines.push(format!(
                "SHOCK\t{}\t{}\t{}",
                dependency_id, shock.start_day, shock.end_day
            ));
        }
    }
    for option in instance.options.values() {
        let dependencies = if option.dependencies.is_empty() {
            "-".to_owned()
        } else {
            option.dependencies.join(",")
        };
        lines.push(format!(
            "OPTION\t{}\\t{}\\t{}\\t{}\\t{}\\t{}\\t{}\\t{}\\t{}\\t{}",
            option.id,
            option.region,
            option.cost_units,
            option.production_days,
            option.inspection_days,
            option.commissioning_days,
            option.handoff_days,
            option.emissions_units,
            option.regional_value_units,
            dependencies
        ));
    }
    for outcome in outcomes {
        lines.push(format!(
            "OUTCOME\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            outcome.option_id,
            outcome.region,
            outcome.receiver_day_without_shock,
            outcome.receiver_day_with_shock,
            outcome.expected_units,
            outcome.on_time_units_with_shock,
            outcome.cost_units,
            outcome.emissions_units,
            outcome.regional_value_units,
            instance.service.deadline_day
        ));
    }
    lines.join("\n") + "\n"
}

pub fn run_audit(input: &str) -> Result<String, String> {
    let instance = parse_instance(input)?;
    let outcomes = evaluate(&instance)?;
    Ok(render_receipt(&instance, &outcomes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_microcase_matches_all_five_expected_outcomes() {
        let instance = parse_instance(FIXTURE).expect("fixture parses");
        let outcomes = evaluate(&instance).expect("evaluation succeeds");
        let expected = [
            ("EXISTING", 23, 23, 0, 90, 45, 0),
            ("IMPORT", 13, 13, 0, 100, 40, 5),
            ("LOCAL-A", 8, 14, 0, 130, 30, 80),
            ("LOCAL-B", 9, 15, 0, 125, 35, 70),
            ("LOCAL-C", 10, 10, 10, 145, 32, 60),
        ];
        assert_eq!(outcomes.len(), expected.len());
        for (outcome, (id, no_shock, with_shock, on_time, cost, emissions, regional)) in
            outcomes.iter().zip(expected)
        {
            assert_eq!(outcome.option_id, id);
            assert_eq!(outcome.receiver_day_without_shock, no_shock);
            assert_eq!(outcome.receiver_day_with_shock, with_shock);
            assert_eq!(outcome.expected_units, 10);
            assert_eq!(outcome.on_time_units_with_shock, on_time);
            assert_eq!(outcome.cost_units, cost);
            assert_eq!(outcome.emissions_units, emissions);
            assert_eq!(outcome.regional_value_units, regional);
        }
    }

    #[test]
    fn late_outcome_keeps_the_full_expected_denominator() {
        let instance = parse_instance(FIXTURE).expect("fixture parses");
        let import = evaluate(&instance)
            .expect("evaluation succeeds")
            .into_iter()
            .find(|outcome| outcome.option_id == "IMPORT")
            .expect("import option exists");
        assert_eq!(import.expected_units, 10);
        assert_eq!(import.on_time_units_with_shock, 0);
    }

    #[test]
    fn input_record_order_does_not_change_the_receipt() {
        let reversed = FIXTURE.lines().rev().collect::<Vec<_>>().join("\n");
        assert_eq!(run_audit(FIXTURE).unwrap(), run_audit(&reversed).unwrap());
    }

    #[test]
    fn unknown_dependency_fails_closed() {
        let changed = FIXTURE.replace(
            "MILL-01,GRID-A,NDT-01",
            "MILL-UNKNOWN,GRID-A,NDT-01",
        );
        assert!(parse_instance(&changed)
            .unwrap_err()
            .contains("undeclared dependency"));
    }

    #[test]
    fn duplicate_dependency_fails_closed() {
        let changed = FIXTURE.replace(
            "MILL-01,GRID-A,NDT-01",
            "MILL-01,MILL-01,NDT-01",
        );
        assert!(parse_instance(&changed)
            .unwrap_err()
            .contains("duplicate dependency"));
    }

    #[test]
    fn nonzero_start_shock_is_not_approximated() {
        let changed = FIXTURE.replace("SHOCK\tGRID-A\t0\t6", "SHOCK\tGRID-A\t2\t6");
        assert!(parse_instance(&changed)
            .unwrap_err()
            .contains("must begin at day 0"));
    }
}
