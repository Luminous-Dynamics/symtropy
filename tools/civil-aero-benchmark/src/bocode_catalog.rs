//! Strict import of BOCoDe's normalized task metadata catalog.
//!
//! The external BOCoDe Python/PyTorch package exports a normalized one-row-per-task CSV.
//! This module validates the published problem classes and identity digests without importing,
//! executing, or reimplementing BOCoDe in the production Rust crate.
//!
//! Import success establishes metadata structural consistency only. It does not authenticate the
//! upstream repository, prove a task can execute, clear optional dependency/data licenses, or
//! validate a physical model.

use std::collections::BTreeSet;

use crate::integrity::sha256_hex;
use crate::shapebench::parse_csv_record;

pub const BOCODE_CATALOG_SCHEMA: &str = "bocode-task-catalog/0.1.0";
pub const BOCODE_CATALOG_CSV_HEADER: &str = "problem_id,application,problem_class,dimension,num_objectives,num_constraints,variable_type_class,benchmark_set_member,bounds_state,bounds_sha256,variable_types_state,variable_types_sha256,source_reference_state,source_reference,metadata_sha256,required_extra";
pub const BOCODE_OBJECTIVE_DIRECTION: &str = "maximize";
pub const BOCODE_CONSTRAINT_CONVENTION: &str = "g(x)<=0_is_feasible";
pub const BOCODE_CATALOG_EVIDENCE_LEVEL: &str =
    "external_benchmark_metadata_import_only_task_execution_and_scientific_validity_unverified";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoCoDeApplication {
    Engineering,
    Hpo,
    Synthetic,
    Extended,
}

impl BoCoDeApplication {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "engineering" => Ok(Self::Engineering),
            "hpo" => Ok(Self::Hpo),
            "synthetic" => Ok(Self::Synthetic),
            "extended" => Ok(Self::Extended),
            _ => Err(format!("unsupported BOCoDe application domain {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Engineering => "engineering",
            Self::Hpo => "hpo",
            Self::Synthetic => "synthetic",
            Self::Extended => "extended",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoCoDeProblemClass {
    SingleObjectiveUnconstrained,
    SingleObjectiveConstrained,
    MultiObjectiveUnconstrained,
    MultiObjectiveConstrained,
    /// BOCoDe's paper class includes constrained and unconstrained single-objective mixed tasks.
    SingleObjectiveMixedVariable,
    /// Explicit extension for a multi-objective mixed-variable task.
    MultiObjectiveMixedVariable,
}

impl BoCoDeProblemClass {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "so_unconstrained" => Ok(Self::SingleObjectiveUnconstrained),
            "so_constrained" => Ok(Self::SingleObjectiveConstrained),
            "mo_unconstrained" => Ok(Self::MultiObjectiveUnconstrained),
            "mo_constrained" => Ok(Self::MultiObjectiveConstrained),
            "so_mixed_variable" => Ok(Self::SingleObjectiveMixedVariable),
            "mo_mixed_variable_extension" => Ok(Self::MultiObjectiveMixedVariable),
            _ => Err(format!("unsupported BOCoDe problem class {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SingleObjectiveUnconstrained => "so_unconstrained",
            Self::SingleObjectiveConstrained => "so_constrained",
            Self::MultiObjectiveUnconstrained => "mo_unconstrained",
            Self::MultiObjectiveConstrained => "mo_constrained",
            Self::SingleObjectiveMixedVariable => "so_mixed_variable",
            Self::MultiObjectiveMixedVariable => "mo_mixed_variable_extension",
        }
    }

    fn validate(
        self,
        objectives: u32,
        constraints: u32,
        variable_type_class: BoCoDeVariableTypeClass,
    ) -> Result<(), String> {
        let valid = match self {
            Self::SingleObjectiveUnconstrained => {
                objectives == 1
                    && constraints == 0
                    && variable_type_class != BoCoDeVariableTypeClass::Mixed
            }
            Self::SingleObjectiveConstrained => {
                objectives == 1
                    && constraints > 0
                    && variable_type_class != BoCoDeVariableTypeClass::Mixed
            }
            Self::MultiObjectiveUnconstrained => {
                objectives > 1
                    && constraints == 0
                    && variable_type_class != BoCoDeVariableTypeClass::Mixed
            }
            Self::MultiObjectiveConstrained => {
                objectives > 1
                    && constraints > 0
                    && variable_type_class != BoCoDeVariableTypeClass::Mixed
            }
            Self::SingleObjectiveMixedVariable => {
                objectives == 1 && variable_type_class == BoCoDeVariableTypeClass::Mixed
            }
            Self::MultiObjectiveMixedVariable => {
                objectives > 1 && variable_type_class == BoCoDeVariableTypeClass::Mixed
            }
        };
        if !valid {
            return Err(format!(
                "problem class {} contradicts objective/constraint/variable metadata",
                self.as_str()
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoCoDeVariableTypeClass {
    Continuous,
    Discrete,
    Mixed,
}

impl BoCoDeVariableTypeClass {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "continuous" => Ok(Self::Continuous),
            "discrete" => Ok(Self::Discrete),
            "mixed" => Ok(Self::Mixed),
            _ => Err(format!("unsupported BOCoDe variable_type_class {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Continuous => "continuous",
            Self::Discrete => "discrete",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoCoDeFieldAvailability {
    Present,
    Unavailable,
}

impl BoCoDeFieldAvailability {
    fn parse(value: &str, field: &str) -> Result<Self, String> {
        match value {
            "present" => Ok(Self::Present),
            "unavailable" => Ok(Self::Unavailable),
            _ => Err(format!("unsupported {field} state {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoCoDeSourceReferenceStatus {
    Present,
    Missing,
}

impl BoCoDeSourceReferenceStatus {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "present" => Ok(Self::Present),
            "missing" => Ok(Self::Missing),
            _ => Err(format!("unsupported source_reference_state {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoCoDeCatalogContext {
    /// Exact 40-character upstream BOCoDe Git commit.
    pub source_commit: String,
    /// Exact package version installed in the external benchmark environment.
    pub package_version: String,
    /// SHA-256 of the canonical sorted (problem ID, raw metadata SHA-256) inventory.
    pub source_catalog_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoCoDeTaskRecord {
    pub problem_id: String,
    pub application: BoCoDeApplication,
    pub problem_class: BoCoDeProblemClass,
    pub dimension: u32,
    pub num_objectives: u32,
    pub num_constraints: u32,
    pub variable_type_class: BoCoDeVariableTypeClass,
    pub benchmark_set_member: bool,
    pub bounds_state: BoCoDeFieldAvailability,
    pub bounds_sha256: String,
    pub variable_types_state: BoCoDeFieldAvailability,
    pub variable_types_sha256: String,
    pub source_reference_state: BoCoDeSourceReferenceStatus,
    pub source_reference: String,
    pub metadata_sha256: String,
    /// Use "none" when no BOCoDe optional extra is needed.
    pub required_extra: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoCoDeCatalogReport {
    pub schema: String,
    pub context: BoCoDeCatalogContext,
    pub objective_direction: String,
    pub constraint_convention: String,
    pub tasks: Vec<BoCoDeTaskRecord>,
    pub import_sha256: String,
    pub evidence_level: String,
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_token(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\0')
    {
        return Err(format!("{name} must be non-empty and single-line"));
    }
    Ok(())
}

fn validate_context(context: &BoCoDeCatalogContext) -> Result<(), String> {
    if !valid_hex(&context.source_commit, 40) {
        return Err("source_commit must be a full 40-character hexadecimal revision".to_owned());
    }
    validate_token(&context.package_version, "package_version")?;
    if !valid_hex(&context.source_catalog_sha256, 64) {
        return Err("source_catalog_sha256 must be a 64-character SHA-256 digest".to_owned());
    }
    Ok(())
}

fn parse_bool(value: &str, field: &str, line_number: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("line {line_number}: {field} must be true or false")),
    }
}

fn parse_count(value: &str, field: &str, line_number: usize) -> Result<u32, String> {
    value.parse::<u32>().map_err(|error| {
        format!("line {line_number}: {field} must be a non-negative integer: {error}")
    })
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn canonical_source_catalog_bytes(tasks: &[BoCoDeTaskRecord]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for task in tasks {
        append_string(&mut bytes, &task.problem_id);
        append_string(&mut bytes, &task.metadata_sha256);
    }
    bytes
}

fn canonical_import_bytes(context: &BoCoDeCatalogContext, tasks: &[BoCoDeTaskRecord]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        BOCODE_CATALOG_SCHEMA,
        context.source_commit.as_str(),
        context.package_version.as_str(),
        context.source_catalog_sha256.as_str(),
        BOCODE_OBJECTIVE_DIRECTION,
        BOCODE_CONSTRAINT_CONVENTION,
        BOCODE_CATALOG_EVIDENCE_LEVEL,
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, tasks.len() as u64);
    for task in tasks {
        for value in [
            task.problem_id.as_str(),
            task.application.as_str(),
            task.problem_class.as_str(),
            task.variable_type_class.as_str(),
            task.bounds_state.as_str(),
            task.bounds_sha256.as_str(),
            task.variable_types_state.as_str(),
            task.variable_types_sha256.as_str(),
            task.source_reference_state.as_str(),
            task.source_reference.as_str(),
            task.metadata_sha256.as_str(),
            task.required_extra.as_str(),
        ] {
            append_string(&mut bytes, value);
        }
        append_u64(&mut bytes, task.dimension as u64);
        append_u64(&mut bytes, task.num_objectives as u64);
        append_u64(&mut bytes, task.num_constraints as u64);
        bytes.push(u8::from(task.benchmark_set_member));
    }
    bytes
}

fn validate_task(task: &BoCoDeTaskRecord, line_number: usize) -> Result<(), String> {
    validate_token(&task.problem_id, "problem_id")?;
    match task.source_reference_state {
        BoCoDeSourceReferenceStatus::Present => {
            validate_token(&task.source_reference, "source_reference")?;
        }
        BoCoDeSourceReferenceStatus::Missing => {
            if !task.source_reference.is_empty() {
                return Err(format!(
                    "line {line_number}: missing source state requires an empty source_reference"
                ));
            }
        }
    }
    validate_token(&task.required_extra, "required_extra")?;
    if task.dimension == 0 || task.num_objectives == 0 {
        return Err(format!(
            "line {line_number}: dimension and num_objectives must be positive"
        ));
    }
    if !valid_hex(&task.bounds_sha256, 64)
        || !valid_hex(&task.variable_types_sha256, 64)
        || !valid_hex(&task.metadata_sha256, 64)
    {
        return Err(format!(
            "line {line_number}: bounds, variable_types and metadata fields require SHA-256 digests"
        ));
    }
    task.problem_class
        .validate(
            task.num_objectives,
            task.num_constraints,
            task.variable_type_class,
        )
        .map_err(|error| format!("line {line_number}: {error}"))?;
    Ok(())
}

impl BoCoDeCatalogReport {
    pub fn engineering_task_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| {
                task.application == BoCoDeApplication::Engineering && task.benchmark_set_member
            })
            .count()
    }

    pub fn benchmark_set_task_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.benchmark_set_member)
            .count()
    }

    pub fn to_csv(&self) -> String {
        let header = "schema,source_commit,package_version,source_catalog_sha256,objective_direction,constraint_convention,problem_id,application,problem_class,dimension,num_objectives,num_constraints,variable_type_class,benchmark_set_member,bounds_state,bounds_sha256,variable_types_state,variable_types_sha256,source_reference_state,source_reference,metadata_sha256,required_extra,import_sha256,evidence_level";
        let mut output = format!("{header}\n");
        for task in &self.tasks {
            let fields = [
                self.schema.clone(),
                self.context.source_commit.clone(),
                self.context.package_version.clone(),
                self.context.source_catalog_sha256.clone(),
                self.objective_direction.clone(),
                self.constraint_convention.clone(),
                task.problem_id.clone(),
                task.application.as_str().to_owned(),
                task.problem_class.as_str().to_owned(),
                task.dimension.to_string(),
                task.num_objectives.to_string(),
                task.num_constraints.to_string(),
                task.variable_type_class.as_str().to_owned(),
                task.benchmark_set_member.to_string(),
                task.bounds_state.as_str().to_owned(),
                task.bounds_sha256.clone(),
                task.variable_types_state.as_str().to_owned(),
                task.variable_types_sha256.clone(),
                task.source_reference_state.as_str().to_owned(),
                task.source_reference.clone(),
                task.metadata_sha256.clone(),
                task.required_extra.clone(),
                self.import_sha256.clone(),
                self.evidence_level.clone(),
            ];
            output.push_str(
                &fields
                    .iter()
                    .map(|value| format!("\"{}\"", value.replace('"', "\"\"")))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            output.push('\n');
        }
        output
    }
}

/// Import the normalized one-row-per-problem BOCoDe catalog. The raw source-catalog digest is
/// supplied by the external adapter and is not inferred from normalized CSV rows.
pub fn import_bocode_catalog_csv(
    input: &str,
    context: BoCoDeCatalogContext,
) -> Result<BoCoDeCatalogReport, String> {
    validate_context(&context)?;
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "BOCoDe catalog CSV is empty".to_owned())?
        .trim_end_matches('\r');
    if header != BOCODE_CATALOG_CSV_HEADER {
        return Err(format!(
            "BOCoDe catalog header mismatch; expected exactly: {BOCODE_CATALOG_CSV_HEADER}"
        ));
    }

    let mut tasks = Vec::new();
    let mut seen = BTreeSet::new();
    let mut previous: Option<String> = None;
    for (index, raw_line) in lines.enumerate() {
        let line_number = index + 2;
        if raw_line.trim().is_empty() {
            return Err(format!(
                "line {line_number}: blank catalog records are not permitted"
            ));
        }
        let fields = parse_csv_record(raw_line.trim_end_matches('\r'))
            .map_err(|error| format!("line {line_number}: {error}"))?;
        if fields.len() != 16 {
            return Err(format!(
                "line {line_number}: expected 16 catalog columns, got {}",
                fields.len()
            ));
        }
        let dimension = parse_count(&fields[3], "dimension", line_number)?;
        let num_objectives = parse_count(&fields[4], "num_objectives", line_number)?;
        let num_constraints = parse_count(&fields[5], "num_constraints", line_number)?;
        let task = BoCoDeTaskRecord {
            problem_id: fields[0].clone(),
            application: BoCoDeApplication::parse(&fields[1])
                .map_err(|error| format!("line {line_number}: {error}"))?,
            problem_class: BoCoDeProblemClass::parse(&fields[2])
                .map_err(|error| format!("line {line_number}: {error}"))?,
            dimension,
            num_objectives,
            num_constraints,
            variable_type_class: BoCoDeVariableTypeClass::parse(&fields[6])
                .map_err(|error| format!("line {line_number}: {error}"))?,
            benchmark_set_member: parse_bool(&fields[7], "benchmark_set_member", line_number)?,
            bounds_state: BoCoDeFieldAvailability::parse(&fields[8], "bounds")
                .map_err(|error| format!("line {line_number}: {error}"))?,
            bounds_sha256: fields[9].clone(),
            variable_types_state: BoCoDeFieldAvailability::parse(&fields[10], "variable_types")
                .map_err(|error| format!("line {line_number}: {error}"))?,
            variable_types_sha256: fields[11].clone(),
            source_reference_state: BoCoDeSourceReferenceStatus::parse(&fields[12])
                .map_err(|error| format!("line {line_number}: {error}"))?,
            source_reference: fields[13].clone(),
            metadata_sha256: fields[14].clone(),
            required_extra: fields[15].clone(),
        };
        validate_task(&task, line_number)?;
        if !seen.insert(task.problem_id.clone()) {
            return Err(format!(
                "line {line_number}: duplicate problem_id {}",
                task.problem_id
            ));
        }
        if previous
            .as_ref()
            .is_some_and(|value| value >= &task.problem_id)
        {
            return Err(format!(
                "line {line_number}: problem IDs must be in strict lexical order"
            ));
        }
        previous = Some(task.problem_id.clone());
        tasks.push(task);
    }
    if tasks.is_empty() {
        return Err("BOCoDe catalog must contain at least one problem".to_owned());
    }
    let source_catalog_sha256 = sha256_hex(&canonical_source_catalog_bytes(&tasks))?;
    if source_catalog_sha256 != context.source_catalog_sha256 {
        return Err(
            "source catalog digest does not match the task metadata digest inventory".to_owned(),
        );
    }
    let import_sha256 = sha256_hex(&canonical_import_bytes(&context, &tasks))?;
    Ok(BoCoDeCatalogReport {
        schema: BOCODE_CATALOG_SCHEMA.to_owned(),
        context,
        objective_direction: BOCODE_OBJECTIVE_DIRECTION.to_owned(),
        constraint_convention: BOCODE_CONSTRAINT_CONVENTION.to_owned(),
        tasks,
        import_sha256,
        evidence_level: BOCODE_CATALOG_EVIDENCE_LEVEL.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_catalog_sha256(input: &str) -> String {
        let mut lines = input.lines();
        let _header = lines.next().expect("fixture header");
        let mut tasks = Vec::new();
        for line in lines {
            let fields = parse_csv_record(line).expect("fixture CSV record");
            tasks.push((fields[0].clone(), fields[14].clone()));
        }
        tasks.sort_by(|left, right| left.0.cmp(&right.0));
        let mut bytes = Vec::new();
        for (problem_id, metadata_sha256) in tasks {
            append_string(&mut bytes, &problem_id);
            append_string(&mut bytes, &metadata_sha256);
        }
        sha256_hex(&bytes).expect("hash catalog inventory")
    }

    fn context() -> BoCoDeCatalogContext {
        BoCoDeCatalogContext {
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            package_version: "1.0.2.dev2".to_owned(),
            source_catalog_sha256: source_catalog_sha256(&fixture()),
        }
    }

    fn row(
        id: &str,
        domain: &str,
        class: &str,
        objectives: u32,
        constraints: u32,
        variable_types: &str,
    ) -> String {
        format!(
            "{id},{domain},{class},4,{objectives},{constraints},{variable_types},true,present,{},present,{},present,https://example.org/source,{},none",
            "b".repeat(64),
            "c".repeat(64),
            "d".repeat(64)
        )
    }

    fn fixture() -> String {
        format!(
            "{BOCODE_CATALOG_CSV_HEADER}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            row("car", "engineering", "so_mixed_variable", 1, 10, "mixed"),
            row(
                "continuous_constrained",
                "engineering",
                "so_constrained",
                1,
                3,
                "continuous"
            ),
            row("cre21", "engineering", "mo_constrained", 2, 3, "continuous"),
            row("hoip", "engineering", "so_unconstrained", 1, 0, "discrete"),
            row(
                "re21",
                "engineering",
                "mo_unconstrained",
                2,
                0,
                "continuous"
            ),
            row(
                "truss",
                "engineering",
                "so_unconstrained",
                1,
                0,
                "continuous"
            ),
            row(
                "truss_mixed",
                "engineering",
                "so_mixed_variable",
                1,
                0,
                "mixed"
            )
        )
    }

    #[test]
    fn imports_all_five_documented_problem_classes_and_fixed_semantics() {
        let report = import_bocode_catalog_csv(&fixture(), context()).unwrap();
        assert_eq!(report.tasks.len(), 7);
        assert_eq!(report.benchmark_set_task_count(), 7);
        assert_eq!(report.engineering_task_count(), 7);
        assert_eq!(report.objective_direction, "maximize");
        assert_eq!(report.constraint_convention, "g(x)<=0_is_feasible");
        let car = report
            .tasks
            .iter()
            .find(|task| task.problem_id == "car")
            .unwrap();
        assert_eq!(
            car.problem_class,
            BoCoDeProblemClass::SingleObjectiveMixedVariable
        );
        assert_eq!(car.num_constraints, 10);
        let hoip = report
            .tasks
            .iter()
            .find(|task| task.problem_id == "hoip")
            .unwrap();
        assert_eq!(
            hoip.problem_class,
            BoCoDeProblemClass::SingleObjectiveUnconstrained
        );
        assert_eq!(hoip.variable_type_class, BoCoDeVariableTypeClass::Discrete);
        assert_eq!(report.to_csv().lines().count(), 8);
        let repeated = import_bocode_catalog_csv(&fixture(), context()).unwrap();
        assert_eq!(report.import_sha256, repeated.import_sha256);
    }

    #[test]
    fn preserves_unavailable_bounds_and_variable_types_without_guessing() {
        let mut fields = parse_csv_record(&row(
            "mazda",
            "engineering",
            "mo_constrained",
            5,
            54,
            "continuous",
        ))
        .unwrap();
        fields[8] = "unavailable".to_owned();
        fields[9] = sha256_hex(b"null").unwrap();
        fields[10] = "unavailable".to_owned();
        fields[11] = sha256_hex(b"null").unwrap();
        let catalog = format!("{BOCODE_CATALOG_CSV_HEADER}\n{}\n", fields.join(","));
        let report = import_bocode_catalog_csv(
            &catalog,
            BoCoDeCatalogContext {
                source_commit: context().source_commit,
                package_version: context().package_version,
                source_catalog_sha256: source_catalog_sha256(&catalog),
            },
        )
        .expect("unavailable fields remain explicit rather than guessed");
        assert_eq!(
            report.tasks[0].bounds_state,
            BoCoDeFieldAvailability::Unavailable
        );
        assert_eq!(
            report.tasks[0].variable_types_state,
            BoCoDeFieldAvailability::Unavailable
        );
    }

    #[test]
    fn rejects_source_catalog_digest_inventory_mismatch() {
        let mut bad = context();
        bad.source_catalog_sha256 = "f".repeat(64);
        assert!(
            import_bocode_catalog_csv(&fixture(), bad)
                .unwrap_err()
                .contains("source catalog digest")
        );
    }

    #[test]
    fn rejects_class_contradictions_unknown_domains_and_bad_digests() {
        let contradicted = format!(
            "{BOCODE_CATALOG_CSV_HEADER}\n{}\n",
            row("bad", "engineering", "so_unconstrained", 1, 1, "continuous")
        );
        assert!(
            import_bocode_catalog_csv(&contradicted, context())
                .unwrap_err()
                .contains("contradicts")
        );

        let unknown = format!(
            "{BOCODE_CATALOG_CSV_HEADER}\n{}\n",
            row("bad", "chemistry", "so_unconstrained", 1, 0, "continuous")
        );
        assert!(import_bocode_catalog_csv(&unknown, context()).is_err());

        let bad_digest = format!(
            "{BOCODE_CATALOG_CSV_HEADER}\n{}\n",
            row("bad", "engineering", "so_unconstrained", 1, 0, "continuous")
                .replace(&"c".repeat(64), "not-a-digest")
        );
        assert!(import_bocode_catalog_csv(&bad_digest, context()).is_err());
    }

    #[test]
    fn rejects_duplicate_or_unsorted_ids_and_missing_citations() {
        let duplicate = format!(
            "{BOCODE_CATALOG_CSV_HEADER}\n{}\n{}\n",
            row("car", "engineering", "so_unconstrained", 1, 0, "continuous"),
            row("car", "engineering", "so_unconstrained", 1, 0, "continuous")
        );
        assert!(
            import_bocode_catalog_csv(&duplicate, context())
                .unwrap_err()
                .contains("duplicate")
        );

        let unsorted = format!(
            "{BOCODE_CATALOG_CSV_HEADER}\n{}\n{}\n",
            row(
                "zebra",
                "engineering",
                "so_unconstrained",
                1,
                0,
                "continuous"
            ),
            row(
                "alpha",
                "engineering",
                "so_unconstrained",
                1,
                0,
                "continuous"
            )
        );
        assert!(
            import_bocode_catalog_csv(&unsorted, context())
                .unwrap_err()
                .contains("lexical order")
        );

        let missing_source = format!(
            "{BOCODE_CATALOG_CSV_HEADER}\n{}\n",
            row("car", "engineering", "so_unconstrained", 1, 0, "continuous")
                .replace(",present,https://example.org/source,", ",missing,,")
        );
        let missing_source_report = import_bocode_catalog_csv(
            &missing_source,
            BoCoDeCatalogContext {
                source_commit: context().source_commit,
                package_version: context().package_version,
                source_catalog_sha256: source_catalog_sha256(&missing_source),
            },
        )
        .expect("missing upstream citations are retained explicitly");
        assert_eq!(
            missing_source_report.tasks[0].source_reference_state,
            BoCoDeSourceReferenceStatus::Missing
        );
    }

    #[test]
    fn rejects_incomplete_and_malformed_catalog_context() {
        let mut bad = context();
        bad.source_commit = "main".to_owned();
        assert!(import_bocode_catalog_csv(&fixture(), bad).is_err());

        let wrong_version = fixture().replace(BOCODE_CATALOG_CSV_HEADER, "id,domain");
        assert!(import_bocode_catalog_csv(&wrong_version, context()).is_err());

        let extra_column = fixture().replace(",none\n", ",none,unexpected\n");
        assert!(import_bocode_catalog_csv(&extra_column, context()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn independent_python_oracle_replays_catalog_and_tamper_control() {
        let report = import_bocode_catalog_csv(&fixture(), context()).unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("bocode-catalog-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let csv_path = root.join("catalog.csv");
        std::fs::write(&csv_path, fixture()).unwrap();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/verify-bocode-catalog-independent.py");
        let output = std::process::Command::new("python3")
            .arg(&script)
            .arg(&csv_path)
            .arg(&context().source_commit)
            .arg(&context().package_version)
            .arg(&context().source_catalog_sha256)
            .arg(&report.import_sha256)
            .output()
            .expect("execute independent BOCoDe catalog oracle");
        assert!(
            output.status.success(),
            "independent BOCoDe catalog oracle failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let tampered = fixture().replace("truss,engineering", "truss,synthetic");
        std::fs::write(&csv_path, tampered).unwrap();
        let rejected = std::process::Command::new("python3")
            .arg(&script)
            .arg(&csv_path)
            .arg(&context().source_commit)
            .arg(&context().package_version)
            .arg(&context().source_catalog_sha256)
            .arg(&report.import_sha256)
            .output()
            .expect("execute negative-control catalog oracle");
        assert!(!rejected.status.success());
        std::fs::remove_dir_all(root).unwrap();
    }
}
