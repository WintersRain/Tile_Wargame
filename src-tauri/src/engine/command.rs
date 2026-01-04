use rand::Rng;
use crate::models::army_model::Army;
use crate::models::commander_model::{Commander, CommanderAssignment, CommanderRoster};
use crate::models::military_model::MilitaryRoster;

/// Result of commander death - either step-up or leaderless
pub enum CommanderDeathResult {
    StepUp {
        new_commander: Commander,
        morale_loss: i32,
    },
    NoStepUp {
        morale_loss: i32,
        cohesion_loss: f32,
        rout_risk: f32,
    },
}

/// Handle commander death - someone may step up from the ranks
pub fn handle_commander_death(
    army: &mut Army,
    _dead_commander: &Commander,
    military_roster: &MilitaryRoster,
    commander_roster: &mut CommanderRoster,
) -> CommanderDeathResult {
    let mut rng = rand::thread_rng();

    // Find most experienced unit in army
    let best_unit = army.unit_ids.iter()
        .filter_map(|id| military_roster.get_unit(*id))
        .max_by_key(|u| u.experience);

    // Step-up chance based on unit experience
    let step_up_chance = match best_unit {
        Some(unit) if unit.experience > 500 => 0.8,  // Veteran likely has capable NCO
        Some(unit) if unit.experience > 250 => 0.5,  // Competent might step up
        Some(_) => 0.3,                               // Green troops struggle
        None => 0.0,                                  // No units = no one to step up
    };

    if rng.gen::<f32>() < step_up_chance {
        // Someone steps up - create field-promoted commander
        let new_id = commander_roster.next_id;
        commander_roster.next_id += 1;

        let new_commander = Commander {
            id: new_id,
            name: format!("Field Officer (promoted)"),
            faction: army.faction.clone(),
            experience: best_unit.map(|u| u.experience / 4).unwrap_or(50),
            battles_fought: 0,
            victories: 0,
            tactics: rng.gen_range(20..50),
            logistics: rng.gen_range(20..40),
            inspiration: rng.gen_range(30..60),
            cunning: rng.gen_range(20..50),
            traits: vec![],
            assignment: CommanderAssignment::LeadingArmy(army.id),
            is_alive: true,
            location: Some(army.location),
        };

        commander_roster.commanders.push(new_commander.clone());
        army.commander_id = Some(new_id);

        CommanderDeathResult::StepUp {
            new_commander,
            morale_loss: 10,  // Some loss, but leadership maintained
        }
    } else {
        // No one steps up - army temporarily leaderless
        army.commander_id = None;

        CommanderDeathResult::NoStepUp {
            morale_loss: 30,       // Significant but not catastrophic
            cohesion_loss: 0.3,
            rout_risk: 0.2,        // 20% chance of immediate rout
        }
    }
}

/// Apply morale loss to all units in an army
pub fn apply_army_morale_loss(
    army: &Army,
    morale_loss: i32,
    military_roster: &mut MilitaryRoster,
) {
    for unit_id in &army.unit_ids {
        if let Some(unit) = military_roster.get_unit_mut(*unit_id) {
            unit.morale = (unit.morale - morale_loss).max(0);
        }
    }
}

/// Check if army routs based on rout risk
pub fn check_army_rout(rout_risk: f32) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen::<f32>() < rout_risk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rout_check() {
        // Rout risk 0 should never rout
        let mut any_routed = false;
        for _ in 0..100 {
            if check_army_rout(0.0) {
                any_routed = true;
            }
        }
        assert!(!any_routed, "0% rout risk should never trigger");

        // Rout risk 1 should always rout
        let mut all_routed = true;
        for _ in 0..100 {
            if !check_army_rout(1.0) {
                all_routed = false;
            }
        }
        assert!(all_routed, "100% rout risk should always trigger");
    }
}
