use std::collections::BTreeSet;

pub const REFERENCE_CSV: &str =
    include_str!("../data/transnet-trim-2026-corridor-scorecard.csv");

const EXPECTED_HEADER: &str = "corridor,train_execution_target_pct,train_execution_actual_pct,otd_target_pct,otd_including_80s_actual_pct,otd_excluding_80s_actual_pct,ota_target_pct,ota_actual_pct";

const EXPECTED_CORRIDORS: [&str; 6] = [
    "Central Corridor",
    "Container Corridor",
    "North Corridor",
    "North-East Corridor",
    "Ore Corridor",
    "Cape Corridor",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorridorObservation {
    pub corridor: String,
    pub train_execution_target_pct: u8,
    pub train_execution_actual_pct: u8,
    pub otd_target_pct: u8,
    pub otd_including_80s_actual_pct: u8,
    pub otd_excluding_80s_actual_pct: u8,
    pub ota_target_pct: u8,
    pub ota_actual_pct: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetGaps {
    pub train_execution_pp: i16,
    pub otd_including_80s_pp: i16,
    pub otd_excluding_80s_pp: i16,
    pub ota_pp: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScorecardRow {
    pub observation: CorridorObservation,
    pub gaps: TargetGaps,
}

impl CorridorObservation {
    pub fn target_gaps(&self) -> TargetGaps {
        TargetGaps {
            train_execution_pp: i16::from(self.train_execution_actual_pct)
                - i16::from(self.train_execution_target_pct),
            otd_including_80s_pp: i16::from(self.otd_including_80s_actual_pct)
                - i16::from(self.otd_target_pct),
            otd_excluding_80s_pp: i16::from(self.otd_excluding_80s_actual_pct)
                - i16::from(self.otd_target_pct),
            ota_pp: i16::from(self.ota_actual_pct) - i16::from(self.ota_target_pct),
        }
    }
}

fn parse_percentage(value: &str, field: &str, line_number: usize) -> Result<u8, String> {
    let parsed: u16 = value.parse().map_err(|_| {
        format!("line {line_number}: {field} must be an integer percentage, got {value:?}")
    })?;

    if parsed > 100 {
        return Err(format!(
            "line {line_number}: {field} must be in 0..=100, got {parsed}"
        ));
    }

    Ok(parsed as u8)
}

/// Parse the frozen, deliberately simple CSV profile.
///
/// This parser is intentionally not a general CSV implementation: the
/// admitted source fixture has a fixed eight-column header and no quoted
/// fields. A schema or quoting change must be reviewed, not silently guessed.
pub fn parse_scorecard(csv: &str) -> Result<Vec<ScorecardRow>, String> {
    let mut lines = csv.lines().filter(|line| !line.trim().is_empty());

    let header = lines
        .next()
        .ok_or_else(|| "scorecard is empty".to_string())?
        .trim();

    if header != EXPECTED_HEADER {
        return Err(format!(
            "unexpected header; expected exact v0.1 columns, got {header:?}"
        ));
    }

    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();

    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let fields: Vec<&str> = line.trim().split(',').collect();

        if fields.len() != 8 {
            return Err(format!(
                "line {line_number}: expected 8 fields, found {}",
                fields.len()
            ));
        }

        let corridor = fields[0].trim();
        if !EXPECTED_CORRIDORS.contains(&corridor) {
            return Err(format!(
                "line {line_number}: unknown corridor {corridor:?}"
            ));
        }

        if !seen.insert(corridor.to_string()) {
            return Err(format!(
                "line {line_number}: duplicate corridor {corridor:?}"
            ));
        }

        let observation = CorridorObservation {
            corridor: corridor.to_string(),
            train_execution_target_pct: parse_percentage(
                fields[1],
                "train_execution_target_pct",
                line_number,
            )?,
            train_execution_actual_pct: parse_percentage(
                fields[2],
                "train_execution_actual_pct",
                line_number,
            )?,
            otd_target_pct: parse_percentage(fields[3], "otd_target_pct", line_number)?,
            otd_including_80s_actual_pct: parse_percentage(
                fields[4],
                "otd_including_80s_actual_pct",
                line_number,
            )?,
            otd_excluding_80s_actual_pct: parse_percentage(
                fields[5],
                "otd_excluding_80s_actual_pct",
                line_number,
            )?,
            ota_target_pct: parse_percentage(fields[6], "ota_target_pct", line_number)?,
            ota_actual_pct: parse_percentage(fields[7], "ota_actual_pct", line_number)?,
        };

        let gaps = observation.target_gaps();
        rows.push(ScorecardRow { observation, gaps });
    }

    if rows.len() != EXPECTED_CORRIDORS.len() {
        return Err(format!(
            "expected exactly {} corridor rows, found {}",
            EXPECTED_CORRIDORS.len(),
            rows.len()
        ));
    }

    for expected in EXPECTED_CORRIDORS {
        if !seen.contains(expected) {
            return Err(format!("missing required corridor {expected:?}"));
        }
    }

    Ok(rows)
}

pub fn load_reference_scorecard() -> Result<Vec<ScorecardRow>, String> {
    parse_scorecard(REFERENCE_CSV)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_snapshot_contains_all_six_corridors() {
        let rows = load_reference_scorecard().expect("reference corpus should validate");
        assert_eq!(rows.len(), 6);
        assert_eq!(
            rows.iter()
                .map(|row| row.observation.corridor.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            6
        );
    }

    #[test]
    fn all_values_match_the_frozen_published_table() {
        let expected = [
            ("Central Corridor", 60, 40, 95, 88, 56, 60, 50),
            ("Container Corridor", 60, 24, 95, 96, 18, 60, 56),
            ("North Corridor", 75, 69, 95, 92, 6, 60, 23),
            ("North-East Corridor", 75, 71, 95, 82, 36, 60, 52),
            ("Ore Corridor", 95, 91, 95, 93, 1, 60, 35),
            ("Cape Corridor", 95, 75, 95, 97, 70, 60, 36),
        ];
        let rows = load_reference_scorecard().expect("reference corpus should validate");

        for (row, expected) in rows.iter().zip(expected) {
            let observed = &row.observation;
            assert_eq!(
                (
                    observed.corridor.as_str(),
                    observed.train_execution_target_pct,
                    observed.train_execution_actual_pct,
                    observed.otd_target_pct,
                    observed.otd_including_80s_actual_pct,
                    observed.otd_excluding_80s_actual_pct,
                    observed.ota_target_pct,
                    observed.ota_actual_pct,
                ),
                expected
            );
        }
    }

    #[test]
    fn container_corridor_gaps_are_percentage_points() {
        let rows = load_reference_scorecard().expect("reference corpus should validate");
        let row = rows
            .iter()
            .find(|row| row.observation.corridor == "Container Corridor")
            .expect("container corridor must exist");

        assert_eq!(row.gaps.train_execution_pp, -36);
        assert_eq!(row.gaps.otd_including_80s_pp, 1);
        assert_eq!(row.gaps.otd_excluding_80s_pp, -77);
        assert_eq!(row.gaps.ota_pp, -4);
    }

    #[test]
    fn otd_including_and_excluding_80s_remain_separate_measures() {
        let rows = load_reference_scorecard().expect("reference corpus should validate");
        let row = rows
            .iter()
            .find(|row| row.observation.corridor == "Container Corridor")
            .expect("container corridor must exist");

        assert_eq!(row.observation.otd_including_80s_actual_pct, 96);
        assert_eq!(row.observation.otd_excluding_80s_actual_pct, 18);
        assert_ne!(
            row.observation.otd_including_80s_actual_pct,
            row.observation.otd_excluding_80s_actual_pct
        );
    }

    #[test]
    fn duplicate_corridors_are_rejected() {
        let duplicate = "Central Corridor,60,40,95,88,56,60,50";
        let mut csv = REFERENCE_CSV.to_string();
        csv.push_str(duplicate);
        csv.push('\n');
        let error = parse_scorecard(&csv).expect_err("duplicate must fail");
        assert!(error.contains("duplicate corridor"));
    }

    #[test]
    fn unknown_corridors_are_rejected() {
        let csv = REFERENCE_CSV.replace("North-East Corridor", "East Corridor");
        let error = parse_scorecard(&csv).expect_err("unknown corridor must fail");
        assert!(error.contains("unknown corridor"));
    }

    #[test]
    fn out_of_range_percentages_are_rejected() {
        let csv = REFERENCE_CSV.replace(
            "North Corridor,75,69",
            "North Corridor,75,101",
        );
        let error = parse_scorecard(&csv).expect_err("101 percent must fail");
        assert!(error.contains("0..=100"));
    }

    #[test]
    fn missing_corridors_are_rejected() {
        let csv = REFERENCE_CSV
            .lines()
            .filter(|line| !line.starts_with("North-East Corridor,"))
            .collect::<Vec<_>>()
            .join("\n");
        let error = parse_scorecard(&csv).expect_err("missing corridor must fail");
        assert!(error.contains("expected exactly 6 corridor rows"));
    }

    #[test]
    fn unknown_or_reordered_schema_is_not_guessed() {
        let csv = REFERENCE_CSV.replacen(
            "otd_including_80s_actual_pct",
            "otd_including_exceptions_actual_pct",
            1,
        );
        let error = parse_scorecard(&csv).expect_err("changed header must fail");
        assert!(error.contains("unexpected header"));
    }
}
