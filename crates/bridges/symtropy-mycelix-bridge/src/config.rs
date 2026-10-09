// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::Resource;

/// Runtime configuration for the Bevy ↔ Mycelix bridge.
///
/// The bridge runs `mycelix-conductor-bridge` (from the monorepo) as a child
/// process and speaks JSON over stdin/stdout. This keeps the Holochain
/// dependency graph (serde=1.0.203 exact-pinned by `holochain_client`) out
/// of the Bevy compilation unit entirely — no serde conflict.
///
/// # Prerequisites
///
/// 1. `mycelix-conductor-bridge` built and on `PATH`, OR path supplied via
///    [`Self::bridge_binary`].
/// 2. A running Holochain conductor reachable at [`Self::conductor_url`]
///    with the target hApp installed.
/// 3. `MYCELIX_APP_TOKEN` env var set (the bridge subprocess inherits it).
#[derive(Debug, Clone, Resource)]
pub struct MycelixConfig {
    /// Path to the `mycelix-conductor-bridge` executable. Resolved against
    /// `PATH` when it's a bare name.
    pub bridge_binary: PathBuf,
    /// Passed to the subprocess as `--conductor-url`.
    pub conductor_url: String,
    /// Passed to the subprocess as `--app-id`.
    pub app_id: String,
    /// Passed to the subprocess as `--role`.
    pub role: String,
    /// Maximum accepted operations across the request queue, dispatched
    /// subprocess calls, and responses awaiting transfer into Bevy's message
    /// queue. When exhausted, [`crate::MycelixClient::send`] returns an error
    /// rather than blocking the Bevy schedule.
    pub inflight_budget: usize,
    /// Maximum time from successful stdin flush to response acceptance.
    /// Expiry fences the subprocess generation; it does not prove a remote
    /// mutation was rejected. Zero is normalized to one millisecond.
    pub response_timeout: Duration,
}

impl Default for MycelixConfig {
    fn default() -> Self {
        Self {
            bridge_binary: PathBuf::from("mycelix-conductor-bridge"),
            conductor_url: "ws://localhost:8888".to_string(),
            app_id: "mycelix-governance".to_string(),
            role: "governance".to_string(),
            inflight_budget: 128,
            response_timeout: Duration::from_secs(30),
        }
    }
}

impl MycelixConfig {
    /// Effective end-to-end admission budget.
    ///
    /// Zero is normalized to one so the bounded channels and semaphore retain
    /// a consistent, non-deadlocking minimum capacity.
    #[must_use]
    pub fn effective_inflight_budget(&self) -> usize {
        self.inflight_budget.max(1)
    }

    /// Override the bridge binary path.
    pub fn with_bridge_binary(mut self, path: impl Into<PathBuf>) -> Self {
        self.bridge_binary = path.into();
        self
    }

    /// Override the conductor URL.
    pub fn with_conductor_url(mut self, url: impl Into<String>) -> Self {
        self.conductor_url = url.into();
        self
    }

    /// Override the app id.
    pub fn with_app_id(mut self, id: impl Into<String>) -> Self {
        self.app_id = id.into();
        self
    }

    /// Override the role name.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    /// Override the inflight request budget.
    pub fn with_inflight_budget(mut self, budget: usize) -> Self {
        self.inflight_budget = budget;
        self
    }

    /// Effective finite deadline for a dispatched request.
    ///
    /// A zero duration is normalized to one millisecond so this setting can
    /// never silently disable response-deadline enforcement.
    #[must_use]
    pub fn effective_response_timeout(&self) -> Duration {
        self.response_timeout.max(Duration::from_millis(1))
    }

    /// Override the response deadline measured from successful stdin flush.
    pub fn with_response_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = timeout;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::MycelixConfig;
    use std::time::Duration;

    #[test]
    fn inflight_budget_has_a_nonzero_effective_capacity() {
        assert_eq!(
            MycelixConfig::default()
                .with_inflight_budget(0)
                .effective_inflight_budget(),
            1
        );
        assert_eq!(
            MycelixConfig::default()
                .with_inflight_budget(16)
                .effective_inflight_budget(),
            16
        );
        assert_eq!(
            MycelixConfig::default().effective_response_timeout(),
            Duration::from_secs(30)
        );
        assert_eq!(
            MycelixConfig::default()
                .with_response_timeout(Duration::ZERO)
                .effective_response_timeout(),
            Duration::from_millis(1)
        );
    }
}
