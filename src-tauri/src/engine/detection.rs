use crate::models::tile_model::{Direction, Section, Terrain, TileCoord};
use crate::models::scout_model::{ScoutMode, ScoutUnit};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionResult {
    Undetected,
    Detected { scout_id: u32, location: (TileCoord, Direction) },
    Captured { scout_id: u32, location: (TileCoord, Direction) },
}

/// Calculate detection probability for a scout in a section
pub fn calculate_detection_probability(
    scout: &ScoutUnit,
    section: &Section,
    patrols_in_section: i32,
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

    // Scout stealth reduces detection
    let stealth_reduction = scout.stealth as f32 * 0.5; // 50 stealth = -25% detection

    // Mode modifier
    let mode_modifier = match scout.mode {
        ScoutMode::Observe => 0.0,  // No additional risk
        ScoutMode::Probe => 15.0,   // +15% detection risk
    };

    // Terrain modifier
    let terrain_modifier = match section.terrain {
        Terrain::Plains => 0.0,
        Terrain::Forest => -10.0,   // Forests provide cover
        Terrain::Mountains => -5.0, // Mountains provide some cover
        Terrain::Swamp => 5.0,      // Swamps are harder to navigate stealthily
        Terrain::Desert => 5.0,     // Deserts offer little cover
    };

    // Final probability (clamped to 0-95%)
    let probability = defender_perception + mode_modifier + terrain_modifier - stealth_reduction;
    probability.max(0.0).min(95.0)
}

/// Roll for detection of a deployed scout
pub fn roll_detection(scout: &ScoutUnit, section: &Section, patrols_in_section: i32) -> DetectionResult {
    let mut rng = rand::thread_rng();
    let detection_chance = calculate_detection_probability(scout, section, patrols_in_section);

    let roll = rng.gen_range(0.0..100.0);

    if roll < detection_chance {
        // Detected! Now roll for capture (perception gap determines capture chance)
        let capture_threshold = if scout.perception > 50 {
            // High-perception scouts are harder to capture (they spot the trap)
            20.0 - ((scout.perception as f32 - 50.0) * 0.3)
        } else {
            20.0
        };

        let capture_roll = rng.gen_range(0.0..100.0);
        if capture_roll < capture_threshold {
            DetectionResult::Captured {
                scout_id: scout.id,
                location: scout.assigned_location.unwrap(),
            }
        } else {
            DetectionResult::Detected {
                scout_id: scout.id,
                location: scout.assigned_location.unwrap(),
            }
        }
    } else {
        DetectionResult::Undetected
    }
}

/// Process detection rolls for all deployed scouts in a faction
pub fn process_scout_detection(
    scouts: &[ScoutUnit],
    get_section: impl Fn(&TileCoord, &Direction) -> Option<Section>,
    get_patrol_count: impl Fn(&TileCoord, &Direction) -> i32,
) -> Vec<DetectionResult> {
    let mut results = Vec::new();

    for scout in scouts {
        if !scout.is_deployed {
            continue;
        }

        if let Some((coord, dir)) = scout.assigned_location {
            if let Some(section) = get_section(&coord, &dir) {
                let patrol_count = get_patrol_count(&coord, &dir);
                let result = roll_detection(scout, &section, patrol_count);

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
    use crate::models::scout_model::ReportingMethod;

    fn create_test_scout(stealth: i32, perception: i32, mode: ScoutMode) -> ScoutUnit {
        ScoutUnit {
            id: 1,
            name: "Test Scout".to_string(),
            faction: "Test".to_string(),
            stealth,
            perception,
            mobility: 20,
            assigned_location: Some((TileCoord { x: 0, y: 0 }, Direction::N)),
            mode,
            reporting_method: ReportingMethod::Return,
            is_deployed: true,
            turns_observing: 0,
            has_returned: false,
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
        let scout = create_test_scout(0, 0, ScoutMode::Observe);
        let section = create_test_section(Terrain::Plains, false);

        // 0 patrols = 0% base
        assert_eq!(calculate_detection_probability(&scout, &section, 0), 0.0);

        // 1 patrol = 10% base
        assert_eq!(calculate_detection_probability(&scout, &section, 1), 10.0);

        // 5+ patrols = 40% base (capped)
        assert_eq!(calculate_detection_probability(&scout, &section, 5), 40.0);
    }

    #[test]
    fn test_stealth_reduction() {
        let scout = create_test_scout(50, 0, ScoutMode::Observe);
        let section = create_test_section(Terrain::Plains, false);

        // 50 stealth = -25% detection
        // 1 patrol (10%) - 25% = 0% (clamped)
        assert_eq!(calculate_detection_probability(&scout, &section, 1), 0.0);

        // 5 patrols (40%) - 25% = 15%
        assert_eq!(calculate_detection_probability(&scout, &section, 5), 15.0);
    }

    #[test]
    fn test_watchtower_bonus() {
        let scout = create_test_scout(0, 0, ScoutMode::Observe);
        let section = create_test_section(Terrain::Plains, true);

        // 1 patrol (10%) + watchtower (20%) = 30%
        assert_eq!(calculate_detection_probability(&scout, &section, 1), 30.0);
    }

    #[test]
    fn test_terrain_modifiers() {
        let scout = create_test_scout(0, 0, ScoutMode::Observe);

        // Forest: -10%
        let forest = create_test_section(Terrain::Forest, false);
        assert_eq!(calculate_detection_probability(&scout, &forest, 1), 0.0); // 10% - 10% = 0%

        // Swamp: +5%
        let swamp = create_test_section(Terrain::Swamp, false);
        assert_eq!(calculate_detection_probability(&scout, &swamp, 1), 15.0); // 10% + 5% = 15%
    }

    #[test]
    fn test_probe_mode_risk() {
        let scout = create_test_scout(0, 0, ScoutMode::Probe);
        let section = create_test_section(Terrain::Plains, false);

        // 1 patrol (10%) + probe mode (15%) = 25%
        assert_eq!(calculate_detection_probability(&scout, &section, 1), 25.0);
    }
}
