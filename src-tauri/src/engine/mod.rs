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
use crate::models::military_model::{MilitaryUnit, MilitaryRoster, UnitTemplate, UnitAssignment, CombatMode};
use combat::{resolve_engagement, resolve_ambush, EngagementReport};
use crate::models::tile_model::Section;
use scouting::{deploy_scout, recall_scout, process_scout_intelligence};
use ai::{AIPersonality, AIObjective, AIAction};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

// ========== AI Helper Functions ==========

/// Get all sections owned by a specific faction
fn get_owned_sections(map: &HashMap<String, Tile>, owner: &str) -> Vec<(TileCoord, Direction)> {
    let mut owned = Vec::new();

    for (_key, tile) in map {
        for (dir, section) in &tile.sections {
            if let Some(ref sect_owner) = section.ownership {
                if sect_owner == owner {
                    owned.push((TileCoord { x: tile.x, y: tile.y }, *dir));
                }
            }
        }
    }

    owned
}

/// Get adjacent sections with their ownership info
fn get_adjacent_sections(
    map: &HashMap<String, Tile>,
    coord: TileCoord,
    dir: Direction,
) -> Vec<(TileCoord, Direction, Option<String>, Option<Faction>)> {
    let mut adjacent = Vec::new();

    let neighbors = Tile::get_neighbors(coord.x, coord.y, dir);

    for ((nx, ny), ndir) in neighbors {
        let key = format!("({}, {})", nx, ny);
        if let Some(tile) = map.get(&key) {
            if let Some(section) = tile.sections.get(&ndir) {
                adjacent.push((
                    TileCoord { x: nx, y: ny },
                    ndir,
                    section.ownership.clone(),
                    section.ownership_faction,
                ));
            }
        }
    }

    adjacent
}

/// Evaluate all possible AI actions for a faction
fn evaluate_ai_actions(
    map: &HashMap<String, Tile>,
    faction: &str,
    personality: &AIPersonality,
) -> Vec<AIAction> {
    let mut available_actions: Vec<(AIObjective, f32, String, (i32, i32), Direction)> = Vec::new();

    let owned_sections = get_owned_sections(map, faction);

    for (coord, dir) in owned_sections {
        let adjacent = get_adjacent_sections(map, coord, dir);

        for (adj_coord, adj_dir, adj_owner, _adj_faction) in adjacent {
            match adj_owner {
                // Neutral settlement - expansion opportunity
                Some(ref owner) if owner.contains("Neutral") => {
                    let base_value = 10.0;
                    let description = format!(
                        "Expand to ({}, {}) {:?}",
                        adj_coord.x, adj_coord.y, adj_dir
                    );
                    available_actions.push((
                        AIObjective::Expansion,
                        base_value,
                        description,
                        (adj_coord.x, adj_coord.y),
                        adj_dir,
                    ));
                }

                // Enemy section - aggression opportunity
                Some(ref owner) if owner != faction => {
                    let base_value = 15.0;
                    let description = format!(
                        "Attack {} at ({}, {}) {:?}",
                        owner, adj_coord.x, adj_coord.y, adj_dir
                    );
                    available_actions.push((
                        AIObjective::Aggression,
                        base_value,
                        description,
                        (adj_coord.x, adj_coord.y),
                        adj_dir,
                    ));
                }

                // Unowned section - expansion opportunity
                None => {
                    let base_value = 8.0;
                    let description = format!(
                        "Claim empty ({}, {}) {:?}",
                        adj_coord.x, adj_coord.y, adj_dir
                    );
                    available_actions.push((
                        AIObjective::Expansion,
                        base_value,
                        description,
                        (adj_coord.x, adj_coord.y),
                        adj_dir,
                    ));
                }

                // Own section - skip
                _ => {}
            }
        }
    }

    personality.evaluate_actions(available_actions)
}

/// Execute an AI action by moving a unit toward the target
fn execute_ai_action(
    military_rosters: &mut HashMap<String, MilitaryRoster>,
    map: &HashMap<String, Tile>,
    faction: &str,
    action: &AIAction,
) -> Result<(), String> {
    let roster = military_rosters.get_mut(faction)
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))?;

    // Find a unit that can be reassigned (garrisoned and not moving)
    let available_unit = roster.units.iter_mut()
        .find(|u| matches!(&u.assignment,
            UnitAssignment::Garrison { is_stationed: true, .. } |
            UnitAssignment::Garrison { is_stationed: false, .. } |
            UnitAssignment::Recovering));

    if let Some(unit) = available_unit {
        let from_location = match &unit.assignment {
            UnitAssignment::Garrison { location, .. } => *location,
            _ => {
                let owned = get_owned_sections(map, faction);
                if let Some(loc) = owned.first() {
                    *loc
                } else {
                    return Err("No valid starting location for unit".to_string());
                }
            }
        };

        let target_coord = TileCoord { x: action.location.0, y: action.location.1 };
        let target_location = (target_coord, action.target_section);

        if from_location == target_location {
            unit.assignment = UnitAssignment::Garrison {
                location: target_location,
                is_stationed: false,
            };
        } else {
            unit.assignment = UnitAssignment::Moving {
                from: from_location,
                to: target_location,
                progress: 0.0,
            };
        }

        println!(
            "AI ACTION: {} - {} (score: {:.2})",
            faction, action.description, action.score
        );

        Ok(())
    } else {
        Err(format!("No available units for faction '{}'", faction))
    }
}

/// Process AI turn for a faction
fn process_ai_turn(
    military_rosters: &mut HashMap<String, MilitaryRoster>,
    map: &HashMap<String, Tile>,
    faction: &str,
    personality: &AIPersonality,
) {
    let actions = evaluate_ai_actions(map, faction, personality);

    if actions.is_empty() {
        println!("AI TURN: {} has no valid actions", faction);
        return;
    }

    let best_action = &actions[0];

    println!(
        "AI TURN: {} evaluating {} actions, best: {} (score: {:.2})",
        faction, actions.len(), best_action.description, best_action.score
    );

    if let Err(e) = execute_ai_action(military_rosters, map, faction, best_action) {
        println!("AI ACTION FAILED: {} - {}", faction, e);
    }
}

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
    pub military_rosters: HashMap<String, MilitaryRoster>,
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

        // 7. Initialize military rosters for each faction
        let mut military_rosters = HashMap::new();

        // Player starting force - human infantry at their Core
        let mut player_roster = MilitaryRoster::new("Player".to_string());
        player_roster.add_template(UnitTemplate::human_infantry());
        player_roster.add_template(UnitTemplate::elven_ranger()); // Give player variety
        let player_unit_id = player_roster.create_unit(
            "1st Militia".to_string(),
            "human_infantry",
            100,
        ).expect("Failed to create player starting unit");

        // Assign player unit to garrison at their Core
        if let Some(unit) = player_roster.get_unit_mut(player_unit_id) {
            unit.assignment = UnitAssignment::Garrison {
                location: (TileCoord { x: p_x, y: p_y }, Direction::Core),
                is_stationed: true,
            };
        }
        military_rosters.insert("Player".to_string(), player_roster);

        // Hive Prime starting force - orc raiders at their Core
        let mut hive_roster = MilitaryRoster::new("Hive Prime".to_string());
        hive_roster.add_template(UnitTemplate::orc_raider());
        let hive_unit_id = hive_roster.create_unit(
            "Hive Brood".to_string(),
            "orc_raider",
            100,
        ).expect("Failed to create Hive Prime starting unit");

        // Assign Hive Prime unit to garrison at their Core
        if let Some(unit) = hive_roster.get_unit_mut(hive_unit_id) {
            unit.assignment = UnitAssignment::Garrison {
                location: (TileCoord { x: e_x, y: e_y }, Direction::Core),
                is_stationed: true,
            };
        }
        military_rosters.insert("Hive Prime".to_string(), hive_roster);

        Self {
            map,
            current_week: 1,
            current_day: 1,
            task_queue: Vec::new(),
            player_faction: p_faction,
            scout_rosters,
            faction_dossiers,
            military_rosters,
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

        // 4. Process AI turns for non-player factions
        let ai_factions: Vec<String> = self.military_rosters.keys()
            .filter(|f| *f != "Player")
            .cloned()
            .collect();

        for faction in ai_factions {
            let personality = if faction == "Hive Prime" {
                AIPersonality::new_aggressive()
            } else {
                AIPersonality::new_expansionist()
            };

            process_ai_turn(&mut self.military_rosters, &self.map, &faction, &personality);
        }

        // 5. Process unit movement
        let faction_names: Vec<String> = self.military_rosters.keys().cloned().collect();
        let mut arrivals: Vec<(String, u32, (TileCoord, Direction))> = Vec::new();

        for faction in &faction_names {
            if let Some(roster) = self.military_rosters.get_mut(faction) {
                for unit in roster.units.iter_mut() {
                    if let UnitAssignment::Moving { from: _, to, progress } = &mut unit.assignment {
                        // Increment progress based on mobility (mobility 20 = 7 days to move)
                        let daily_progress = unit.mobility as f32 / 140.0;
                        *progress += daily_progress;

                        // Check if unit has arrived
                        if *progress >= 1.0 {
                            arrivals.push((faction.clone(), unit.id, *to));
                            println!(
                                "ARRIVAL: {} ({}) reached ({}, {}) {:?}",
                                unit.name, faction, to.0.x, to.0.y, to.1
                            );
                        }
                    }
                }
            }
        }

        // Process arrivals - change assignment to Garrison
        for (faction, unit_id, destination) in &arrivals {
            if let Some(roster) = self.military_rosters.get_mut(faction) {
                if let Some(unit) = roster.get_unit_mut(*unit_id) {
                    unit.assignment = UnitAssignment::Garrison {
                        location: *destination,
                        is_stationed: false, // Arrive patrolling, not stationed
                    };
                }
            }
        }

        // 6. Resolve combat in sections with units from multiple factions
        // First detect ambushes, then resolve standard combat
        let mut combat_results: Vec<(String, Direction, String, u32, String, u32, Terrain)> = Vec::new();
        let mut ambush_results: Vec<(String, Direction, String, u32, CombatMode, String, u32, Terrain)> = Vec::new();

        // Scan all sections for conflicts
        for (tile_key, tile) in &self.map {
            for (direction, section) in &tile.sections {
                // Parse tile coordinates from key format "(x, y)"
                let coords: Vec<&str> = tile_key.trim_matches(|c| c == '(' || c == ')').split(", ").collect();
                if coords.len() != 2 { continue; }
                let tile_x = coords[0].parse::<i32>().unwrap_or(0);
                let tile_y = coords[1].parse::<i32>().unwrap_or(0);
                let location = (TileCoord { x: tile_x, y: tile_y }, *direction);

                // Collect units at this location grouped by faction, noting ambush status
                let mut faction_units: HashMap<String, Vec<(u32, Option<CombatMode>)>> = HashMap::new();
                for (faction_name, roster) in &self.military_rosters {
                    let units_here: Vec<(u32, Option<CombatMode>)> = roster.get_units_at_location(location)
                        .iter()
                        .map(|u| {
                            let ambush_mode = match &u.assignment {
                                UnitAssignment::Ambush(_, _) => Some(CombatMode::Raiding),
                                UnitAssignment::Detached { mode, .. }
                                    if matches!(mode, CombatMode::Guerilla | CombatMode::Raiding) => {
                                    Some(mode.clone())
                                }
                                _ => None,
                            };
                            (u.id, ambush_mode)
                        })
                        .collect();

                    if !units_here.is_empty() {
                        faction_units.insert(faction_name.clone(), units_here);
                    }
                }

                // If multiple factions present, check for ambushes first
                if faction_units.len() >= 2 {
                    let factions: Vec<String> = faction_units.keys().cloned().collect();

                    // Check for ambush situations
                    let mut has_ambush = false;
                    for (faction_name, units) in &faction_units {
                        for (unit_id, ambush_mode) in units {
                            if let Some(mode) = ambush_mode {
                                // Find a victim from another faction
                                for (other_faction, other_units) in &faction_units {
                                    if other_faction != faction_name {
                                        if let Some((victim_id, _)) = other_units.first() {
                                            ambush_results.push((
                                                tile_key.clone(),
                                                *direction,
                                                faction_name.clone(),
                                                *unit_id,
                                                mode.clone(),
                                                other_faction.clone(),
                                                *victim_id,
                                                section.terrain,
                                            ));
                                            has_ambush = true;
                                            break;
                                        }
                                    }
                                }
                                if has_ambush { break; }
                            }
                        }
                        if has_ambush { break; }
                    }

                    // If no ambush, queue standard combat
                    if !has_ambush {
                        let (attacker_faction, defender_faction) = if let Some(owner) = &section.ownership {
                            let non_owner = factions.iter().find(|f| *f != owner);
                            if let Some(atk) = non_owner {
                                (atk.clone(), owner.clone())
                            } else {
                                (factions[0].clone(), factions[1].clone())
                            }
                        } else {
                            (factions[0].clone(), factions[1].clone())
                        };

                        if let (Some(atk_units), Some(def_units)) = (
                            faction_units.get(&attacker_faction),
                            faction_units.get(&defender_faction)
                        ) {
                            if let (Some((atk_id, _)), Some((def_id, _))) = (atk_units.first(), def_units.first()) {
                                combat_results.push((
                                    tile_key.clone(),
                                    *direction,
                                    attacker_faction,
                                    *atk_id,
                                    defender_faction,
                                    *def_id,
                                    section.terrain,
                                ));
                            }
                        }
                    }
                }
            }
        }

        let mut ownership_changes: Vec<(String, Direction, String, Option<Faction>)> = Vec::new();

        // Process ambush results FIRST
        for (tile_key, direction, ambusher_faction, ambusher_id, ambush_mode, victim_faction, victim_id, terrain) in ambush_results {
            if ambusher_faction == victim_faction { continue; }

            let (mut ambusher_copy, mut victim_copy) = {
                let ambusher_roster = self.military_rosters.get(&ambusher_faction);
                let victim_roster = self.military_rosters.get(&victim_faction);

                match (ambusher_roster, victim_roster) {
                    (Some(ar), Some(vr)) => {
                        match (ar.get_unit(ambusher_id), vr.get_unit(victim_id)) {
                            (Some(a), Some(v)) => (a.clone(), v.clone()),
                            _ => continue,
                        }
                    }
                    _ => continue,
                }
            };

            // Resolve ambush
            let (ambush_report, should_continue) = resolve_ambush(
                &mut ambusher_copy,
                &mut victim_copy,
                &ambush_mode,
                &terrain
            );

            println!("AMBUSH at {} {:?}: {} ambushes {}", tile_key, direction, ambusher_faction, victim_faction);
            println!("  {}", ambush_report.message);

            // Apply ambush results
            if let Some(roster) = self.military_rosters.get_mut(&ambusher_faction) {
                if let Some(unit) = roster.get_unit_mut(ambusher_id) {
                    unit.current_endurance = ambusher_copy.current_endurance;
                    unit.current_strength = (unit.current_strength - ambush_report.attacker_losses).max(0);

                    // Handle Guerilla disengagement
                    if matches!(ambush_mode, CombatMode::Guerilla) && !should_continue {
                        unit.assignment = UnitAssignment::Recovering;
                    }
                }
            }

            if let Some(roster) = self.military_rosters.get_mut(&victim_faction) {
                if let Some(unit) = roster.get_unit_mut(victim_id) {
                    unit.current_endurance = victim_copy.current_endurance;
                    unit.current_strength = (unit.current_strength - ambush_report.defender_losses).max(0);
                }
            }

            // If combat continues (Raiding or failed Guerilla escape), resolve follow-up
            if should_continue && victim_copy.current_endurance > 0 {
                let follow_up_report = resolve_engagement(&mut ambusher_copy, &mut victim_copy, &terrain);
                println!("  FOLLOW-UP: {}", follow_up_report.message);

                if let Some(roster) = self.military_rosters.get_mut(&ambusher_faction) {
                    if let Some(unit) = roster.get_unit_mut(ambusher_id) {
                        unit.current_endurance = ambusher_copy.current_endurance;
                        unit.current_strength = (unit.current_strength - follow_up_report.attacker_losses).max(0);
                    }
                }

                if let Some(roster) = self.military_rosters.get_mut(&victim_faction) {
                    if let Some(unit) = roster.get_unit_mut(victim_id) {
                        unit.current_endurance = victim_copy.current_endurance;
                        unit.current_strength = (unit.current_strength - follow_up_report.defender_losses).max(0);
                    }
                }
            }

            // Check if victim wiped
            let victim_wiped = {
                self.military_rosters.get(&victim_faction)
                    .and_then(|r| r.get_unit(victim_id))
                    .map(|u| u.current_strength <= 0 || u.current_endurance <= 0)
                    .unwrap_or(false)
            };

            if victim_wiped {
                let ambusher_faction_enum = match ambusher_faction.as_str() {
                    "Player" => Some(Faction::Human),
                    "Hive Prime" => Some(Faction::Lamia),
                    _ => None,
                };
                ownership_changes.push((tile_key.clone(), direction, ambusher_faction.clone(), ambusher_faction_enum));
                println!("  VICTIM ELIMINATED! {} captures section", ambusher_faction);
            }
        }

        // Process standard combat results
        for (tile_key, direction, atk_faction, atk_id, def_faction, def_id, terrain) in combat_results {
            if atk_faction == def_faction { continue; }

            let (mut atk_copy, mut def_copy) = {
                let atk_roster = self.military_rosters.get(&atk_faction);
                let def_roster = self.military_rosters.get(&def_faction);

                match (atk_roster, def_roster) {
                    (Some(ar), Some(dr)) => {
                        match (ar.get_unit(atk_id), dr.get_unit(def_id)) {
                            (Some(a), Some(d)) => (a.clone(), d.clone()),
                            _ => continue,
                        }
                    }
                    _ => continue,
                }
            };

            let report = resolve_engagement(&mut atk_copy, &mut def_copy, &terrain);

            if let Some(roster) = self.military_rosters.get_mut(&atk_faction) {
                if let Some(unit) = roster.get_unit_mut(atk_id) {
                    unit.current_endurance = atk_copy.current_endurance;
                    unit.current_strength = (unit.current_strength - report.attacker_losses).max(0);
                }
            }
            if let Some(roster) = self.military_rosters.get_mut(&def_faction) {
                if let Some(unit) = roster.get_unit_mut(def_id) {
                    unit.current_endurance = def_copy.current_endurance;
                    unit.current_strength = (unit.current_strength - report.defender_losses).max(0);
                }
            }

            println!("COMBAT at {} {:?}: {} vs {}", tile_key, direction, atk_faction, def_faction);
            println!("  {}", report.message);

            let defender_wiped = {
                self.military_rosters.get(&def_faction)
                    .and_then(|r| r.get_unit(def_id))
                    .map(|u| u.current_strength <= 0 || u.current_endurance <= 0)
                    .unwrap_or(false)
            };

            if defender_wiped {
                let atk_faction_enum = match atk_faction.as_str() {
                    "Player" => Some(Faction::Human),
                    "Hive Prime" => Some(Faction::Lamia),
                    _ => None,
                };
                ownership_changes.push((tile_key.clone(), direction, atk_faction.clone(), atk_faction_enum));
                println!("  DEFENDER ELIMINATED! {} captures section", atk_faction);
            }
        }

        // Apply ownership changes
        for (tile_key, direction, new_owner, new_faction) in ownership_changes {
            if let Some(tile) = self.map.get_mut(&tile_key) {
                if let Some(section) = tile.sections.get_mut(&direction) {
                    section.ownership = Some(new_owner);
                    section.ownership_faction = new_faction;
                }
            }
        }

        // 7. Remove destroyed units (strength <= 0)
        for roster in self.military_rosters.values_mut() {
            roster.units.retain(|u| u.current_strength > 0);
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
        military_rosters: game.military_rosters.clone(),
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
        military_rosters: game.military_rosters.clone(),
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
        military_rosters: game.military_rosters.clone(),
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
        military_rosters: game.military_rosters.clone(),
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
        military_rosters: game.military_rosters.clone(),
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

// ========== Military Commands ==========

#[tauri::command]
pub fn get_military_roster(
    state: State<AppState>,
    faction: String,
) -> Result<MilitaryRoster, String> {
    let game = state.0.lock().unwrap();

    game.military_rosters.get(&faction)
        .cloned()
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))
}

#[tauri::command]
pub fn create_unit(
    state: State<AppState>,
    faction: String,
    name: String,
    template_id: String,
    strength: i32,
) -> Result<u32, String> {
    let mut game = state.0.lock().unwrap();

    let roster = game.military_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))?;

    roster.create_unit(name, &template_id, strength)
}

#[tauri::command]
pub fn assign_unit_garrison(
    state: State<AppState>,
    faction: String,
    unit_id: u32,
    tile_x: i32,
    tile_y: i32,
    section: Direction,
    is_stationed: bool,
) -> Result<GameState, String> {
    let mut game = state.0.lock().unwrap();

    let roster = game.military_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))?;

    let unit = roster.get_unit_mut(unit_id)
        .ok_or_else(|| format!("Unit {} not found in faction '{}'", unit_id, faction))?;

    unit.assignment = UnitAssignment::Garrison {
        location: (TileCoord { x: tile_x, y: tile_y }, section),
        is_stationed,
    };

    Ok(GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
        military_rosters: game.military_rosters.clone(),
    })
}

#[tauri::command]
pub fn assign_unit_detached(
    state: State<AppState>,
    faction: String,
    unit_id: u32,
    tile_x: i32,
    tile_y: i32,
    section: Direction,
    mode_str: String,
) -> Result<GameState, String> {
    let mut game = state.0.lock().unwrap();

    // Parse combat mode string
    let mode = match mode_str.as_str() {
        "observation" => CombatMode::Observation,
        "sniping" => CombatMode::Sniping,
        "guerilla" => CombatMode::Guerilla,
        "raiding" => CombatMode::Raiding,
        "none" => CombatMode::None,
        _ => return Err(format!("Invalid combat mode: {}", mode_str)),
    };

    let roster = game.military_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))?;

    let unit = roster.get_unit_mut(unit_id)
        .ok_or_else(|| format!("Unit {} not found in faction '{}'", unit_id, faction))?;

    unit.assignment = UnitAssignment::Detached {
        location: (TileCoord { x: tile_x, y: tile_y }, section),
        mode,
    };

    Ok(GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
        military_rosters: game.military_rosters.clone(),
    })
}

#[tauri::command]
pub fn move_unit(
    state: State<AppState>,
    faction: String,
    unit_id: u32,
    target_x: i32,
    target_y: i32,
    target_section: Direction,
) -> Result<GameState, String> {
    let mut game = state.0.lock().unwrap();

    let roster = game.military_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))?;

    let unit = roster.get_unit_mut(unit_id)
        .ok_or_else(|| format!("Unit {} not found in faction '{}'", unit_id, faction))?;

    // Get current location based on assignment
    let from_location = match &unit.assignment {
        UnitAssignment::Garrison { location, .. } => *location,
        UnitAssignment::Detached { location, .. } => *location,
        UnitAssignment::Ambush(coord, dir) => (*coord, *dir),
        UnitAssignment::Moving { from, .. } => *from,
        _ => return Err("Unit cannot move from current assignment".to_string()),
    };

    unit.assignment = UnitAssignment::Moving {
        from: from_location,
        to: (TileCoord { x: target_x, y: target_y }, target_section),
        progress: 0.0,
    };

    Ok(GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
        military_rosters: game.military_rosters.clone(),
    })
}

/// Attack a target section with multiple units
/// This is a convenience command that moves all specified units toward the target
/// Combat will resolve automatically when units from different factions occupy the same section
#[tauri::command]
pub fn attack_section(
    state: State<AppState>,
    faction: String,
    unit_ids: Vec<u32>,
    target_x: i32,
    target_y: i32,
    target_section: Direction,
) -> Result<GameState, String> {
    let mut game = state.0.lock().unwrap();

    if unit_ids.is_empty() {
        return Err("No units specified for attack".to_string());
    }

    let roster = game.military_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))?;

    let target_location = (TileCoord { x: target_x, y: target_y }, target_section);

    // Move all specified units toward the target
    for unit_id in &unit_ids {
        if let Some(unit) = roster.get_unit_mut(*unit_id) {
            // Get current location based on assignment
            let from_location = match &unit.assignment {
                UnitAssignment::Garrison { location, .. } => *location,
                UnitAssignment::Detached { location, .. } => *location,
                UnitAssignment::Ambush(coord, dir) => (*coord, *dir),
                UnitAssignment::Moving { from, .. } => *from,
                UnitAssignment::Attached(_) => continue, // Can't move attached units directly
                UnitAssignment::Recovering => continue, // Can't move recovering units
            };

            // Check if already at target (instant engagement)
            if from_location == target_location {
                // Unit is already there, no movement needed
                // Set to garrison to ensure combat detection picks it up
                unit.assignment = UnitAssignment::Garrison {
                    location: target_location,
                    is_stationed: false,
                };
            } else {
                // Set unit to moving toward target
                unit.assignment = UnitAssignment::Moving {
                    from: from_location,
                    to: target_location,
                    progress: 0.0,
                };
            }
        }
    }

    println!(
        "ATTACK ORDER: {} sends {} units to attack ({}, {}) {:?}",
        faction, unit_ids.len(), target_x, target_y, target_section
    );

    Ok(GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
        military_rosters: game.military_rosters.clone(),
    })
}

/// Recruit a new unit at a section with a Barracks (or SpawningNest for Lamia)
/// The unit is created and assigned to garrison at the specified location
#[tauri::command]
pub fn recruit_unit(
    state: State<AppState>,
    faction: String,
    template_id: String,
    name: String,
    strength: i32,
    tile_x: i32,
    tile_y: i32,
    section_dir: Direction,
) -> Result<GameState, String> {
    let mut game = state.0.lock().unwrap();

    // Check that the section has a recruitment building (Barracks or SpawningNest)
    let tile_key = format!("({}, {})", tile_x, tile_y);
    let section = game.map.get(&tile_key)
        .and_then(|tile| tile.sections.get(&section_dir))
        .ok_or_else(|| format!("Section ({}, {}) {:?} not found", tile_x, tile_y, section_dir))?;

    // Check ownership
    if section.ownership.as_ref() != Some(&faction) {
        return Err(format!("Section not owned by {}", faction));
    }

    // Check for recruitment building
    let has_recruitment_building = section.buildings.iter().any(|b| {
        matches!(b.building_type,
            BuildingType::Barracks | BuildingType::SpawningNest | BuildingType::BroodHive)
    });

    if !has_recruitment_building {
        return Err("Section needs a Barracks (Human) or SpawningNest/BroodHive (Lamia) to recruit units".to_string());
    }

    // Create the unit
    let roster = game.military_rosters.get_mut(&faction)
        .ok_or_else(|| format!("Military roster for faction '{}' not found", faction))?;

    let unit_id = roster.create_unit(name.clone(), &template_id, strength)?;

    // Assign to garrison at the recruitment location
    if let Some(unit) = roster.get_unit_mut(unit_id) {
        unit.assignment = UnitAssignment::Garrison {
            location: (TileCoord { x: tile_x, y: tile_y }, section_dir),
            is_stationed: true,
        };
    }

    println!(
        "RECRUITMENT: {} recruits {} '{}' (strength: {}) at ({}, {}) {:?}",
        faction, template_id, name, strength, tile_x, tile_y, section_dir
    );

    Ok(GameState {
        map: game.map.clone(),
        current_week: game.current_week,
        current_day: game.current_day,
        task_queue: game.task_queue.clone(),
        player_faction: game.player_faction,
        scout_rosters: game.scout_rosters.clone(),
        faction_dossiers: game.faction_dossiers.clone(),
        military_rosters: game.military_rosters.clone(),
    })
}

