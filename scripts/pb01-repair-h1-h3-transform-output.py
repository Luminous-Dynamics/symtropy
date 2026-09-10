#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Normalize the H1/H3 transformer join-dependency block before compilation."""

from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
path = root / "crates/domains/symtropy-player-building/src/lib.rs"
text = path.read_text()

pattern = re.compile(
    r"        for operation in &self\.operations \{\n"
    r"            if let ConstructionAction::JoinElements \{.*?"
    r"\n        topological_order\(&self\.operations\)\.map\(\|_\| \(\)\)",
    re.DOTALL,
)

replacement = '''        for operation in &self.operations {
            match &operation.action {
                ConstructionAction::JoinElements {
                    first_element_id,
                    second_element_id,
                    ..
                } => {
                    let first_realization = realization_by_element.get(first_element_id).ok_or_else(|| {
                        ProposalError::JoinElementNotRealized(first_element_id.clone())
                    })?;
                    let second_realization = realization_by_element.get(second_element_id).ok_or_else(|| {
                        ProposalError::JoinElementNotRealized(second_element_id.clone())
                    })?;
                    if operation.depends_on().binary_search(first_realization).is_err() {
                        return Err(ProposalError::JoinMissingRealizationDependency {
                            join_operation_id: operation.operation_id.clone(),
                            realization_operation_id: first_realization.clone(),
                        });
                    }
                    if operation.depends_on().binary_search(second_realization).is_err() {
                        return Err(ProposalError::JoinMissingRealizationDependency {
                            join_operation_id: operation.operation_id.clone(),
                            realization_operation_id: second_realization.clone(),
                        });
                    }
                }
                ConstructionAction::JoinElementToExactSubject { element_id, .. } => {
                    let realization = realization_by_element.get(element_id).ok_or_else(|| {
                        ProposalError::JoinElementNotRealized(element_id.clone())
                    })?;
                    if operation.depends_on().binary_search(realization).is_err() {
                        return Err(ProposalError::JoinMissingRealizationDependency {
                            join_operation_id: operation.operation_id.clone(),
                            realization_operation_id: realization.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        topological_order(&self.operations).map(|_| ())'''

text, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit(f"join dependency normalization expected 1 match, found {count}")
path.write_text(text)
print(f"normalized {path.relative_to(root)} join dependency validation")
