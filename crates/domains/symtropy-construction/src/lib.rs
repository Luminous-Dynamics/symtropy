// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-backed construction orchestration for Symtropy.
//!
//! This crate owns site/work/staging orchestration truth. It does not own
//! conserved matter, structural physics, fabrication process truth, technical
//! commissioning, Device Bus registration, or civic authorization.

mod orchestration;
mod site;
mod staging;
mod work_order;

pub use orchestration::*;
pub use site::*;
pub use staging::*;
pub use work_order::{
    WorkActorRef, WorkOrderError, WorkOrderId, WorkOrderLifecycle, WorkOrderStatus,
};

// C2 remains an internal primitive after C3. External code receives
// `ScheduledWorkOrder`, whose release path is staging-gated.
pub(crate) use work_order::WorkOrder;
