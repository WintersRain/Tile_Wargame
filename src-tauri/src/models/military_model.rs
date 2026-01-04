use serde::{Deserialize, Serialize};
use super::tile_model::{Direction, TileCoord};
use crate::engine::combat::UnitStats;

/// What TYPE of military capability this unit provides
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitType {
    // Infantry types
    Militia,            // Cheap, low morale, recruited quickly
    LineInfantry,       // Standard soldiers
    HeavyInfantry,      // Armored, slow, high melee

    // Ranged types
    Archers,            // Fast fire, low penetration
    Crossbowmen,        // Slow fire, high penetration
    Skirmishers,        // Light, mobile, harassing

    // Mounted types
    LightCavalry,       // Fast, flanking, pursuit
    HeavyCavalry,       // Shock, charge bonus
    MountedArchers,     // Mobile harassment

    // Specialist types
    Engineers,          // Siege, construction bonus
    Artillery,          // Siege weapons, slow
}

/// What the unit is currently doing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnitAssignment {
    Garrison(TileCoord, Direction),     // Defending a section
    Patrol {                             // Moving between sections
        route: Vec<(TileCoord, Direction)>,
        current_index: usize,
    },
    Ambush(TileCoord, Direction),       // Hidden, waiting to strike
    Moving {                             // Traveling to destination
        from: (TileCoord, Direction),
        to: (TileCoord, Direction),
        progress: f32,                   // 0.0 to 1.0
    },
    Attached(u32),                       // Part of an Army (army_id)
    Recovering,                          // R&R, regaining endurance
}

/// A discrete military unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilitaryUnit {
    pub id: u32,
    pub name: String,                    // "1st Riverside Spears"
    pub faction: String,

    // Composition
    pub unit_type: UnitType,
    pub current_strength: i32,           // Current soldiers (takes casualties)
    pub max_strength: i32,               // Full strength (e.g., 100)
    pub experience: i32,                 // 0-1000, affects effectiveness

    // Equipment (determines base stats)
    pub equipment_tier: i32,             // 1-5, affects armor/weapons
    pub special_equipment: Vec<String>,  // "Crossbows", "Plate Armor"

    // Computed stats (from unit_type + equipment + experience)
    pub stats: UnitStats,

    // Spatial existence
    pub assignment: UnitAssignment,

    // State
    pub morale: i32,                     // Will to fight
    pub days_without_supply: i32,        // Attrition clock
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilitaryRoster {
    pub faction: String,
    pub units: Vec<MilitaryUnit>,
    pub next_id: u32,
}

impl MilitaryRoster {
    pub fn new(faction: String) -> Self {
        Self {
            faction,
            units: Vec::new(),
            next_id: 1,
        }
    }

    pub fn get_unit(&self, id: u32) -> Option<&MilitaryUnit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn get_unit_mut(&mut self, id: u32) -> Option<&mut MilitaryUnit> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    pub fn get_units_at_location(&self, loc: (TileCoord, Direction)) -> Vec<&MilitaryUnit> {
        self.units.iter().filter(|u| {
            match &u.assignment {
                UnitAssignment::Garrison(t, d) => *t == loc.0 && *d == loc.1,
                UnitAssignment::Ambush(t, d) => *t == loc.0 && *d == loc.1,
                _ => false,
            }
        }).collect()
    }

    pub fn create_unit(
        &mut self,
        name: String,
        unit_type: UnitType,
        max_strength: i32,
        stats: UnitStats,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.units.push(MilitaryUnit {
            id,
            name,
            faction: self.faction.clone(),
            unit_type,
            current_strength: max_strength,
            max_strength,
            experience: 0,
            equipment_tier: 1,
            special_equipment: Vec::new(),
            stats,
            assignment: UnitAssignment::Recovering,
            morale: 100,
            days_without_supply: 0,
        });

        id
    }
}
