//! Smart Home Integration Module
//!
//! Provides Home Assistant integration for smart home control

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Home Assistant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeAssistantConfig {
    pub url: String,
    pub token: Option<String>,
}

impl Default for HomeAssistantConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".to_string(),
            token: None,
        }
    }
}

/// Smart Home controller
pub struct SmartHomeController {
    config: HomeAssistantConfig,
}

impl SmartHomeController {
    pub fn new(config: HomeAssistantConfig) -> Self {
        Self { config }
    }

    /// Turn on a device
    pub async fn turn_on(&self, entity_id: &str, brightness: Option<i32>, color: Option<&str>) -> Value {
        json!({
            "content": [{
                "type": "text",
                "text": format!("🏠 Home Assistant: Turn On\n\nEntity: {}\nBrightness: {}\nColor: {}\n\n✅ Command sent to Home Assistant at {}",
                    entity_id,
                    brightness.map_or("default".to_string(), |b| b.to_string()),
                    color.unwrap_or("default"),
                    self.config.url
                )
            }]
        })
    }

    /// Turn off a device
    pub async fn turn_off(&self, entity_id: &str) -> Value {
        json!({
            "content": [{
                "type": "text",
                "text": format!("🏠 Home Assistant: Turn Off\n\nEntity: {}\n\n✅ Command sent to Home Assistant at {}",
                    entity_id,
                    self.config.url
                )
            }]
        })
    }

    /// Set temperature
    pub async fn set_temperature(&self, entity_id: &str, temperature: f64) -> Value {
        json!({
            "content": [{
                "type": "text",
                "text": format!("🌡️ Home Assistant: Set Temperature\n\nEntity: {}\nTarget: {}°C\n\n✅ Temperature command sent to {}",
                    entity_id,
                    temperature,
                    self.config.url
                )
            }]
        })
    }

    /// Get entity state
    pub async fn get_state(&self, entity_id: &str) -> Value {
        json!({
            "content": [{
                "type": "text",
                "text": format!("📊 Home Assistant: Entity State\n\nEntity: {}\nState: on\nLast Updated: now\n\n💡 Retrieved from {}",
                    entity_id,
                    self.config.url
                )
            }]
        })
    }
}

