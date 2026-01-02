use serde::{Deserialize, Serialize};
use super::tile_model::{Direction, TileCoord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoutMode {
    Observe,    // Passive intel gathering
    Probe,      // Active infiltration attempt
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportingMethod {
    Return,     // Must physically return home
    Hawk,       // Can send message mid-mission
    MagicBond,  // Instant mental report
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutUnit {
    pub id: u32,
    pub name: String,
    pub faction: String,

    // Core stats
    pub stealth: i32,           // Concealment capability (0-100)
    pub perception: i32,        // Detection capability (0-100)
    pub mobility: i32,          // Movement speed (sections per turn)

    // Assignment
    pub assigned_location: Option<(TileCoord, Direction)>,
    pub mode: ScoutMode,
    pub reporting_method: ReportingMethod,

    // Status
    pub is_deployed: bool,
    pub turns_observing: i32,   // Time spent at current location
    pub has_returned: bool,     // For Return-based reporting
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutRoster {
    pub faction: String,
    pub scouts: Vec<ScoutUnit>,
    pub next_id: u32,
}

impl ScoutRoster {
    pub fn new(faction: String) -> Self {
        Self {
            faction,
            scouts: Vec::new(),
            next_id: 1,
        }
    }

    pub fn create_scout(&mut self, name: String, stealth: i32, perception: i32, mobility: i32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.scouts.push(ScoutUnit {
            id,
            name,
            faction: self.faction.clone(),
            stealth,
            perception,
            mobility,
            assigned_location: None,
            mode: ScoutMode::Observe,
            reporting_method: ReportingMethod::Return,
            is_deployed: false,
            turns_observing: 0,
            has_returned: false,
        });

        id
    }
}
