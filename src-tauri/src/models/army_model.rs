use serde::{Deserialize, Serialize};
use super::tile_model::{Direction, TileCoord};
use super::military_model::MilitaryRoster;
use super::commander_model::CommanderRoster;
use super::scout_model::ScoutRoster;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArmyMission {
    Garrison,           // Defend a location
    Patrol,             // Watch an area
    Assault,            // Attack objective
    Raid,               // Hit and run
    Siege,              // Reduce fortification
    Escort,             // Protect something moving
    Reserve,            // Ready to reinforce
}

/// An army is a task organization grouping units for a purpose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Army {
    pub id: u32,
    pub name: String,                    // "Northern Expeditionary Force"
    pub faction: String,

    // Leadership
    pub commander_id: Option<u32>,       // Reference to Commander

    // Composition - military units (SEPARATE from scouts)
    pub unit_ids: Vec<u32>,              // MilitaryUnit references

    // Attached scouts (for real-time intel bonus)
    pub attached_scout_ids: Vec<u32>,    // ScoutUnit references

    // Organization
    pub mission: ArmyMission,

    // Location
    pub location: (TileCoord, Direction),
    pub destination: Option<(TileCoord, Direction)>,
    pub movement_progress: f32,

    // Aggregates (computed from units)
    pub total_strength: i32,
    pub movement_speed: i32,             // Limited by slowest unit

    // State
    pub cohesion: f32,                   // How well units work together (0.0-1.0)
    pub fatigue: f32,                    // Army-wide exhaustion
    pub days_without_supply: i32,
}

impl Army {
    /// INHERENT scouting capability (weak - armies are not scouts)
    /// Capped at 40 (scouts can reach 100)
    pub fn inherent_scouting(
        &self,
        roster: &MilitaryRoster,
        commanders: &CommanderRoster,
    ) -> i32 {
        // Average perception of units (military training gives SOME awareness)
        let avg_perception: i32 = if self.unit_ids.is_empty() {
            0
        } else {
            let total: i32 = self.unit_ids.iter()
                .filter_map(|id| roster.get_unit(*id))
                .map(|u| u.stats.perception)
                .sum();
            total / self.unit_ids.len() as i32
        };

        // Inherent scouting CAPPED at 40
        let base_scouting = (avg_perception / 3).clamp(0, 40);

        // Commander bonus
        let commander_bonus = self.commander_id
            .and_then(|id| commanders.get(id))
            .map(|c| c.scouting_bonus())
            .unwrap_or(0);

        (base_scouting + commander_bonus).clamp(0, 40)
    }

    /// ENHANCED scouting from attached scouts (stacks with inherent)
    pub fn total_scouting(
        &self,
        military_roster: &MilitaryRoster,
        commanders: &CommanderRoster,
        scout_roster: &ScoutRoster,
    ) -> i32 {
        let inherent = self.inherent_scouting(military_roster, commanders);

        if self.attached_scout_ids.is_empty() {
            return inherent;
        }

        // Attached scouts add their perception (best scout matters most)
        let best_scout_perception: i32 = self.attached_scout_ids.iter()
            .filter_map(|id| scout_roster.get_scout(*id))
            .map(|s| s.perception)
            .max()
            .unwrap_or(0);

        // Inherent + attached (no cap on total with scouts)
        inherent + best_scout_perception
    }

    /// Attach a scout to this army
    pub fn attach_scout(&mut self, scout_id: u32) {
        if !self.attached_scout_ids.contains(&scout_id) {
            self.attached_scout_ids.push(scout_id);
        }
    }

    /// Detach a scout for independent operation
    pub fn detach_scout(&mut self, scout_id: u32) -> bool {
        if let Some(pos) = self.attached_scout_ids.iter().position(|&id| id == scout_id) {
            self.attached_scout_ids.remove(pos);
            true
        } else {
            false
        }
    }

    /// Attach a military unit
    pub fn attach_unit(&mut self, unit_id: u32) {
        if !self.unit_ids.contains(&unit_id) {
            self.unit_ids.push(unit_id);
        }
    }

    /// Detach a military unit
    pub fn detach_unit(&mut self, unit_id: u32) -> bool {
        if let Some(pos) = self.unit_ids.iter().position(|&id| id == unit_id) {
            self.unit_ids.remove(pos);
            true
        } else {
            false
        }
    }

    /// Recalculate aggregates from current units
    pub fn recalculate_aggregates(&mut self, roster: &MilitaryRoster) {
        self.total_strength = self.unit_ids.iter()
            .filter_map(|id| roster.get_unit(*id))
            .map(|u| u.current_strength)
            .sum();

        // Movement speed is limited by slowest unit
        self.movement_speed = self.unit_ids.iter()
            .filter_map(|id| roster.get_unit(*id))
            .map(|u| u.stats.mobility)
            .min()
            .unwrap_or(0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmyRoster {
    pub faction: String,
    pub armies: Vec<Army>,
    pub next_id: u32,
}

impl ArmyRoster {
    pub fn new(faction: String) -> Self {
        Self {
            faction,
            armies: Vec::new(),
            next_id: 1,
        }
    }

    pub fn get_army(&self, id: u32) -> Option<&Army> {
        self.armies.iter().find(|a| a.id == id)
    }

    pub fn get_army_mut(&mut self, id: u32) -> Option<&mut Army> {
        self.armies.iter_mut().find(|a| a.id == id)
    }

    pub fn create_army(
        &mut self,
        name: String,
        location: (TileCoord, Direction),
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.armies.push(Army {
            id,
            name,
            faction: self.faction.clone(),
            commander_id: None,
            unit_ids: Vec::new(),
            attached_scout_ids: Vec::new(),
            mission: ArmyMission::Reserve,
            location,
            destination: None,
            movement_progress: 0.0,
            total_strength: 0,
            movement_speed: 0,
            cohesion: 1.0,
            fatigue: 0.0,
            days_without_supply: 0,
        });

        id
    }
}
