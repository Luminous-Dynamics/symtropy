use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct DeviceBusEvent {
    pub source: String,
    pub topic: String,
    pub payload: String,
}

#[derive(Resource, Default)]
pub struct DeviceBus {
    pub history: Vec<DeviceBusEvent>,
}

impl DeviceBus {
    pub fn broadcast(&mut self, event: DeviceBusEvent) {
        self.history.push(event);
    }
}
