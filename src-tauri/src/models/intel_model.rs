use super::tile_model::{BuildingType, Direction, Faction, Terrain, TileCoord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Confidence level for intelligence data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Rumor,      // Unconfirmed, third-hand information
    Low,        // Single observation, old data
    Medium,     // Multiple observations, recent
    High,       // Recent, repeated, or probe-confirmed
    Confirmed,  // Captured documents, returned scouts with MagicBond
}

/// Intel about a specific building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingIntel {
    pub building_type: BuildingType,
    pub level: Option<i32>,        // May not know exact level
    pub confidence: Confidence,
    pub last_observed: (i32, i32), // (week, day)
}

/// Intel about a section within a tile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionIntel {
    pub direction: Direction,
    pub terrain: Terrain,
    pub ownership: Option<String>,
    pub ownership_faction: Option<Faction>,
    pub population: Option<PopulationIntel>,
    pub buildings: Vec<BuildingIntel>,
    pub patrol_density: Option<i32>,  // Observed patrols per turn
    pub last_observed: (i32, i32),    // (week, day)
}

/// Intel about population in a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationIntel {
    pub count: Option<i32>,       // Exact count if known
    pub range: Option<(i32, i32)>, // Estimated range
    pub confidence: Confidence,
    pub last_observed: (i32, i32),
}

/// Intel about an entire tile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileIntel {
    pub coord: TileCoord,
    pub sections: HashMap<Direction, SectionIntel>,
    pub overall_threat_level: ThreatLevel,
    pub notes: Vec<String>, // Free-form observations
}

/// Assessed threat level for a tile
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreatLevel {
    Unknown,
    Minimal,    // Neutral/undefended
    Low,        // Light patrols
    Moderate,   // Regular patrols + fortifications
    High,       // Heavy defenses
    Extreme,    // Fortress + elite units
}

/// A single intelligence report from a scout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelReport {
    pub scout_id: u32,
    pub location: (TileCoord, Direction),
    pub timestamp: (i32, i32), // (week, day)
    pub observations: Observations,
    pub reporting_method: ReportingMethod,
}

/// Type of reporting used (affects timing and reliability)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReportingMethod {
    Return,     // Scout must return physically
    Hawk,       // Message sent via animal
    MagicBond,  // Instant telepathic link
}

/// What a scout observed during their mission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observations {
    pub terrain: Option<Terrain>,
    pub ownership: Option<String>,
    pub ownership_faction: Option<Faction>,
    pub population_estimate: Option<(i32, i32)>, // (min, max) range
    pub buildings_observed: Vec<BuildingType>,
    pub patrol_count: Option<i32>,
    pub special_notes: Vec<String>, // Traps, ambushes, fortifications
}

/// Complete intelligence dossier for a faction about another faction or territory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionDossier {
    pub observer: String,      // Who is collecting this intel
    pub subject: Option<String>, // Who/what is being observed (None for general recon)
    pub tiles: HashMap<String, TileIntel>, // Key: "(x, y)"
    pub reports: Vec<IntelReport>,
    pub summary_notes: Vec<String>,
}

impl FactionDossier {
    pub fn new(observer: String, subject: Option<String>) -> Self {
        Self {
            observer,
            subject,
            tiles: HashMap::new(),
            reports: Vec::new(),
            summary_notes: Vec::new(),
        }
    }

    /// Add a new intel report and update tile intelligence
    pub fn add_report(&mut self, report: IntelReport) {
        let (coord, direction) = report.location;
        let key = format!("({}, {})", coord.x, coord.y);

        // Get or create TileIntel
        let tile_intel = self.tiles.entry(key.clone()).or_insert_with(|| TileIntel {
            coord,
            sections: HashMap::new(),
            overall_threat_level: ThreatLevel::Unknown,
            notes: Vec::new(),
        });

        // Update or create SectionIntel
        let section_intel = tile_intel.sections.entry(direction).or_insert_with(|| {
            SectionIntel {
                direction,
                terrain: Terrain::Plains, // Default, will be updated
                ownership: None,
                ownership_faction: None,
                population: None,
                buildings: Vec::new(),
                patrol_density: None,
                last_observed: report.timestamp,
            }
        });

        // Update section intel from observations
        if let Some(terrain) = report.observations.terrain {
            section_intel.terrain = terrain;
        }

        if let Some(ownership) = &report.observations.ownership {
            section_intel.ownership = Some(ownership.clone());
        }

        if let Some(faction) = report.observations.ownership_faction {
            section_intel.ownership_faction = Some(faction);
        }

        if let Some((min, max)) = report.observations.population_estimate {
            section_intel.population = Some(PopulationIntel {
                count: None,
                range: Some((min, max)),
                confidence: Confidence::Medium,
                last_observed: report.timestamp,
            });
        }

        if let Some(patrol_count) = report.observations.patrol_count {
            section_intel.patrol_density = Some(patrol_count);
        }

        // Add building intel
        for building_type in &report.observations.buildings_observed {
            // Check if we already have intel on this building
            let existing = section_intel.buildings.iter_mut()
                .find(|b| b.building_type == *building_type);

            if let Some(existing_building) = existing {
                // Update confidence and timestamp
                existing_building.confidence = Confidence::Medium;
                existing_building.last_observed = report.timestamp;
            } else {
                // New building observed
                section_intel.buildings.push(BuildingIntel {
                    building_type: *building_type,
                    level: None,
                    confidence: Confidence::Low,
                    last_observed: report.timestamp,
                });
            }
        }

        section_intel.last_observed = report.timestamp;

        // Update threat level based on new information
        self.update_threat_level(&key);

        // Store the report
        self.reports.push(report);
    }

    /// Recalculate threat level for a tile based on intel
    fn update_threat_level(&mut self, tile_key: &str) {
        if let Some(tile_intel) = self.tiles.get_mut(tile_key) {
            let mut threat_score = 0;

            for section_intel in tile_intel.sections.values() {
                // Patrols increase threat
                if let Some(patrol_count) = section_intel.patrol_density {
                    threat_score += patrol_count * 5;
                }

                // Fortifications increase threat
                for building in &section_intel.buildings {
                    match building.building_type {
                        BuildingType::Watchtower => threat_score += 10,
                        BuildingType::Keep | BuildingType::BroodHive => threat_score += 20,
                        _ => {}
                    }
                }
            }

            tile_intel.overall_threat_level = match threat_score {
                0..=5 => ThreatLevel::Minimal,
                6..=15 => ThreatLevel::Low,
                16..=30 => ThreatLevel::Moderate,
                31..=50 => ThreatLevel::High,
                _ => ThreatLevel::Extreme,
            };
        }
    }

    /// Get the most recent intel for a specific section
    pub fn get_section_intel(&self, coord: &TileCoord, direction: &Direction) -> Option<&SectionIntel> {
        let key = format!("({}, {})", coord.x, coord.y);
        self.tiles.get(&key)?.sections.get(direction)
    }

    /// Check how stale intel is (returns weeks since last observation)
    pub fn intel_age(&self, coord: &TileCoord, direction: &Direction, current_week: i32) -> Option<i32> {
        let section_intel = self.get_section_intel(coord, direction)?;
        let (last_week, _) = section_intel.last_observed;
        Some(current_week - last_week)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dossier_creation() {
        let dossier = FactionDossier::new("Player".to_string(), Some("Lamia".to_string()));
        assert_eq!(dossier.observer, "Player");
        assert_eq!(dossier.subject, Some("Lamia".to_string()));
        assert_eq!(dossier.tiles.len(), 0);
    }

    #[test]
    fn test_add_report_creates_tile_intel() {
        let mut dossier = FactionDossier::new("Player".to_string(), None);

        let report = IntelReport {
            scout_id: 1,
            location: (TileCoord { x: 1, y: 1 }, Direction::N),
            timestamp: (1, 1),
            observations: Observations {
                terrain: Some(Terrain::Forest),
                ownership: Some("Enemy".to_string()),
                ownership_faction: Some(Faction::Lamia),
                population_estimate: Some((50, 100)),
                buildings_observed: vec![BuildingType::Watchtower],
                patrol_count: Some(2),
                special_notes: vec![],
            },
            reporting_method: ReportingMethod::Return,
        };

        dossier.add_report(report);

        assert_eq!(dossier.tiles.len(), 1);
        assert_eq!(dossier.reports.len(), 1);

        let tile_intel = dossier.tiles.get("(1, 1)").unwrap();
        assert_eq!(tile_intel.sections.len(), 1);

        let section_intel = tile_intel.sections.get(&Direction::N).unwrap();
        assert_eq!(section_intel.terrain, Terrain::Forest);
        assert_eq!(section_intel.ownership, Some("Enemy".to_string()));
        assert_eq!(section_intel.patrol_density, Some(2));
    }

    #[test]
    fn test_threat_level_calculation() {
        let mut dossier = FactionDossier::new("Player".to_string(), None);

        // Low threat: 1 watchtower, 1 patrol
        let report1 = IntelReport {
            scout_id: 1,
            location: (TileCoord { x: 0, y: 0 }, Direction::N),
            timestamp: (1, 1),
            observations: Observations {
                terrain: Some(Terrain::Plains),
                ownership: Some("Enemy".to_string()),
                ownership_faction: Some(Faction::Lamia),
                population_estimate: None,
                buildings_observed: vec![BuildingType::Watchtower],
                patrol_count: Some(1),
                special_notes: vec![],
            },
            reporting_method: ReportingMethod::Return,
        };

        dossier.add_report(report1);
        let tile_intel = dossier.tiles.get("(0, 0)").unwrap();
        assert_eq!(tile_intel.overall_threat_level, ThreatLevel::Low);
    }

    #[test]
    fn test_intel_age() {
        let mut dossier = FactionDossier::new("Player".to_string(), None);

        let report = IntelReport {
            scout_id: 1,
            location: (TileCoord { x: 0, y: 0 }, Direction::Core),
            timestamp: (1, 1),
            observations: Observations {
                terrain: Some(Terrain::Plains),
                ownership: None,
                ownership_faction: None,
                population_estimate: None,
                buildings_observed: vec![],
                patrol_count: None,
                special_notes: vec![],
            },
            reporting_method: ReportingMethod::Return,
        };

        dossier.add_report(report);

        let age = dossier.intel_age(&TileCoord { x: 0, y: 0 }, &Direction::Core, 5);
        assert_eq!(age, Some(4)); // 5 - 1 = 4 weeks old
    }
}
