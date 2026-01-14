use crate::models::intel_model::{
    Confidence, FactionDossier, IntelReport, Observations, ReportingMethod,
};
use crate::models::military_model::{CombatMode, MilitaryUnit, ReportingMethod as UnitReportingMethod, UnitAssignment};
use crate::models::tile_model::{Direction, Section, TileCoord};
use rand::Rng;

/// Generate intelligence observations based on unit capability and time observing
pub fn gather_intelligence(
    unit: &MilitaryUnit,
    section: &Section,
    turns_observing: i32,
) -> Observations {
    let mut rng = rand::thread_rng();

    // Base perception modified by unit's perception stat
    let effective_perception = unit.perception as f32 / 100.0;

    // Time increases detail granularity
    let time_bonus = (turns_observing as f32 * 0.1).min(0.5); // Max +50% from time
    let total_perception = (effective_perception + time_bonus).min(1.0);

    // Always observe terrain (basic visual)
    let terrain = Some(section.terrain);

    // Ownership visible with minimal observation
    let ownership = if total_perception > 0.2 {
        section.ownership.clone()
    } else {
        None
    };

    let ownership_faction = if total_perception > 0.2 {
        section.ownership_faction
    } else {
        None
    };

    // Population estimation - accuracy improves with time and perception
    let population_estimate = if let Some(_) = section.ownership {
        if total_perception > 0.3 {
            let actual_pop = section.population as f32;
            // Error margin decreases with perception
            let error_margin = (1.0 - total_perception) * 0.5; // Max 50% error
            let min_estimate = (actual_pop * (1.0 - error_margin)).max(0.0) as i32;
            let max_estimate = (actual_pop * (1.0 + error_margin)) as i32;
            Some((min_estimate, max_estimate))
        } else {
            None
        }
    } else {
        None
    };

    // Building observation - number of buildings spotted depends on perception
    let mut buildings_observed = Vec::new();
    let building_spot_threshold = 0.4;

    if total_perception > building_spot_threshold {
        // Can spot some buildings
        let buildings_visible =
            ((total_perception - building_spot_threshold) * section.buildings.len() as f32).ceil()
                as usize;

        for (i, building) in section.buildings.iter().enumerate() {
            if i < buildings_visible {
                buildings_observed.push(building.building_type);
            } else {
                // Random chance to spot additional buildings
                if rng.gen_bool((total_perception * 0.5) as f64) {
                    buildings_observed.push(building.building_type);
                }
            }
        }
    }

    // Patrol count observation (requires high perception or long observation)
    let patrol_count = if total_perception > 0.6 {
        // For now, return None as we don't have patrol data in Section
        // This will be populated when patrol system is implemented
        None
    } else {
        None
    };

    // Special notes based on perception
    let mut special_notes = Vec::new();

    if total_perception > 0.7 {
        if section.buildings.iter().any(|b| {
            matches!(
                b.building_type,
                crate::models::tile_model::BuildingType::Watchtower
            )
        }) {
            special_notes.push("Strong defensive position detected".to_string());
        }
    }

    if total_perception > 0.8 {
        // Could detect traps, ambush positions, etc.
        special_notes.push("Detailed tactical assessment available".to_string());
    }

    Observations {
        terrain,
        ownership,
        ownership_faction,
        population_estimate,
        buildings_observed,
        patrol_count,
        special_notes,
    }
}

/// Process intelligence gathering for all units in Observation mode
pub fn process_observation_intelligence(
    units: &mut [MilitaryUnit],
    get_section: impl Fn(&TileCoord, &Direction) -> Option<Section>,
    dossier: &mut FactionDossier,
    current_week: i32,
    current_day: i32,
) -> Vec<IntelReport> {
    let mut new_reports = Vec::new();

    for unit in units {
        // Only process units in Detached assignment with Observation mode
        if let UnitAssignment::Detached { location, mode } = &unit.assignment {
            if !matches!(mode, CombatMode::Observation) {
                continue;
            }

            let (coord, dir) = location;
            if let Some(section) = get_section(coord, dir) {
                // Increment observation time
                unit.turns_observing += 1;

                // Gather observations
                let observations = gather_intelligence(unit, &section, unit.turns_observing);

                // Check if unit can report this turn based on reporting method
                let can_report = match unit.reporting_method {
                    UnitReportingMethod::MagicBond => true, // Always report instantly
                    UnitReportingMethod::Hawk => unit.turns_observing % 3 == 0, // Every 3 turns
                    UnitReportingMethod::Return => {
                        // Only when unit returns (triggered separately)
                        false
                    }
                };

                if can_report {
                    let report = IntelReport {
                        scout_id: unit.id,
                        location: (*coord, *dir),
                        timestamp: (current_week, current_day),
                        observations,
                        reporting_method: match unit.reporting_method {
                            UnitReportingMethod::MagicBond => ReportingMethod::MagicBond,
                            UnitReportingMethod::Hawk => ReportingMethod::Hawk,
                            UnitReportingMethod::Return => ReportingMethod::Return,
                        },
                    };

                    // Add to dossier
                    dossier.add_report(report.clone());
                    new_reports.push(report);
                }
            }
        }
    }

    new_reports
}

/// Recall a unit from observation mission (generates final report for Return method)
pub fn recall_observer(
    unit: &mut MilitaryUnit,
    get_section: impl Fn(&TileCoord, &Direction) -> Option<Section>,
    dossier: &mut FactionDossier,
    current_week: i32,
    current_day: i32,
) -> Option<IntelReport> {
    // Only process units in Detached assignment with Observation mode
    if let UnitAssignment::Detached { location, mode } = &unit.assignment {
        if !matches!(mode, CombatMode::Observation) {
            return None;
        }

        let (coord, dir) = location;
        if let Some(section) = get_section(coord, dir) {
            // Generate final report with all accumulated knowledge
            let observations = gather_intelligence(unit, &section, unit.turns_observing);

            let report = IntelReport {
                scout_id: unit.id,
                location: (*coord, *dir),
                timestamp: (current_week, current_day),
                observations,
                reporting_method: ReportingMethod::Return,
            };

            // Add to dossier
            dossier.add_report(report.clone());

            // Reset unit to recovering state
            unit.assignment = UnitAssignment::Recovering;
            unit.turns_observing = 0;

            return Some(report);
        }
    }

    None
}

/// Deploy a unit to observation mode at a target location
pub fn deploy_observer(
    unit: &mut MilitaryUnit,
    target: (TileCoord, Direction),
) -> Result<(), String> {
    // Check if unit is already assigned (only Recovering units can be deployed)
    if !matches!(unit.assignment, UnitAssignment::Recovering) {
        return Err(format!("Unit {} is already assigned", unit.name));
    }

    unit.assignment = UnitAssignment::Detached {
        location: target,
        mode: CombatMode::Observation,
    };
    unit.turns_observing = 0;

    Ok(())
}

/// Calculate intel quality score based on unit stats and observation time
pub fn calculate_intel_quality(unit: &MilitaryUnit, turns_observing: i32) -> Confidence {
    let perception_factor = unit.perception as f32 / 100.0;
    let time_factor = (turns_observing as f32 / 5.0).min(1.0); // Maxes at 5 turns

    let quality_score = (perception_factor + time_factor) / 2.0;

    match quality_score {
        x if x < 0.2 => Confidence::Rumor,
        x if x < 0.4 => Confidence::Low,
        x if x < 0.6 => Confidence::Medium,
        x if x < 0.8 => Confidence::High,
        _ => Confidence::Confirmed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tile_model::{Building, BuildingType, Terrain};

    fn create_test_unit(perception: i32, mode: CombatMode) -> MilitaryUnit {
        MilitaryUnit {
            id: 1,
            name: "Test Observer".to_string(),
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
            stealth: 50,
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
            reporting_method: UnitReportingMethod::MagicBond,
        }
    }

    fn create_test_section() -> Section {
        Section {
            direction: Direction::N,
            terrain: Terrain::Forest,
            ownership: Some("Enemy".to_string()),
            ownership_faction: Some(crate::models::tile_model::Faction::Lamia),
            population: 100,
            traits: Vec::new(),
            buildings: vec![
                Building {
                    building_type: BuildingType::Watchtower,
                    level: 1,
                    is_active: true,
                },
                Building {
                    building_type: BuildingType::Farm,
                    level: 1,
                    is_active: true,
                },
            ],
        }
    }

    #[test]
    fn test_basic_intelligence_gathering() {
        let unit = create_test_unit(50, CombatMode::Observation);
        let section = create_test_section();

        let observations = gather_intelligence(&unit, &section, 0);

        // Should always see terrain
        assert_eq!(observations.terrain, Some(Terrain::Forest));

        // With 50 perception, should see ownership
        assert!(observations.ownership.is_some());
    }

    #[test]
    fn test_time_improves_detail() {
        let unit = create_test_unit(30, CombatMode::Observation);
        let section = create_test_section();

        // Immediate observation
        let obs1 = gather_intelligence(&unit, &section, 0);

        // After 5 turns
        let obs2 = gather_intelligence(&unit, &section, 5);

        // Should gather more details over time
        // With low perception + time, should eventually see buildings
        assert!(
            obs2.buildings_observed.len() >= obs1.buildings_observed.len(),
            "More time should reveal more or equal details"
        );
    }

    #[test]
    fn test_population_estimation_accuracy() {
        let high_perception_unit = create_test_unit(90, CombatMode::Observation);
        let low_perception_unit = create_test_unit(20, CombatMode::Observation);
        let section = create_test_section();

        let high_obs = gather_intelligence(&high_perception_unit, &section, 5);
        let low_obs = gather_intelligence(&low_perception_unit, &section, 0);

        // High perception should get population estimate
        assert!(high_obs.population_estimate.is_some());

        // Low perception might not
        if let Some((min, max)) = high_obs.population_estimate {
            let range = max - min;
            // High perception should have tighter range
            assert!(range < 50, "High perception should have narrow estimate");
        }
    }

    #[test]
    fn test_deploy_observer() {
        let mut unit = create_test_unit(50, CombatMode::Observation);
        unit.assignment = UnitAssignment::Unassigned;

        let target = (TileCoord { x: 1, y: 1 }, Direction::Core);
        let result = deploy_observer(&mut unit, target);

        assert!(result.is_ok());
        assert!(matches!(
            unit.assignment,
            UnitAssignment::Detached { location: _, mode: CombatMode::Observation }
        ));
        assert_eq!(unit.turns_observing, 0);
    }

    #[test]
    fn test_cannot_deploy_already_assigned_unit() {
        let mut unit = create_test_unit(50, CombatMode::Observation);
        // Unit is already in Detached assignment

        let target = (TileCoord { x: 1, y: 1 }, Direction::Core);
        let result = deploy_observer(&mut unit, target);

        assert!(result.is_err());
    }

    #[test]
    fn test_intel_quality_calculation() {
        let high_perception = create_test_unit(80, CombatMode::Observation);
        let low_perception = create_test_unit(20, CombatMode::Observation);

        // High perception + time = Confirmed
        let quality1 = calculate_intel_quality(&high_perception, 5);
        assert_eq!(quality1, Confidence::Confirmed);

        // Low perception + no time = Rumor
        let quality2 = calculate_intel_quality(&low_perception, 0);
        assert_eq!(quality2, Confidence::Rumor);

        // Medium perception + some time = Medium
        let medium_unit = create_test_unit(40, CombatMode::Observation);
        let quality3 = calculate_intel_quality(&medium_unit, 2);
        assert!(matches!(quality3, Confidence::Low | Confidence::Medium));
    }

    #[test]
    fn test_process_observation_only_observation_mode() {
        let mut observer = create_test_unit(50, CombatMode::Observation);

        let mut raider = create_test_unit(50, CombatMode::Raiding);
        raider.id = 2;

        let mut units = vec![observer, raider];

        let get_section = |_coord: &TileCoord, _dir: &Direction| Some(create_test_section());

        let mut dossier = FactionDossier::new("Test".to_string());

        let reports = process_observation_intelligence(&mut units, get_section, &mut dossier, 1, 1);

        // Only the observer should generate a report (MagicBond reports immediately)
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].scout_id, 1); // The observer
    }
}
