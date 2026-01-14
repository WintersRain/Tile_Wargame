use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::models::tile_model::Terrain;
use crate::models::military_model::{MilitaryUnit, CombatMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatOutcome {
    StandoffSuppression,
    MeleeEngagement,
    Retreat,
    Annihilation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementReport {
    pub outcome: CombatOutcome,
    pub attacker_losses: i32,
    pub defender_losses: i32,
    pub message: String,
}

pub fn resolve_engagement(
    attacker: &mut MilitaryUnit,
    defender: &mut MilitaryUnit,
    terrain: &Terrain,
) -> EngagementReport {
    let mut rng = rand::thread_rng();

    // Apply terrain modifiers
    let terrain_mobility = terrain.mobility_modifier();
    let terrain_accuracy = terrain.accuracy_modifier();
    let _terrain_stealth = terrain.stealth_modifier();

    // Effective stats with terrain
    let attacker_mobility_effective = (attacker.mobility as f32 * terrain_mobility) as i32;
    let defender_mobility_effective = (defender.mobility as f32 * terrain_mobility) as i32;
    let attacker_accuracy_effective = (attacker.accuracy as f32 * terrain_accuracy) as i32;

    // 1. Standoff Check (terrain-aware)
    let can_maintain_standoff =
        attacker.range > defender.range && attacker_mobility_effective >= defender_mobility_effective;

    if can_maintain_standoff {
        // Standoff suppression (attacker fires from range)
        let damage = (attacker_accuracy_effective - defender.armor).max(0) + rng.gen_range(1..6);
        defender.current_endurance -= damage;

        return EngagementReport {
            outcome: CombatOutcome::StandoffSuppression,
            attacker_losses: 0,
            defender_losses: damage / 10,
            message: format!(
                "{} maintains range advantage over {}. Standoff damage: {}",
                attacker.name, defender.name, damage
            ),
        };
    }

    // 2. Closing Attempt (terrain affects success)
    let closing_chance = (defender_mobility_effective as f32 / attacker_mobility_effective as f32) * 50.0;
    let roll = rng.gen_range(0..100);

    if roll < closing_chance as i32 {
        // 3. Successful Close - Melee Engagement

        // Penetration now reduces armor effectiveness
        let attacker_effective_pen = attacker.penetration.max(0);
        let defender_effective_armor = (defender.armor - attacker_effective_pen).max(0);
        let attacker_damage = (attacker.melee - defender_effective_armor).max(2) + rng.gen_range(1..10);

        let defender_effective_pen = defender.penetration.max(0);
        let attacker_effective_armor = (attacker.armor - defender_effective_pen).max(0);
        let defender_damage = (defender.melee - attacker_effective_armor).max(2) + rng.gen_range(1..10);

        attacker.current_endurance -= defender_damage;
        defender.current_endurance -= attacker_damage;

        // 4. Morale Check for Retreat
        if attacker.current_endurance < (attacker.morale / 2) && attacker.morale < 50 {
            return EngagementReport {
                outcome: CombatOutcome::Retreat,
                attacker_losses: (defender_damage / 10) + 5,  // Extra losses from panicked retreat
                defender_losses: attacker_damage / 10,
                message: format!("{} breaks and retreats!", attacker.name),
            };
        }

        if defender.current_endurance < (defender.morale / 2) && defender.morale < 50 {
            return EngagementReport {
                outcome: CombatOutcome::Retreat,
                attacker_losses: defender_damage / 10,
                defender_losses: (attacker_damage / 10) + 5,
                message: format!("{} breaks and retreats!", defender.name),
            };
        }

        // 5. Annihilation Check
        if defender.current_endurance <= 0 {
            return EngagementReport {
                outcome: CombatOutcome::Annihilation,
                attacker_losses: defender_damage / 10,
                defender_losses: 100,  // Total loss
                message: format!("{} annihilated!", defender.name),
            };
        }

        if attacker.current_endurance <= 0 {
            return EngagementReport {
                outcome: CombatOutcome::Annihilation,
                attacker_losses: 100,
                defender_losses: attacker_damage / 10,
                message: format!("{} annihilated!", attacker.name),
            };
        }

        // 6. Standard Melee Result
        EngagementReport {
            outcome: CombatOutcome::MeleeEngagement,
            attacker_losses: defender_damage / 10,
            defender_losses: attacker_damage / 10,
            message: format!(
                "{} successfully closed! Melee: {} dealt {} dmg, {} dealt {} dmg",
                defender.name, attacker.name, attacker_damage, defender.name, defender_damage
            ),
        }
    } else {
        // Failed Close -> Skirmish (Attacker fires unmolested)
        let damage = (attacker_accuracy_effective - defender.armor).max(5) + rng.gen_range(1..12);
        defender.current_endurance -= damage;

        EngagementReport {
            outcome: CombatOutcome::StandoffSuppression,
            attacker_losses: 0,
            defender_losses: damage / 10,
            message: format!(
                "{} failed to close. {} picks them off from afar. Damage: {}",
                defender.name, attacker.name, damage
            ),
        }
    }
}

/// Resolve an ambush engagement with surprise attack mechanics
/// Returns (engagement_report, should_continue_combat)
///
/// Ambushers in Guerilla mode strike then attempt to disengage.
/// Ambushers in Raiding mode strike then commit to full combat.
pub fn resolve_ambush(
    ambusher: &mut MilitaryUnit,
    victim: &mut MilitaryUnit,
    ambush_mode: &CombatMode,
    terrain: &Terrain,
) -> (EngagementReport, bool) {
    let mut rng = rand::thread_rng();

    // 1. Surprise Check: ambusher's stealth vs victim's perception
    let terrain_stealth = terrain.stealth_modifier();
    let ambusher_stealth_effective = (ambusher.stealth as f32 * terrain_stealth) as i32;

    let surprise_roll = rng.gen_range(0..100);
    let surprise_threshold = ambusher_stealth_effective - victim.perception;
    let surprise_success = surprise_roll < (50 + surprise_threshold);

    if !surprise_success {
        // Ambush detected! Victim sees it coming - reduce to standard engagement
        return (
            EngagementReport {
                outcome: CombatOutcome::MeleeEngagement,
                attacker_losses: 0,
                defender_losses: 0,
                message: format!(
                    "{} detected the ambush by {}! Combat begins normally.",
                    victim.name, ambusher.name
                ),
            },
            true, // Continue to standard combat
        );
    }

    // 2. FREE STRIKE: Ambusher attacks with no retaliation
    let terrain_accuracy = terrain.accuracy_modifier();
    let ambusher_accuracy_effective = (ambusher.accuracy as f32 * terrain_accuracy) as i32;

    // Ambush gives bonus to hit - point-blank surprise attack
    let ambush_bonus = 20;
    let base_damage = if ambusher.range > 0 {
        // Ranged ambush (archery, crossbows)
        (ambusher_accuracy_effective + ambush_bonus - victim.armor).max(5) + rng.gen_range(5..15)
    } else {
        // Melee ambush (penetration reduces armor)
        let effective_armor = (victim.armor - ambusher.penetration).max(0);
        (ambusher.melee + ambush_bonus - effective_armor).max(8) + rng.gen_range(8..20)
    };

    // Critical hit chance on ambush (30%)
    let crit_roll = rng.gen_range(0..100);
    let final_damage = if crit_roll < 30 {
        (base_damage as f32 * 1.5) as i32
    } else {
        base_damage
    };

    victim.current_endurance -= final_damage;
    let ambush_losses = final_damage / 10;

    // 3. Check if victim is eliminated by ambush
    if victim.current_endurance <= 0 {
        return (
            EngagementReport {
                outcome: CombatOutcome::Annihilation,
                attacker_losses: 0,
                defender_losses: 100,
                message: format!(
                    "AMBUSH! {} struck from hiding (dmg: {}). {} eliminated!",
                    ambusher.name, final_damage, victim.name
                ),
            },
            false, // No further combat needed
        );
    }

    // 4. Mode-specific behavior
    match ambush_mode {
        CombatMode::Guerilla => {
            // Guerilla: Strike and fade
            let terrain_mobility = terrain.mobility_modifier();
            let ambusher_mobility_effective = (ambusher.mobility as f32 * terrain_mobility) as i32;
            let victim_mobility_effective = (victim.mobility as f32 * terrain_mobility) as i32;

            let disengage_roll = rng.gen_range(0..100);
            let disengage_threshold = 40 + (ambusher_mobility_effective - victim_mobility_effective);

            if disengage_roll < disengage_threshold {
                // Successful disengagement
                (
                    EngagementReport {
                        outcome: CombatOutcome::Retreat,
                        attacker_losses: 0,
                        defender_losses: ambush_losses,
                        message: format!(
                            "GUERILLA AMBUSH! {} struck from hiding (dmg: {}), then melted away!",
                            ambusher.name, final_damage
                        ),
                    },
                    false, // No further combat - ambusher disengages
                )
            } else {
                // Failed to disengage - victim closes in
                (
                    EngagementReport {
                        outcome: CombatOutcome::MeleeEngagement,
                        attacker_losses: 0,
                        defender_losses: ambush_losses,
                        message: format!(
                            "AMBUSH! {} dealt {} damage, but {} closed in before escape!",
                            ambusher.name, final_damage, victim.name
                        ),
                    },
                    true, // Continue to melee combat
                )
            }
        }

        CombatMode::Raiding => {
            // Raiding: Strike and commit to destruction
            (
                EngagementReport {
                    outcome: CombatOutcome::MeleeEngagement,
                    attacker_losses: 0,
                    defender_losses: ambush_losses,
                    message: format!(
                        "RAIDING AMBUSH! {} struck from hiding (dmg: {}), now pressing the attack!",
                        ambusher.name, final_damage
                    ),
                },
                true, // Continue to full combat engagement
            )
        }

        _ => {
            // Default to standard combat for non-ambush modes
            (
                EngagementReport {
                    outcome: CombatOutcome::MeleeEngagement,
                    attacker_losses: 0,
                    defender_losses: ambush_losses,
                    message: format!(
                        "Ambush by {} dealt {} damage.",
                        ambusher.name, final_damage
                    ),
                },
                true,
            )
        }
    }
}
