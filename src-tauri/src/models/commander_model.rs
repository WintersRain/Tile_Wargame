use serde::{Deserialize, Serialize};
use super::tile_model::{Direction, TileCoord};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommanderTrait {
    // Positive
    Aggressive,       // +20% attack, -10% defense
    Cautious,         // +20% ambush detection, -10% pursuit
    Inspiring,        // +15% morale to led units
    Cunning,          // +25% ambush effectiveness
    Veteran,          // Units under command gain XP faster

    // Negative (earned through failures)
    Reckless,         // -20% ambush detection
    Hesitant,         // -15% initiative
    Cruel,            // +10% enemy morale when fighting against
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommanderAssignment {
    Unassigned,
    LeadingArmy(u32),                     // army_id
    Garrison(TileCoord, Direction),       // Defending personally
    Recovering,                            // Wounded, resting
}

/// A named character who leads military forces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commander {
    pub id: u32,
    pub name: String,
    pub faction: String,

    // Experience & Growth
    pub experience: i32,              // 0-1000 (DF-style scale)
    pub battles_fought: i32,
    pub victories: i32,

    // Leadership stats
    pub tactics: i32,                 // Battle initiative, flanking
    pub logistics: i32,               // Supply consumption, movement
    pub inspiration: i32,             // Morale recovery, rout prevention
    pub cunning: i32,                 // Ambush detection, trap setting

    // Traits (earned through events)
    pub traits: Vec<CommanderTrait>,

    // Assignment
    pub assignment: CommanderAssignment,

    // State
    pub is_alive: bool,
    pub location: Option<(TileCoord, Direction)>,
}

impl Commander {
    /// Calculate command capacity from experience
    /// More experience = can lead more units
    pub fn command_capacity(&self) -> i32 {
        match self.experience {
            0..=99 => 1,      // Green: 1 unit
            100..=249 => 2,   // Novice: 2 units
            250..=399 => 3,   // Competent: 3 units
            400..=549 => 4,
            550..=699 => 5,
            700..=799 => 6,
            800..=899 => 7,
            900..=949 => 8,
            950..=989 => 9,
            990..=1000 => 10, // Legendary: 10 units
            _ => 1,
        }
    }

    /// Scouting bonus this commander provides to army
    pub fn scouting_bonus(&self) -> i32 {
        let base = self.cunning / 5;
        let trait_bonus: i32 = self.traits.iter().map(|t| match t {
            CommanderTrait::Cunning => 10,
            CommanderTrait::Cautious => 5,
            CommanderTrait::Reckless => -10,
            _ => 0,
        }).sum();

        base + trait_bonus
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommanderRoster {
    pub faction: String,
    pub commanders: Vec<Commander>,
    pub next_id: u32,
}

impl CommanderRoster {
    pub fn new(faction: String) -> Self {
        Self {
            faction,
            commanders: Vec::new(),
            next_id: 1,
        }
    }

    pub fn get(&self, id: u32) -> Option<&Commander> {
        self.commanders.iter().find(|c| c.id == id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Commander> {
        self.commanders.iter_mut().find(|c| c.id == id)
    }

    pub fn create_commander(
        &mut self,
        name: String,
        tactics: i32,
        logistics: i32,
        inspiration: i32,
        cunning: i32,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.commanders.push(Commander {
            id,
            name,
            faction: self.faction.clone(),
            experience: 0,
            battles_fought: 0,
            victories: 0,
            tactics,
            logistics,
            inspiration,
            cunning,
            traits: Vec::new(),
            assignment: CommanderAssignment::Unassigned,
            is_alive: true,
            location: None,
        });

        id
    }
}
