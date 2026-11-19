#![forbid(unsafe_code)]

//! Dependency-free replay of paired-actuator run ledgers and encoder-speed
//! traces. This audits source data; it does not validate sensors or prove
//! mechanical load sharing.

use std::collections::BTreeMap;
use std::mem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl CsvTable {
    /// Parse CSV with quoted fields, doubled quotes, embedded commas/newlines,
    /// CRLF and an optional UTF-8 BOM. Blank separator rows are ignored.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut field = String::new();
        let mut chars = input.chars().peekable();
        let mut in_quotes = false;
        let mut after_quote = false;
        let mut field_started = false;

        while let Some(ch) = chars.next() {
            if in_quotes {
                if ch == '"' {
                    if chars.peek() == Some(&'"') {
                        field.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                        after_quote = true;
                    }
                } else {
                    field.push(ch);
                }
                continue;
            }

            if after_quote {
                match ch {
                    ',' => {
                        row.push(mem::take(&mut field));
                        after_quote = false;
                        field_started = false;
                    }
                    '\n' | '\r' => {
                        if ch == '\r' && chars.peek() == Some(&'\n') {
                            chars.next();
                        }
                        row.push(mem::take(&mut field));
                        push_nonblank_row(&mut rows, &mut row);
                        after_quote = false;
                        field_started = false;
                    }
                    ' ' | '\t' => {}
                    other => {
                        return Err(format!("unexpected character {other:?} after closing CSV quote"));
                    }
                }
                continue;
            }

            match ch {
                '"' if !field_started && field.is_empty() => {
                    in_quotes = true;
                    field_started = true;
                }
                '"' => return Err("quote encountered inside an unquoted CSV field".to_owned()),
                ',' => {
                    row.push(mem::take(&mut field));
                    field_started = false;
                }
                '\n' | '\r' => {
                    if ch == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    row.push(mem::take(&mut field));
                    push_nonblank_row(&mut rows, &mut row);
                    field_started = false;
                }
                other => {
                    field.push(other);
                    field_started = true;
                }
            }
        }

        if in_quotes {
            return Err("unterminated quoted CSV field".to_owned());
        }
        if !field.is_empty() || !row.is_empty() || field_started || after_quote {
            row.push(field);
            push_nonblank_row(&mut rows, &mut row);
        }
        if rows.is_empty() {
            return Err("CSV contains no header or data rows".to_owned());
        }

        let mut headers = rows.remove(0);
        if let Some(first) = headers.first_mut() {
            *first = first.trim_start_matches('\u{feff}').to_owned();
        }
        if headers.iter().any(|header| header.trim().is_empty()) {
            return Err("CSV header contains an empty column name".to_owned());
        }
        for (index, data_row) in rows.iter().enumerate() {
            if data_row.len() != headers.len() {
                return Err(format!(
                    "CSV row {} has {} fields; header has {}",
                    index + 2,
                    data_row.len(),
                    headers.len()
                ));
            }
        }
        Ok(Self { headers, rows })
    }

    fn column_index(&self, name: &str) -> Result<usize, String> {
        let matches: Vec<usize> = self.headers.iter().enumerate()
            .filter_map(|(i, header)| (header == name).then_some(i))
            .collect();
        match matches.as_slice() {
            [index] => Ok(*index),
            [] => Err(format!("required CSV column {name:?} is missing")),
            _ => Err(format!("CSV column {name:?} is duplicated")),
        }
    }
}

fn push_nonblank_row(rows: &mut Vec<Vec<String>>, row: &mut Vec<String>) {
    if row.iter().all(|cell| cell.trim().is_empty()) {
        row.clear();
    } else {
        rows.push(mem::take(row));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentCounts {
    pub experiment: String,
    pub attempted: usize,
    pub included: usize,
    pub excluded_or_invalid: usize,
    pub unknown_validity: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLedgerSummary {
    pub attempted_runs: usize,
    pub included_runs: usize,
    pub excluded_or_invalid_runs: usize,
    pub unknown_validity_runs: usize,
    pub by_experiment: Vec<ExperimentCounts>,
}

impl RunLedgerSummary {
    /// Count each nonblank ledger row. Inclusion is not task completion.
    pub fn from_csv(table: &CsvTable) -> Result<Self, String> {
        let run_id_col = table.column_index("run_id")?;
        let experiment_col = table.column_index("experiment")?;
        let valid_col = table.column_index("valid")?;

        #[derive(Default)]
        struct Counts { attempted: usize, included: usize, excluded: usize, unknown: usize }

        let mut groups: BTreeMap<String, Counts> = BTreeMap::new();
        let mut result = Self {
            attempted_runs: 0,
            included_runs: 0,
            excluded_or_invalid_runs: 0,
            unknown_validity_runs: 0,
            by_experiment: Vec::new(),
        };

        for (index, row) in table.rows.iter().enumerate() {
            let run_id = row[run_id_col].trim();
            let experiment = row[experiment_col].trim();
            if run_id.is_empty() || experiment.is_empty() {
                return Err(format!("run ledger row {} has blank identity", index + 2));
            }
            result.attempted_runs += 1;
            let group = groups.entry(experiment.to_owned()).or_default();
            group.attempted += 1;
            match row[valid_col].trim().to_ascii_lowercase().as_str() {
                "yes" => {
                    result.included_runs += 1;
                    group.included += 1;
                }
                "no" => {
                    result.excluded_or_invalid_runs += 1;
                    group.excluded += 1;
                }
                "" => return Err(format!("run ledger row {} has blank validity", index + 2)),
                _ => {
                    result.unknown_validity_runs += 1;
                    group.unknown += 1;
                }
            }
        }

        result.by_experiment = groups.into_iter().map(|(experiment, c)| ExperimentCounts {
            experiment,
            attempted: c.attempted,
            included: c.included,
            excluded_or_invalid: c.excluded,
            unknown_validity: c.unknown,
        }).collect();
        Ok(result)
    }

    pub fn to_json(&self) -> String {
        let groups = self.by_experiment.iter().map(|g| format!(
            "{{\"experiment\":{},\"attempted\":{},\"included\":{},\"excluded_or_invalid\":{},\"unknown_validity\":{}}}",
            json_quote(&g.experiment), g.attempted, g.included, g.excluded_or_invalid, g.unknown_validity
        )).collect::<Vec<_>>().join(",");
        format!(
            "{{\"attempted_runs\":{},\"included_runs\":{},\"excluded_or_invalid_runs\":{},\"unknown_validity_runs\":{},\"by_experiment\":[{}]}}",
            self.attempted_runs, self.included_runs, self.excluded_or_invalid_runs, self.unknown_validity_runs, groups
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepetitionSpeed {
    pub repetition: u32,
    pub points_in_window: usize,
    pub speed_rad_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpeedTraceSummary {
    pub q_window_deg: f64,
    pub minimum_points: usize,
    pub repetitions: Vec<RepetitionSpeed>,
    pub mean_rad_s: f64,
    pub population_sd_rad_s: f64,
}

impl SpeedTraceSummary {
    /// Absolute OLS slope of joint angle (radians) against logged time, per
    /// repetition, using samples inside the requested joint-angle window.
    pub fn from_csv(table: &CsvTable, q_window_deg: f64, minimum_points: usize) -> Result<Self, String> {
        if !q_window_deg.is_finite() || q_window_deg <= 0.0 || q_window_deg > 180.0 {
            return Err("q_window_deg must be finite and in (0, 180]".to_owned());
        }
        if minimum_points < 3 {
            return Err("minimum_points must be at least 3".to_owned());
        }

        let rep_col = table.column_index("rep")?;
        let time_col = table.column_index("t_s")?;
        let angle_col = table.column_index("q_deg")?;
        let mut grouped: BTreeMap<u32, Vec<(f64, f64)>> = BTreeMap::new();

        for (index, row) in table.rows.iter().enumerate() {
            let rep = parse_u32(&row[rep_col], "rep", index)?;
            let time = parse_f64(&row[time_col], "t_s", index)?;
            let angle = parse_f64(&row[angle_col], "q_deg", index)?;
            if angle.abs() <= q_window_deg {
                grouped.entry(rep).or_default().push((time, angle.to_radians()));
            }
        }
        if grouped.is_empty() {
            return Err("no samples fell inside the requested angle window".to_owned());
        }

        let mut repetitions = Vec::with_capacity(grouped.len());
        for (rep, mut points) in grouped {
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            if points.len() < minimum_points {
                return Err(format!("repetition {rep} has {} in-window points; need {minimum_points}", points.len()));
            }
            if points.windows(2).any(|pair| pair[1].0 <= pair[0].0) {
                return Err(format!("repetition {rep} has duplicate or non-increasing timestamps"));
            }

            let n = points.len() as f64;
            let mean_t = points.iter().map(|p| p.0).sum::<f64>() / n;
            let mean_q = points.iter().map(|p| p.1).sum::<f64>() / n;
            let mut numerator = 0.0;
            let mut denominator = 0.0;
            for (time, angle) in &points {
                numerator += (*time - mean_t) * (*angle - mean_q);
                denominator += (*time - mean_t).powi(2);
            }
            if !denominator.is_finite() || denominator <= f64::MIN_POSITIVE {
                return Err(format!("repetition {rep} has no usable timestamp variance"));
            }
            let speed = (numerator / denominator).abs();
            if !speed.is_finite() {
                return Err(format!("repetition {rep} produced non-finite speed"));
            }
            repetitions.push(RepetitionSpeed {
                repetition: rep,
                points_in_window: points.len(),
                speed_rad_s: speed,
            });
        }

        let n = repetitions.len() as f64;
        let mean = repetitions.iter().map(|r| r.speed_rad_s).sum::<f64>() / n;
        let variance = repetitions.iter().map(|r| (r.speed_rad_s - mean).powi(2)).sum::<f64>() / n;
        let sd = variance.sqrt();
        if !mean.is_finite() || !sd.is_finite() {
            return Err("speed summary produced a non-finite result".to_owned());
        }
        Ok(Self {
            q_window_deg,
            minimum_points,
            repetitions,
            mean_rad_s: mean,
            population_sd_rad_s: sd,
        })
    }

    pub fn to_json(&self, label: &str) -> String {
        let repetitions = self.repetitions.iter().map(|r| format!(
            "{{\"rep\":{},\"points_in_window\":{},\"speed_rad_s\":{:.12}}}",
            r.repetition, r.points_in_window, r.speed_rad_s
        )).collect::<Vec<_>>().join(",");
        format!(
            "{{\"label\":{},\"q_window_deg\":{:.6},\"minimum_points\":{},\"repetitions\":[{}],\"mean_rad_s\":{:.12},\"population_sd_rad_s\":{:.12}}}",
            json_quote(label), self.q_window_deg, self.minimum_points, repetitions, self.mean_rad_s, self.population_sd_rad_s
        )
    }
}

fn parse_u32(value: &str, column: &str, row: usize) -> Result<u32, String> {
    value.trim().parse::<u32>().map_err(|e| format!(
        "invalid {column} value {:?} on CSV row {}: {e}", value, row + 2
    ))
}

fn parse_f64(value: &str, column: &str, row: usize) -> Result<f64, String> {
    let parsed = value.trim().parse::<f64>().map_err(|e| format!(
        "invalid {column} value {:?} on CSV row {}: {e}", value, row + 2
    ))?;
    if !parsed.is_finite() {
        return Err(format!("non-finite {column} value on CSV row {}", row + 2));
    }
    Ok(parsed)
}

pub fn json_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => { out.push('\\'); out.push('"'); }
            '\\' => { out.push('\\'); out.push('\\'); }
            '\n' => { out.push('\\'); out.push('n'); }
            '\r' => { out.push('\\'); out.push('r'); }
            '\t' => { out.push('\\'); out.push('t'); }
            control if control.is_control() => out.push_str(&format!("\\u{:04x}", control as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parser_handles_quotes_commas_bom_and_doubled_quotes() {
        let source = r#"run_id,note,valid
R1,"comma, and ""quote""",yes
"#;
        let table = CsvTable::parse(source).expect("quoted CSV should parse");
        assert_eq!(table.rows[0][1], r#"comma, and "quote""#);
        let bom = format!("{}run_id,valid\nR2,yes\n", '\u{feff}');
        assert_eq!(CsvTable::parse(&bom).unwrap().headers[0], "run_id");
    }

    #[test]
    fn csv_parser_skips_blank_separator_rows_and_rejects_bad_width() {
        let source = "run_id,experiment,valid\nA,static,yes\n,,,\nB,dynamic,no\n";
        let table = CsvTable::parse(source).expect("separator row should be skipped");
        assert_eq!(table.rows.len(), 2);
        assert!(CsvTable::parse("a,b\n1\n").is_err());
    }

    #[test]
    fn run_ledger_reproduces_included_and_excluded_counts() {
        let source = "run_id,experiment,valid\nS1,static,yes\nS2,static,no\nD1,dynamic,yes\n";
        let table = CsvTable::parse(source).unwrap();
        let summary = RunLedgerSummary::from_csv(&table).unwrap();
        assert_eq!(summary.attempted_runs, 3);
        assert_eq!(summary.included_runs, 2);
        assert_eq!(summary.excluded_or_invalid_runs, 1);
        assert_eq!(summary.unknown_validity_runs, 0);
        assert_eq!(summary.by_experiment.len(), 2);
    }

    #[test]
    fn speed_trace_reconstructs_ols_slope_per_repetition() {
        let source = r#"rep,t_s,q_deg,note
1,0,0,"first, sample"
1,1,90,
1,2,180,
2,0,0,
2,1,90,
2,2,180,
"#;
        let table = CsvTable::parse(source).unwrap();
        let summary = SpeedTraceSummary::from_csv(&table, 180.0, 3).unwrap();
        assert_eq!(summary.repetitions.len(), 2);
        assert!((summary.mean_rad_s - std::f64::consts::FRAC_PI_2).abs() < 1.0e-12);
        assert!(summary.population_sd_rad_s < 1.0e-12);
    }

    #[test]
    fn speed_trace_filters_window_and_rejects_duplicate_timestamps() {
        let source = "rep,t_s,q_deg\n1,0,0\n1,1,90\n1,2,180\n1,3,181\n";
        let table = CsvTable::parse(source).unwrap();
        let summary = SpeedTraceSummary::from_csv(&table, 180.0, 3).unwrap();
        assert_eq!(summary.repetitions[0].points_in_window, 3);
        let invalid = "rep,t_s,q_deg\n1,0,0\n1,1,90\n1,1,120\n";
        let invalid_table = CsvTable::parse(invalid).unwrap();
        assert!(SpeedTraceSummary::from_csv(&invalid_table, 180.0, 3).is_err());
    }

    #[test]
    fn speed_trace_rejects_nonfinite_values_and_short_repetitions() {
        let nonfinite = "rep,t_s,q_deg\n1,0,0\n1,NaN,1\n1,2,2\n";
        let nonfinite_table = CsvTable::parse(nonfinite).unwrap();
        assert!(SpeedTraceSummary::from_csv(&nonfinite_table, 180.0, 3).is_err());
        let short = "rep,t_s,q_deg\n1,0,0\n1,1,1\n";
        let short_table = CsvTable::parse(short).unwrap();
        assert!(SpeedTraceSummary::from_csv(&short_table, 180.0, 3).is_err());
    }
}
