// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksPlugin;
use tokio::sync::Semaphore;

use crate::config::MycelixConfig;
use crate::events::MycelixResponse;
use crate::resource::{MycelixClient, MycelixRequestOutbox, MycelixResponseInbox};
use crate::systems::{pump_responses, spawn_dispatcher_task};

/// Bevy plugin that wires up the Mycelix bridge.
///
/// Adds [`TokioTasksPlugin`] (if not already present), inserts
/// [`MycelixConfig`] and [`MycelixClient`] resources, registers
/// [`MycelixResponse`] as a Bevy event, and registers the
/// startup + pump systems.
pub struct BevyMycelixPlugin {
    pub config: MycelixConfig,
}

impl BevyMycelixPlugin {
    /// Construct with a given [`MycelixConfig`].
    pub fn new(config: MycelixConfig) -> Self {
        Self { config }
    }
}

impl Default for BevyMycelixPlugin {
    fn default() -> Self {
        Self::new(MycelixConfig::default())
    }
}

impl Plugin for BevyMycelixPlugin {
    fn build(&self, app: &mut App) {
        // Bring in bevy-tokio-tasks if the host app hasn't already.
        if !app.is_plugin_added::<TokioTasksPlugin>() {
            app.add_plugins(TokioTasksPlugin::default());
        }

        // Bounded channels so a saturated conductor can't OOM the game. Keep
        // their capacity aligned with the dispatcher's in-flight semaphore.
        let inflight_budget = self.config.effective_inflight_budget();
        if self.config.inflight_budget == 0 {
            warn!("symtropy-mycelix-bridge: inflight_budget=0 normalized to 1");
        }
        let (req_tx, req_rx) = flume::bounded(inflight_budget);
        let (resp_tx, resp_rx) = flume::bounded(inflight_budget);
        // One shared credit budget covers queued requests, dispatched calls,
        // and responses awaiting transfer into Bevy's message queue.
        let admission = Arc::new(Semaphore::new(inflight_budget));

        app.insert_resource(self.config.clone())
            .insert_resource(MycelixClient::new(req_tx, admission))
            .insert_resource(MycelixResponseInbox { rx: resp_rx })
            .insert_resource(MycelixRequestOutbox {
                rx: Some(req_rx),
                response_tx: Some(resp_tx),
            })
            .add_message::<MycelixResponse>()
            .add_systems(Startup, spawn_dispatcher_task)
            .add_systems(Update, pump_responses);
    }
}
