use crate::models::intel_model::{
    Confidence, FactionDossier, IntelReport, Observations, ReportingMethod,
};
use crate::models::scout_model::{ReportingMethod as ScoutReportingMethod, ScoutUnit};
use crate::models::tile_model::{Direction, Section, TileCoord};
use rand::Rng;

/// Generate intelligence observations based on scout capability and time observing
pub fn gather_intelligence(
    scout: &ScoutUnit,
    section: &Section,
    turns_observing: i32,
) -> Observations {
    let mut rng = rand::thread_rng();

    // Base perception modified by scout's perception stat
    let effective_perception = scout.perception as f32 / 100.0;

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

    // Special notes based on mode and perception
    let mut special_notes = Vec::new();

    // Probe mode can discover hidden details
    if matches!(scout.mode, crate::models::scout_model::ScoutMode::Probe) {
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

/// Process scout intelligence gathering for all deployed scouts
pub fn process_scout_intelligence(
    scouts: &mut [ScoutUnit],
    get_section: impl Fn(&TileCoord, &Direction) -> Option<Section>,
    dossier: &mut FactionDossier,
    current_week: i32,
    current_day: i32,
) -> Vec<IntelReport> {
    let mut new_reports = Vec::new();

    for scout in scouts {
        if !scout.is_deployed {
            continue;
        }

        if let Some((coord, dir)) = scout.assigned_location {
            if let Some(section) = get_section(&coord, &dir) {
                // Increment observation time
                scout.turns_observing += 1;

                // Gather observations
                let observations = gather_intelligence(scout, &section, scout.turns_observing);

                // Check if scout can report this turn
                let can_report = match scout.reporting_method {
                    ScoutReportingMethod::MagicBond => true, // Always report instantly
                    ScoutReportingMethod::Hawk => scout.turns_observing % 3 == 0, // Every 3 turns
                    ScoutReportingMethod::Return => {
                        // Only when scout returns (triggered separately)
                        false
                    }
                };

                if can_report {
                    let report = IntelReport {
                        scout_id: scout.id,
                        location: (coord, dir),
                        timestamp: (current_week, current_day),
                        observations,
                        reporting_method: match scout.reporting_method {
                            ScoutReportingMethod::MagicBond => ReportingMethod::MagicBond,
                            ScoutReportingMethod::Hawk => ReportingMethod::Hawk,
                            ScoutReportingMethod::Return => ReportingMethod::Return,
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

/// Recall a scout from their mission (generates final report for Return method)
pub fn recall_scout(
    scout: &mut ScoutUnit,
    get_section: impl Fn(&TileCoord, &Direction) -> Option<Section>,
    dossier: &mut FactionDossier,
    current_week: i32,
    current_day: i32,
) -> Option<IntelReport> {
    if !scout.is_deployed {
        return None;
    }

    if let Some((coord, dir)) = scout.assigned_location {
        if let Some(section) = get_section(&coord, &dir) {
            // Generate final report with all accumulated knowledge
            let observations = gather_intelligence(scout, &section, scout.turns_observing);

            let report = IntelReport {
                scout_id: scout.id,
                location: (coord, dir),
                timestamp: (current_week, current_day),
                observations,
                reporting_method: ReportingMethod::Return,
            };

            // Add to dossier
            dossier.add_report(report.clone());

            // Reset scout status
            scout.is_deployed = false;
            scout.assigned_location = None;
            scout.turns_observing = 0;
            scout.has_returned = true;

            return Some(report);
        }
    }

    None
}

/// Deploy a scout to a target location
pub fn deploy_scout(
    scout: &mut ScoutUnit,
    target: (TileCoord, Direction),
) -> Result<(), String> {
    if scout.is_deployed {
        return Err(format!("Scout {} is already deployed", scout.name));
    }

    scout.is_deployed = true;
    scout.assigned_location = Some(target);
    scout.turns_observing = 0;
    scout.has_returned = false;

    Ok(())
}

/// Calculate intel quality score based on scout stats and observation time
pub fn calculate_intel_quality(scout: &ScoutUnit, turns_observing: i32) -> Confidence {
    let perception_factor = scout.perception as f32 / 100.0;
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
    use crate::models::scout_model::ScoutMode;
    use crate::models::tile_model::{Building, BuildingType, Terrain};

    fn create_test_scout(perception: i32, mode: ScoutMode) -> ScoutUnit {
        ScoutUnit {
            id: 1,
            name: "Test Scout".to_string(),
            faction: "Test".to_string(),
            stealth: 50,
            perception,
            mobility: 20,
            assigned_location: Some((TileCoord { x: 0, y: 0 }, Direction::N)),
            mode,
            reporting_method: ScoutReportingMethod::MagicBond,
            is_deployed: true,
            turns_observing: 0,
            has_returned: false,
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
        let scout = create_test_scout(50, ScoutMode::Observe);
        let section = create_test_section();

        let observations = gather_intelligence(&scout, &section, 0);

        // Should always see terrain
        assert_eq!(observations.terrain, Some(Terrain::Forest));

        // With 50 perception, should see ownership
        assert!(observations.ownership.is_some());
    }

    #[test]
    fn test_time_improves_detail() {
        let scout = create_test_scout(30, ScoutMode::Observe);
        let section = create_test_section();

        // Immediate observation
        let obs1 = gather_intelligence(&scout, &section, 0);

        // After 5 turns
        let obs2 = gather_intelligence(&scout, &section, 5);

        // Should gather more details over time
        // With low perception + time, should eventually see buildings
        assert!(
            obs2.buildings_observed.len() >= obs1.buildings_observed.len(),
            "More time should reveal more or equal details"
        );
    }

    #[test]
    fn test_population_estimation_accuracy() {
        let high_perception_scout = create_test_scout(90, ScoutMode::Observe);
        let low_perception_scout = create_test_scout(20, ScoutMode::Observe);
        let section = create_test_section();

        let high_obs = gather_intelligence(&high_perception_scout, &section, 5);
        let low_obs = gather_intelligence(&low_perception_scout, &section, 0);

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
    fn test_deploy_scout() {
        let mut scout = create_test_scout(50, ScoutMode::Observe);
        scout.is_deployed = false;
        scout.assigned_location = None;

        let target = (TileCoord { x: 1, y: 1 }, Direction::Core);
        let result = deploy_scout(&mut scout, target);

        assert!(result.is_ok());
        assert!(scout.is_deployed);
        assert_eq!(scout.assigned_location, Some(target));
        assert_eq!(scout.turns_observing, 0);
    }

    #[test]
    fn test_cannot_deploy_already_deployed_scout() {
        let mut scout = create_test_scout(50, ScoutMode::Observe);
        // Scout is already deployed

        let target = (TileCoord { x: 1, y: 1 }, Direction::Core);
        let result = deploy_scout(&mut scout, target);

        assert!(result.is_err());
    }

    #[test]
    fn test_intel_quality_calculation() {
        let high_perception = create_test_scout(80, ScoutMode::Observe);
        let low_perception = create_test_scout(20, ScoutMode::Observe);

        // High perception + time = Confirmed
        let quality1 = calculate_intel_quality(&high_perception, 5);
        assert_eq!(quality1, Confidence::Confirmed);

        // Low perception + no time = Rumor
        let quality2 = calculate_intel_quality(&low_perception, 0);
        assert_eq!(quality2, Confidence::Rumor);

        // Medium perception + some time = Medium
        let medium_scout = create_test_scout(40, ScoutMode::Observe);
        let quality3 = calculate_intel_quality(&medium_scout, 2);
        assert!(matches!(quality3, Confidence::Low | Confidence::Medium));
    }
}
