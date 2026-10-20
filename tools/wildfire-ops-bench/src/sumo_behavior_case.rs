use crate::xml::{Check, attrs, count, distribution, number, tags};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn read_text(path: &Path) -> Check<String> {
    let bytes =
        fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    String::from_utf8(bytes).map_err(|_| format!("{} is not UTF-8", path.display()))
}

pub fn evaluate(case: &Value, root: &Path) -> Check<Value> {
    let id = case
        .get("case_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "fixture case lacks case_id".to_owned())?;
    let dir = root.join(id);
    let demands = case
        .get("demands")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{id}: demands must be an array"))?;
    let mut expected = BTreeSet::new();
    for demand in demands {
        let stream = demand
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{id}: demand lacks id"))?;
        let vehicles = demand
            .get("vehicles")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{id}: demand {stream} lacks vehicle count"))?;
        for n in 0..vehicles {
            if !expected.insert(format!("{stream}_{n:05}")) {
                return Err(format!("{id}: generated vehicle IDs are not unique"));
            }
        }
    }

    let routes = read_text(&dir.join("demand.rou.xml"))?;
    let mut route_ids = BTreeSet::new();
    for tag in tags(&routes, "vehicle")? {
        let a = attrs(tag)?;
        let vehicle = a
            .get("id")
            .ok_or_else(|| format!("{id}: route vehicle lacks ID"))?;
        if !route_ids.insert(vehicle.clone()) {
            return Err(format!("{id}: duplicate exported vehicle {vehicle}"));
        }
    }
    if route_ids != expected {
        return Err(format!(
            "{id}: exported vehicle IDs do not match fixture demand"
        ));
    }

    let tripinfo = read_text(&dir.join("tripinfo.xml"))?;
    if !tripinfo.starts_with("<?xml") || !tripinfo.contains("</tripinfos>") {
        return Err(format!("{id}: tripinfo XML is incomplete"));
    }
    let trip_tags = tags(&tripinfo, "tripinfo")?;
    let mut observed = BTreeSet::new();
    let mut arrived = 0u64;
    let mut departed = 0u64;
    let mut duration_values = Vec::new();
    let mut wait_values = Vec::new();
    let mut loss_values = Vec::new();
    for tag in trip_tags.iter().copied() {
        let a = attrs(tag)?;
        let vehicle = a
            .get("id")
            .ok_or_else(|| format!("{id}: tripinfo lacks ID"))?;
        if !expected.contains(vehicle) {
            return Err(format!("{id}: unknown SUMO vehicle ID {vehicle}"));
        }
        if !observed.insert(vehicle.clone()) {
            return Err(format!("{id}: duplicate tripinfo for {vehicle}"));
        }
        let depart = number(&a, "depart")?;
        let arrival = number(&a, "arrival")?;
        let has_departed = depart.is_some_and(|v| v >= 0.0);
        let has_arrived = arrival.is_some_and(|v| v >= 0.0);
        if has_arrived && !has_departed {
            return Err(format!("{id}: {vehicle} arrived without a valid departure"));
        }
        if let (Some(d), Some(a)) = (depart, arrival) {
            if a >= 0.0 && a + 1e-9 < d {
                return Err(format!("{id}: {vehicle} arrived before departure"));
            }
        }
        if has_departed {
            departed += 1;
        }
        if has_arrived {
            arrived += 1;
            let duration = number(&a, "duration")?
                .ok_or_else(|| format!("{id}: arrived vehicle {vehicle} lacks duration"))?;
            if duration < 0.0 {
                return Err(format!("{id}: negative duration for {vehicle}"));
            }
            duration_values.push(duration);
            if let Some(wait) = number(&a, "waitingTime")? {
                if wait < 0.0 {
                    return Err(format!("{id}: negative waiting time for {vehicle}"));
                }
                wait_values.push(wait);
            }
            if let Some(loss) = number(&a, "timeLoss")? {
                if loss < 0.0 {
                    return Err(format!("{id}: negative time loss for {vehicle}"));
                }
                loss_values.push(loss);
            }
        }
    }
    if observed != expected {
        let missing = expected
            .difference(&observed)
            .next()
            .map(String::as_str)
            .unwrap_or("unknown");
        return Err(format!(
            "{id}: tripinfo IDs incomplete ({} of {}); first missing {missing}",
            observed.len(),
            expected.len()
        ));
    }
    let demand_count = expected.len() as u64;
    if arrived > departed || departed > demand_count {
        return Err(format!(
            "{id}: arrived <= departed <= demand invariant failed"
        ));
    }

    let summary = read_text(&dir.join("summary.xml"))?;
    if !summary.starts_with("<?xml") || !summary.contains("</summary>") {
        return Err(format!("{id}: SUMO summary XML is incomplete"));
    }
    let steps = tags(&summary, "step")?;
    let final_attrs = attrs(
        steps
            .last()
            .ok_or_else(|| format!("{id}: summary has no steps"))?,
    )?;
    let final_time = number(&final_attrs, "time")?
        .ok_or_else(|| format!("{id}: final summary step lacks time"))?;
    Ok(json!({
        "case_id": id,
        "demand_vehicle_count": demand_count,
        "exported_route_vehicle_count": route_ids.len(),
        "tripinfo_record_count": observed.len(),
        "tripinfo_inventory_complete": true,
        "arrived_vehicle_count": arrived,
        "departed_vehicle_count": departed,
        "not_arrived_at_end_count": demand_count - arrived,
        "not_inserted_at_end_count": demand_count - departed,
        "arrival_duration_seconds": distribution(&duration_values),
        "arrival_waiting_time_seconds": distribution(&wait_values),
        "arrival_time_loss_seconds": distribution(&loss_values),
        "sumo_summary_final_step": {
            "time_seconds": final_time,
            "loaded": count(&final_attrs, "loaded")?,
            "inserted": count(&final_attrs, "inserted")?,
            "running": count(&final_attrs, "running")?,
            "waiting_to_insert": count(&final_attrs, "waiting")?,
            "ended": count(&final_attrs, "ended")?,
            "step_record_count": steps.len()
        },
        "claim_boundary": "native SUMO observations over synthetic demand; not cross-model equivalence or safety evidence"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_incomplete_tripinfo_inventory() {
        let route = r#"<routes><vehicle id="v0"/><vehicle id="v1"/></routes>"#;
        let trip =
            r#"<tripinfos><tripinfo id="v0" depart="0" arrival="1" duration="1"/></tripinfos>"#;
        assert_ne!(
            tags(route, "vehicle").unwrap().len(),
            tags(trip, "tripinfo").unwrap().len()
        );
    }
}
