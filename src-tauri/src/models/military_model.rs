use serde::{Deserialize, Serialize};
use super::tile_model::{Direction, TileCoord};

// ═══════════════════════════════════════════════════════════════════
// COMBAT MODE (for detachment system)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatMode {
    /// Not operating as detachment - standard army unit
    None,
    /// Watch and report, avoid contact, withdraw if approached
    Observation,
    /// Standoff harassment, engage then fade, maintain distance
    Sniping,
    /// Close ambush, strike then disengage if possible
    Guerilla,
    /// Close ambush, commit to destruction
    Raiding,
}

impl Default for CombatMode {
    fn default() -> Self {
        CombatMode::None
    }
}

// ═══════════════════════════════════════════════════════════════════
// REPORTING METHOD (for scouting/observation intel)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportingMethod {
    /// Must physically return home to report - no mid-mission intel
    Return,
    /// Can send messages via trained hawk - report every few turns
    Hawk,
    /// Instant mental/magical bond - real-time updates
    MagicBond,
}

impl Default for ReportingMethod {
    fn default() -> Self {
        ReportingMethod::Return
    }
}

// ═══════════════════════════════════════════════════════════════════
// UNIT TEMPLATE (defines what a unit type IS - race + role)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTemplate {
    pub id: String,              // "orc_raider", "human_infantry", "elven_ranger"
    pub display_name: String,    // "Orc Raider"
    pub race: String,            // "Orc", "Human", "Elf", "Goblin"

    // Base stats (modified by equipment, experience, upgrades)
    pub base_health: i32,        // Orc: 8+, Human: 5, Goblin: 3
    pub base_melee: i32,
    pub base_accuracy: i32,
    pub base_armor: i32,
    pub base_penetration: i32,
    pub base_morale: i32,
    pub base_mobility: i32,
    pub base_perception: i32,
    pub base_stealth: i32,
    pub base_range: i32,         // 0 = melee only

    // Upgrade capabilities
    pub can_upgrade_health: bool,
    pub available_upgrades: Vec<String>,

    // Detachment modes this unit type can use
    pub available_modes: Vec<CombatMode>,
}

impl UnitTemplate {
    /// Create a basic human infantry template
    pub fn human_infantry() -> Self {
        Self {
            id: "human_infantry".to_string(),
            display_name: "Human Infantry".to_string(),
            race: "Human".to_string(),
            base_health: 5,
            base_melee: 30,
            base_accuracy: 20,
            base_armor: 15,
            base_penetration: 5,
            base_morale: 50,
            base_mobility: 20,
            base_perception: 25,
            base_stealth: 20,
            base_range: 0,
            can_upgrade_health: false,
            available_upgrades: vec!["better_armor".to_string(), "polearms".to_string()],
            available_modes: vec![
                CombatMode::Observation,
                CombatMode::Guerilla,
                CombatMode::Raiding,
            ],
        }
    }

    /// Create a basic orc raider template
    pub fn orc_raider() -> Self {
        Self {
            id: "orc_raider".to_string(),
            display_name: "Orc Raider".to_string(),
            race: "Orc".to_string(),
            base_health: 8,
            base_melee: 45,
            base_accuracy: 10,
            base_armor: 20,
            base_penetration: 10,
            base_morale: 60,
            base_mobility: 25,
            base_perception: 20,
            base_stealth: 15,
            base_range: 0,
            can_upgrade_health: true,
            available_upgrades: vec!["brutal_weapons".to_string(), "war_paint".to_string()],
            available_modes: vec![
                CombatMode::Observation,  // Can try, will probably get caught
                CombatMode::Raiding,
            ],
        }
    }

    /// Create a basic elven ranger template
    pub fn elven_ranger() -> Self {
        Self {
            id: "elven_ranger".to_string(),
            display_name: "Elven Ranger".to_string(),
            race: "Elf".to_string(),
            base_health: 4,
            base_melee: 20,
            base_accuracy: 50,
            base_armor: 10,
            base_penetration: 15,
            base_morale: 55,
            base_mobility: 30,
            base_perception: 60,
            base_stealth: 65,
            base_range: 70,
            can_upgrade_health: false,
            available_upgrades: vec!["enchanted_arrows".to_string(), "cloaks".to_string()],
            available_modes: vec![
                CombatMode::Observation,
                CombatMode::Sniping,
                CombatMode::Guerilla,
                CombatMode::Raiding,
            ],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// UNIT ASSIGNMENT (where/what it's doing)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnitAssignment {
    /// Defending a section (with stationed/patrolling status)
    Garrison {
        location: (TileCoord, Direction),
        is_stationed: bool,  // true = full defense bonus, low recon
    },
    /// Moving between sections on patrol route
    Patrol {
        route: Vec<(TileCoord, Direction)>,
        current_index: usize,
    },
    /// Hidden, waiting to ambush
    Ambush(TileCoord, Direction),
    /// Traveling to destination
    Moving {
        from: (TileCoord, Direction),
        to: (TileCoord, Direction),
        progress: f32,  // 0.0 to 1.0
    },
    /// Part of an Army
    Attached(u32),  // army_id
    /// R&R, regaining endurance
    Recovering,
    /// Operating as independent detachment
    Detached {
        location: (TileCoord, Direction),
        mode: CombatMode,
    },
}

// ═══════════════════════════════════════════════════════════════════
// MILITARY UNIT (instance of a template)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilitaryUnit {
    pub id: u32,
    pub name: String,            // Player-given name: "1st Riverside Spears"
    pub faction: String,
    pub template_id: String,     // References UnitTemplate

    // Composition
    pub current_strength: i32,   // Current soldiers (takes casualties)
    pub max_strength: i32,       // Full strength (e.g., 100)
    pub experience: i32,         // 0-1000

    // Current stats (computed from template + equipment + experience)
    // These are the EFFECTIVE values after all modifiers
    pub health: i32,
    pub melee: i32,
    pub accuracy: i32,
    pub armor: i32,
    pub penetration: i32,
    pub morale: i32,
    pub mobility: i32,
    pub perception: i32,
    pub stealth: i32,
    pub range: i32,
    pub current_endurance: i32,
    pub max_endurance: i32,

    // Equipment
    pub equipment: Vec<Equipment>,

    // Where/what it's doing
    pub assignment: UnitAssignment,

    // State
    pub days_without_supply: i32,

    // Observation/Scouting state (used when in Detached Observation mode)
    pub turns_observing: i32,
    pub reporting_method: ReportingMethod,
}

// ═══════════════════════════════════════════════════════════════════
// EQUIPMENT
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipment {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,  // "PIERCING", "PEN5", "REACH", "DIST80", etc.

    // Stat modifiers
    pub melee_mod: i32,
    pub accuracy_mod: i32,
    pub armor_mod: i32,
    pub penetration_mod: i32,
    pub mobility_mod: i32,
    pub range_mod: i32,
}

// ═══════════════════════════════════════════════════════════════════
// MILITARY ROSTER
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilitaryRoster {
    pub faction: String,
    pub units: Vec<MilitaryUnit>,
    pub templates: Vec<UnitTemplate>,  // Available unit types for this faction
    pub next_id: u32,
}

impl MilitaryRoster {
    pub fn new(faction: String) -> Self {
        Self {
            faction,
            units: Vec::new(),
            templates: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_template(&mut self, template: UnitTemplate) {
        self.templates.push(template);
    }

    pub fn get_template(&self, template_id: &str) -> Option<&UnitTemplate> {
        self.templates.iter().find(|t| t.id == template_id)
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
                UnitAssignment::Garrison { location, .. } => *location == loc,
                UnitAssignment::Ambush(t, d) => (*t, *d) == loc,
                UnitAssignment::Detached { location, .. } => *location == loc,
                _ => false,
            }
        }).collect()
    }

    /// Create a unit from a template
    pub fn create_unit(
        &mut self,
        name: String,
        template_id: &str,
        strength: i32,
    ) -> Result<u32, String> {
        let template = self.get_template(template_id)
            .ok_or_else(|| format!("Template '{}' not found", template_id))?
            .clone();

        let id = self.next_id;
        self.next_id += 1;

        let max_endurance = template.base_health * strength;

        self.units.push(MilitaryUnit {
            id,
            name,
            faction: self.faction.clone(),
            template_id: template_id.to_string(),
            current_strength: strength,
            max_strength: strength,
            experience: 0,
            health: template.base_health,
            melee: template.base_melee,
            accuracy: template.base_accuracy,
            armor: template.base_armor,
            penetration: template.base_penetration,
            morale: template.base_morale,
            mobility: template.base_mobility,
            perception: template.base_perception,
            stealth: template.base_stealth,
            range: template.base_range,
            current_endurance: max_endurance,
            max_endurance,
            equipment: Vec::new(),
            assignment: UnitAssignment::Recovering,
            days_without_supply: 0,
            turns_observing: 0,
            reporting_method: ReportingMethod::default(),
        });

        Ok(id)
    }

    /// Apply equipment to a unit, recalculating stats
    pub fn equip_unit(&mut self, unit_id: u32, equipment: Equipment) -> Result<(), String> {
        let unit = self.get_unit_mut(unit_id)
            .ok_or_else(|| format!("Unit {} not found", unit_id))?;

        // Apply modifiers
        unit.melee += equipment.melee_mod;
        unit.accuracy += equipment.accuracy_mod;
        unit.armor += equipment.armor_mod;
        unit.penetration += equipment.penetration_mod;
        unit.mobility += equipment.mobility_mod;
        unit.range += equipment.range_mod;

        unit.equipment.push(equipment);
        Ok(())
    }
}
