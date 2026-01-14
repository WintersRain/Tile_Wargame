use crate::models::tile_model::{Direction, Section, Terrain, TileCoord};
use crate::models::military_model::{CombatMode, MilitaryUnit, UnitAssignment};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionResult {
    Undetected,
    Detected { unit_id: u32, location: (TileCoord, Direction) },
    Captured { unit_id: u32, location: (TileCoord, Direction) },
}

/// Calculate detection probability for a unit operating in detached mode
pub fn calculate_detection_probability(
    unit: &MilitaryUnit,
    section: &Section,
    patrols_in_section: i32,
    mode: &CombatMode,
) -> f32 {
    // Base detection chance from patrol density (0-40%)
    let patrol_density = (patrols_in_section as f32 * 10.0).min(40.0);

    // Watchtower bonus (+20% perception if present)
    let watchtower_bonus = if section.buildings.iter().any(|b| {
        matches!(
            b.building_type,
            crate::models::tile_model::BuildingType::Watchtower
        )
    }) {
        20.0
    } else {
        0.0
    };

    // Effective defender perception
    let defender_perception = patrol_density + watchtower_bonus;

    // Unit stealth reduces detection
    let stealth_reduction = unit.stealth as f32 * 0.5; // 50 stealth = -25% detection

    // Mode modifier - more aggressive modes increase detection risk
    let mode_modifier = match mode {
        CombatMode::None => 0.0,         // Not applicable
        CombatMode::Observation => 0.0,  // No additional risk - passive observation
        CombatMode::Sniping => 15.0,     // +15% detection risk - engaging from distance
        CombatMode::Guerilla => 20.0,    // +20% detection risk - close ambush activity
        CombatMode::Raiding => 25.0,     // +25% detection risk - aggressive raiding
    };

    // Terrain modifier
    let terrain_modifier = match section.terrain {
        Terrain::Plains => 0.0,
        Terrain::Forest => -10.0,   // Forests provide cover
        Terrain::Mountain => -5.0, // Mountains provide some cover
        Terrain::Swamp => 5.0,      // Swamps are harder to navigate stealthily
        Terrain::Desert => 5.0,     // Deserts offer little cover
    };

    // Final probability (clamped to 0-95%)
    let probability = defender_perception + mode_modifier + terrain_modifier - stealth_reduction;
    probability.max(0.0).min(95.0)
}

/// Roll for detection of a deployed military unit in detached mode
pub fn roll_detection(
    unit: &MilitaryUnit,
    section: &Section,
    patrols_in_section: i32,
    location: (TileCoord, Direction),
    mode: &CombatMode,
) -> DetectionResult {
    let mut rng = rand::thread_rng();
    let detection_chance = calculate_detection_probability(unit, section, patrols_in_section, mode);

    let roll = rng.gen_range(0.0..100.0);

    if roll < detection_chance {
        // Detected! Now roll for capture (perception gap determines capture chance)
        let capture_threshold = if unit.perception > 50 {
            // High-perception units are harder to capture (they spot the trap)
            20.0 - ((unit.perception as f32 - 50.0) * 0.3)
        } else {
            20.0
        };

        let capture_roll = rng.gen_range(0.0..100.0);
        if capture_roll < capture_threshold {
            DetectionResult::Captured {
                unit_id: unit.id,
                location,
            }
        } else {
            DetectionResult::Detected {
                unit_id: unit.id,
                location,
            }
        }
    } else {
        DetectionResult::Undetected
    }
}

/// Process detection rolls for all detached units in a faction
pub fn process_unit_detection(
    units: &[MilitaryUnit],
    get_section: impl Fn(&TileCoord, &Direction) -> Option<Section>,
    get_patrol_count: impl Fn(&TileCoord, &Direction) -> i32,
) -> Vec<DetectionResult> {
    let mut results = Vec::new();

    for unit in units {
        // Only process units in Detached assignment
        if let UnitAssignment::Detached { location, mode } = &unit.assignment {
            let (coord, dir) = location;
            if let Some(section) = get_section(coord, dir) {
                let patrol_count = get_patrol_count(coord, dir);
                let result = roll_detection(unit, &section, patrol_count, *location, mode);

                if !matches!(result, DetectionResult::Undetected) {
                    results.push(result);
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tile_model::{Building, BuildingType};
    use crate::models::military_model::ReportingMethod;

    fn create_test_unit(stealth: i32, perception: i32, mode: CombatMode) -> MilitaryUnit {
        MilitaryUnit {
            id: 1,
            name: "Test Unit".to_string(),
            faction: "Test".to_string(),
            template_id: "test_template".to_string(),
            current_strength: 10,
            max_strength: 10,
            experience: 0,
            health: 5,
            melee: 30,
            accuracy: 20,
            armor: 15,
            penetration: 5,
            morale: 50,
            mobility: 20,
            perception,
            stealth,
            range: 0,
            current_endurance: 50,
            max_endurance: 50,
            equipment: Vec::new(),
            assignment: UnitAssignment::Detached {
                location: (TileCoord { x: 0, y: 0 }, Direction::N),
                mode: mode.clone(),
            },
            days_without_supply: 0,
            turns_observing: 0,
            reporting_method: ReportingMethod::Return,
        }
    }

    fn create_test_section(terrain: Terrain, has_watchtower: bool) -> Section {
        let mut buildings = Vec::new();
        if has_watchtower {
            buildings.push(Building {
                building_type: BuildingType::Watchtower,
                level: 1,
                is_active: true,
            });
        }

        Section {
            direction: Direction::N,
            terrain,
            ownership: None,
            ownership_faction: None,
            population: 0,
            traits: Vec::new(),
            buildings,
        }
    }

    #[test]
    fn test_base_detection_probability() {
        let unit = create_test_unit(0, 0, CombatMode::Observation);
        let section = create_test_section(Terrain::Plains, false);

        // 0 patrols = 0% base
        assert_eq!(calculate_detection_probability(&unit, &section, 0, &CombatMode::Observation), 0.0);

        // 1 patrol = 10% base
        assert_eq!(calculate_detection_probability(&unit, &section, 1, &CombatMode::Observation), 10.0);

        // 5+ patrols = 40% base (capped)
        assert_eq!(calculate_detection_probability(&unit, &section, 5, &CombatMode::Observation), 40.0);
    }

    #[test]
    fn test_stealth_reduction() {
        let unit = create_test_unit(50, 0, CombatMode::Observation);
        let section = create_test_section(Terrain::Plains, false);

        // 50 stealth = -25% detection
        // 1 patrol (10%) - 25% = 0% (clamped)
        assert_eq!(calculate_detection_probability(&unit, &section, 1, &CombatMode::Observation), 0.0);

        // 5 patrols (40%) - 25% = 15%
        assert_eq!(calculate_detection_probability(&unit, &section, 5, &CombatMode::Observation), 15.0);
    }

    #[test]
    fn test_watchtower_bonus() {
        let unit = create_test_unit(0, 0, CombatMode::Observation);
        let section = create_test_section(Terrain::Plains, true);

        // 1 patrol (10%) + watchtower (20%) = 30%
        assert_eq!(calculate_detection_probability(&unit, &section, 1, &CombatMode::Observation), 30.0);
    }

    #[test]
    fn test_terrain_modifiers() {
        let unit = create_test_unit(0, 0, CombatMode::Observation);

        // Forest: -10%
        let forest = create_test_section(Terrain::Forest, false);
        assert_eq!(calculate_detection_probability(&unit, &forest, 1, &CombatMode::Observation), 0.0);

        // Swamp: +5%
        let swamp = create_test_section(Terrain::Swamp, false);
        assert_eq!(calculate_detection_probability(&unit, &swamp, 1, &CombatMode::Observation), 15.0);
    }

    #[test]
    fn test_combat_mode_risks() {
        let unit = create_test_unit(0, 0, CombatMode::Observation);
        let section = create_test_section(Terrain::Plains, false);

        // Observation: 1 patrol (10%) + 0% mode = 10%
        assert_eq!(calculate_detection_probability(&unit, &section, 1, &CombatMode::Observation), 10.0);

        // Sniping: 1 patrol (10%) + 15% mode = 25%
        assert_eq!(calculate_detection_probability(&unit, &section, 1, &CombatMode::Sniping), 25.0);

        // Guerilla: 1 patrol (10%) + 20% mode = 30%
        assert_eq!(calculate_detection_probability(&unit, &section, 1, &CombatMode::Guerilla), 30.0);

        // Raiding: 1 patrol (10%) + 25% mode = 35%
        assert_eq!(calculate_detection_probability(&unit, &section, 1, &CombatMode::Raiding), 35.0);
    }

    #[test]
    fn test_process_unit_detection_only_detached() {
        let detached_unit = create_test_unit(0, 0, CombatMode::Observation);

        let mut garrison_unit = create_test_unit(0, 0, CombatMode::Observation);
        garrison_unit.id = 2;
        garrison_unit.assignment = UnitAssignment::Garrison {
            location: (TileCoord { x: 0, y: 0 }, Direction::N),
            is_stationed: true,
        };

        let units = vec![detached_unit, garrison_unit];

        let get_section = |_coord: &TileCoord, _dir: &Direction| {
            Some(create_test_section(Terrain::Plains, false))
        };

        let get_patrol_count = |_coord: &TileCoord, _dir: &Direction| 0;

        let results = process_unit_detection(&units, get_section, get_patrol_count);

        // With 0 patrols and 0 stealth, detection chance is 0%, so no results expected
        assert_eq!(results.len(), 0);
    }
}
