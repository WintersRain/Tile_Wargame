pub mod ai;
pub mod combat;
pub mod supply;
pub mod detection;
pub mod scouting;
pub mod command;
pub mod dice;

use crate::models::tile_model::{Building, BuildingType, Direction, Faction, Terrain, Tile, TileCoord};
use crate::models::scout_model::{ScoutRoster, ScoutMode, ReportingMethod as ScoutReportingMethod};
use crate::models::intel_model::{FactionDossier, IntelReport};
use crate::models::military_model::{MilitaryUnit, MilitaryRoster, UnitTemplate, UnitAssignment};
use combat::{resolve_engagement, EngagementReport};
use scouting::{deploy_scout, recall_scout, process_scout_intelligence};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub name: String,
    pub building_type: BuildingType,
    pub day_cost: i32,
    pub progress: i32,
    pub location: String, // "x,y"
    pub section: Direction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub map: HashMap<String, Tile>,
    pub current_week: i32,
    pub current_day: i32,
    pub task_queue: Vec<Task>,
    pub player_faction: Faction,
    pub scout_rosters: HashMap<String, ScoutRoster>,
    pub faction_dossiers: HashMap<String, FactionDossier>,
}

pub struct AppState(pub Mutex<GameState>);

impl GameState {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        let mut rng = rand::thread_rng();

        // 1. Initial sections & terrain
        for x in 0..3 {
            for y in 0..3 {
                let key = format!("({}, {})", x, y);
                map.insert(key, Tile::new(x, y, Terrain::Plains));
            }
        }

        // 2. Spawn player (Always in a random tile Core)
        let p_x = rng.gen_range(0..3);
        let p_y = rng.gen_range(0..3);
        let p_faction = Faction::Human;
        let p_key = format!("({}, {})", p_x, p_y);

        if let Some(tile) = map.get_mut(&p_key) {
            if let Some(section) = tile.sections.get_mut(&Direction::Core) {
                section.ownership = Some("Player".to_string());
                section.ownership_faction = Some(p_faction);
                section.population = 100;
                section.buildings.push(Building {
                    building_type: BuildingType::Keep,
                    level: 1,
                    is_active: true,
                });
            }
        }

        // 3. Spawn enemies (Lamia Hive Prime in a different random tile Core)
        let mut e_x = rng.gen_range(0..3);
        let mut e_y = rng.gen_range(0..3);
        while e_x == p_x && e_y == p_y {
            e_x = rng.gen_range(0..3);
            e_y = rng.gen_range(0..3);
        }
        let e_key = format!("({}, {})", e_x, e_y);
        if let Some(tile) = map.get_mut(&e_key) {
            if let Some(section) = tile.sections.get_mut(&Direction::Core) {
                section.ownership = Some("Hive Prime".to_string());
                section.ownership_faction = Some(Faction::Lamia);
                section.population = 100;
                section.buildings.push(Building {
                    building_type: BuildingType::BroodHive,
                    level: 1,
                    is_active: true,
                });
            }
        }

        // 4. Spawn Neutrals (3-5 random settlements in ANY section, including outer ones)
        let num_neutrals = rng.gen_range(3..=5);
        let directions = [
            Direction::N,
            Direction::NE,
            Direction::E,
            Direction::SE,
            Direction::S,
            Direction::SW,
            Direction::W,
            Direction::NW,
            Direction::Core,
        ];

        let mut neutral_count = 0;
        while neutral_count < num_neutrals {
            let n_x = rng.gen_range(0..3);
            let n_y = rng.gen_range(0..3);
            let n_dir = directions[rng.gen_range(0..directions.len())];
            let n_key = format!("({}, {})", n_x, n_y);

            if let Some(tile) = map.get_mut(&n_key) {
                if let Some(section) = tile.sections.get_mut(&n_dir) {
                    if section.ownership.is_none() {
                        section.ownership = Some("Neutral Settlement".to_string());
                        section.population = rng.gen_range(20..60);

                        let b_type = if n_dir == Direction::Core {
                            BuildingType::Granary
                        } else {
                            BuildingType::Farm
                        };
                        section.buildings.push(Building {
                            building_type: b_type,
                            level: 1,
                            is_active: true,
                        });
                        neutral_count += 1;
                    }
                }
            }
        }

        // 5. Initialize scout rosters for each faction
        let mut scout_rosters = HashMap::new();
        scout_rosters.insert("Player".to_string(), ScoutRoster::new("Player".to_string()));
        scout_rosters.insert("Hive Prime".to_string(), ScoutRoster::new("Hive Prime".to_string()));

        // 6. Initialize faction dossiers (each faction tracks intel on others)
        let mut faction_dossiers = HashMap::new();
        faction_dossiers.insert(
            "Player".to_string(),
            FactionDossier::new("Player".to_string(), Some("Hive Prime".to_string()))
        );
        faction_dossiers.insert(
            "Hive Prime".to_string(),
            FactionDossier::new("Hive Prime".to_string(), Some("Player".to_string()))
        );

        Self {
            map,
            current_week: 1,
            current_day: 1,
            task_queue: Vec::new(),
            player_faction: p_faction,
            scout_rosters,
            faction_dossiers,
        }
    }

    pub fn advance_day(&mut self) {
        self.current_day += 1;
        if self.current_day > 7 {
            self.current_day = 1;
            self.current_week += 1;
        }

        // 1. Process construction tasks
        let mut completed_indices = Vec::new();
        for (i, task) in self.task_queue.iter_mut().enumerate() {
            if task.progress < task.day_cost {
                task.progress += 1;
                if task.progress >= task.day_cost {
                    completed_indices.push(i);
                }
            }
        }

        for idx in completed_indices.iter().rev() {
            let task = self.task_queue.remove(*idx);
            if let Some(tile) = self.map.get_mut(&task.location) {
                if let Some(section) = tile.sections.get_mut(&task.section) {
                    section.buildings.push(Building {
                        building_type: task.building_type,
                        level: 1,
                        is_active: true,
                    });
                    println!(
                        "COMPLETED: Added {:?} to Tile {} Section {:?}",
                        task.building_type, task.location, task.section
                    );
                }
            }
        }

        // 2. Resolve Terraforming (Lamia Flood Hatchery)
        let mut changes = Vec::new();
        let mut rng = rand::thread_rng();

        for (key, tile) in &self.map {
            for (dir, section) in &tile.sections {
                let has_hatchery = section
                    .buildings
                    .iter()
                    .any(|b| b.building_type == BuildingType::FloodHatchery);
                if has_hatchery && section.terrain == Terrain::Plains {
                    if rng.gen_bool(0.1) {
                        changes.push((key.clone(), *dir));
                    }
                }
            }
        }

        for (key, dir) in changes {
            if let Some(tile) = self.map.get_mut(&key) {
                if let Some(section) = tile.sections.get_mut(&dir) {
                    section.terrain = Terrain::Swamp;
                    println!(
                        "TERRAFORMED: Section {:?} in Tile {} is now Swamp!",
                        dir, key
                    );
                }
            }
        }

        // 3. Process scout intelligence gathering
        let factions: Vec<String> = self.scout_rosters.keys().cloned().collect();
        for faction in factions {
            if let Some(roster) = self.scout_rosters.get_mut(&faction) {
                if let Some(dossier) = self.faction_dossiers.get_mut(&faction) {
                    let get_section = |coord: &TileCoord, dir: &Direction| {
                        let key = format!("({}, {})", coord.x, coord.y);
                        self.map.get(&key)?.sections.get(dir).cloned()
                    };

                    let reports = process_scout_intelligence(
                        &mut roster.scouts,
                        get_section,
                        dossier,
                        self.current_week,
                        self.current_day,
                    );

                    for report in reports {
                        println!(
                            "INTEL: Scout {} from {} reported on ({}, {}) {:?}",
                            report.scout_id,
                            faction,
                            report.location.0.x,
                            report.location.0.y,
                            report.location.1
                        );
                    }
                }
            }
        }
    }
}

#[tauri::command]
pub fn advance_turn(state: State<AppState>) -> GameState {
    let mut game = state.0.lock().unwrap();
    game.advance_day();
    GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
    }
}

#[tauri::command]
pub fn add_task(
    state: State<AppState>,
    name: String,
    building_type: BuildingType,
    day_cost: i32,
    location: String,
    section: Direction,
) -> GameState {
    let mut game = state.0.lock().unwrap();
    game.task_queue.push(Task {
        name,
        building_type,
        day_cost,
        progress: 0,
        location,
        section,
    });
    GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
    }
}

#[tauri::command]
pub fn get_map(state: State<AppState>) -> GameState {
    let game = state.0.lock().unwrap();
    GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
    }
}

#[tauri::command]
pub fn simulate_combat() -> EngagementReport {
    // Create test units using the new MilitaryUnit structure
    let mut attacker = MilitaryUnit {
        id: 1,
        name: "Imperial Longbowmen".to_string(),
        faction: "Human".to_string(),
        template_id: "human_archer".to_string(),
        current_strength: 100,
        max_strength: 100,
        experience: 0,
        health: 5,
        melee: 5,
        accuracy: 45,
        armor: 5,
        penetration: 10,
        morale: 80,
        mobility: 15,
        perception: 30,
        stealth: 10,
        range: 60,
        current_endurance: 100,
        max_endurance: 500,
        equipment: Vec::new(),
        assignment: UnitAssignment::Recovering,
        days_without_supply: 0,
    };

    let mut defender = MilitaryUnit {
        id: 2,
        name: "Chaos Zealots".to_string(),
        faction: "Chaos".to_string(),
        template_id: "chaos_zealot".to_string(),
        current_strength: 100,
        max_strength: 100,
        experience: 0,
        health: 6,
        melee: 40,
        accuracy: 10,
        armor: 15,
        penetration: 5,
        morale: 100,
        mobility: 35,
        perception: 10,
        stealth: 5,
        range: 10,
        current_endurance: 100,
        max_endurance: 600,
        equipment: Vec::new(),
        assignment: UnitAssignment::Recovering,
        days_without_supply: 0,
    };

    let terrain = Terrain::Plains;  // Default terrain for testing
    resolve_engagement(&mut attacker, &mut defender, &terrain)
}

// ========== Scout Commands ==========

#[tauri::command]
pub fn create_scout(
    state: State<AppState>,
    faction: String,
    name: String,
    stealth: i32,
    perception: i32,
    mobility: i32,
) -> Result<u32, String> {
    let mut game = state.0.lock().unwrap();

    if let Some(roster) = game.scout_rosters.get_mut(&faction) {
        let id = roster.create_scout(name, stealth, perception, mobility);
        Ok(id)
    } else {
        Err(format!("Faction '{}' not found", faction))
    }
}

#[tauri::command]
pub fn assign_scout(
    state: State<AppState>,
    faction: String,
    scout_id: u32,
    target_x: i32,
    target_y: i32,
    target_section: Direction,
    mode: String,
) -> Result<GameState, String> {
    let mut game = state.0.lock().unwrap();

    // Parse mode
    let scout_mode = match mode.as_str() {
        "observe" => ScoutMode::Observe,
        "probe" => ScoutMode::Probe,
        _ => return Err(format!("Invalid scout mode: {}", mode)),
    };

    // Get scout roster
    let roster = game.scout_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Faction '{}' not found", faction))?;

    // Find scout
    let scout = roster.scouts.iter_mut()
        .find(|s| s.id == scout_id)
        .ok_or_else(|| format!("Scout {} not found", scout_id))?;

    // Update mode
    scout.mode = scout_mode;

    // Deploy scout
    let target_coord = TileCoord { x: target_x, y: target_y };
    deploy_scout(scout, (target_coord, target_section))?;

    Ok(GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
    })
}

#[tauri::command]
pub fn recall_scout_cmd(
    state: State<AppState>,
    faction: String,
    scout_id: u32,
) -> Result<GameState, String> {
    let mut game = state.0.lock().unwrap();

    // Get scout roster
    let roster = game.scout_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Faction '{}' not found", faction))?;

    // Find scout
    let scout = roster.scouts.iter_mut()
        .find(|s| s.id == scout_id)
        .ok_or_else(|| format!("Scout {} not found", scout_id))?;

    // Get dossier
    let dossier = game.faction_dossiers.get_mut(&faction)
        .ok_or_else(|| format!("Dossier for faction '{}' not found", faction))?;

    // Recall scout (generates final report)
    let get_section = |coord: &TileCoord, dir: &Direction| {
        let key = format!("({}, {})", coord.x, coord.y);
        game.map.get(&key)?.sections.get(dir).cloned()
    };

    recall_scout(
        scout,
        get_section,
        dossier,
        game.current_week,
        game.current_day,
    );

    Ok(GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
    })
}

#[tauri::command]
pub fn get_faction_intel(
    state: State<AppState>,
    faction: String,
) -> Result<FactionDossier, String> {
    let game = state.0.lock().unwrap();

    game.faction_dossiers.get(&faction)
        .cloned()
        .ok_or_else(|| format!("Dossier for faction '{}' not found", faction))
}

#[tauri::command]
pub fn get_scout_roster(
    state: State<AppState>,
    faction: String,
) -> Result<ScoutRoster, String> {
    let game = state.0.lock().unwrap();

    game.scout_rosters.get(&faction)
        .cloned()
        .ok_or_else(|| format!("Scout roster for faction '{}' not found", faction))
}

