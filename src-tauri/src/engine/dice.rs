use rand::Rng;
use serde::{Deserialize, Serialize};

/// Dice types for combat resolution
/// Design principle: Attrition, not wipes. Variance affects DEGREE, not success/failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiceType {
    D10,      // 1-10 flat distribution
    TwoD10,   // 2-20 bell curve centered on 11
}

/// A combat roll with its results
#[derive(Debug, Clone)]
pub struct CombatRoll {
    pub dice_type: DiceType,
    pub roll: i32,
    pub modifier: i32,
}

impl CombatRoll {
    /// Roll dice with optional modifier
    pub fn roll(dice: DiceType, modifier: i32) -> Self {
        let mut rng = rand::thread_rng();
        let roll = match dice {
            DiceType::D10 => rng.gen_range(1..=10),
            DiceType::TwoD10 => rng.gen_range(1..=10) + rng.gen_range(1..=10),
        };
        Self { dice_type: dice, roll, modifier }
    }

    /// Get total (roll + modifier)
    pub fn total(&self) -> i32 {
        self.roll + self.modifier
    }

    /// Interpret roll as attrition multipliers
    /// Returns (attacker_mult, defender_mult)
    /// - Bad roll: attacker takes MORE, defender takes LESS
    /// - Good roll: attacker takes LESS, defender takes MORE
    /// This implements "attrition not wipes" - bad luck means YOU suffer more
    pub fn attrition_outcome(&self) -> (f32, f32) {
        let total = self.total();

        match self.dice_type {
            DiceType::D10 => {
                match total {
                    ..=2 => (1.5, 0.5),   // Bad: we take 50% more, they 50% less
                    3..=4 => (1.2, 0.8),  // Below average
                    5..=6 => (1.0, 1.0),  // Average: both normal
                    7..=8 => (0.8, 1.2),  // Above average
                    9.. => (0.5, 1.5),    // Good: we less, they more
                }
            }
            DiceType::TwoD10 => {
                // 2-20 range, bell curve centered on 11
                match total {
                    ..=5 => (1.5, 0.5),   // Bad (rare)
                    6..=8 => (1.2, 0.8),  // Below average
                    9..=13 => (1.0, 1.0), // Average (most common)
                    14..=16 => (0.8, 1.2),// Above average
                    17.. => (0.5, 1.5),   // Good (rare)
                }
            }
        }
    }
}

/// Apply attrition to both sides - NEVER wipe a unit from one roll
/// Units have a 10% floor - they can NEVER be reduced below 10% of max strength
/// from a single engagement
pub fn apply_combat_attrition(
    attacker_strength: &mut i32,
    attacker_max: i32,
    defender_strength: &mut i32,
    defender_max: i32,
    base_attacker_casualties: i32,
    base_defender_casualties: i32,
    roll: &CombatRoll,
) {
    let (att_mult, def_mult) = roll.attrition_outcome();

    let attacker_casualties = (base_attacker_casualties as f32 * att_mult) as i32;
    let defender_casualties = (base_defender_casualties as f32 * def_mult) as i32;

    // NEVER reduce below 10% of max strength from single engagement
    // This is the core "attrition not wipes" rule
    let attacker_floor = attacker_max / 10;
    let defender_floor = defender_max / 10;

    *attacker_strength = (*attacker_strength - attacker_casualties).max(attacker_floor);
    *defender_strength = (*defender_strength - defender_casualties).max(defender_floor);
}

/// Calculate base casualties from combat stats
/// This is a simple formula - can be expanded later
pub fn calculate_base_casualties(
    attacker_strength: i32,
    attacker_melee: i32,
    attacker_penetration: i32,
    defender_strength: i32,
    defender_armor: i32,
) -> (i32, i32) {
    // Attacker casualties: defender hits us
    let defender_damage = (defender_strength / 10).max(1);
    let attacker_casualties = defender_damage;

    // Defender casualties: we hit them (penetration reduces armor effectiveness)
    let effective_armor = (defender_armor - attacker_penetration).max(0);
    let attacker_damage = ((attacker_strength / 10) + (attacker_melee / 5) - (effective_armor / 5)).max(1);
    let defender_casualties = attacker_damage;

    (attacker_casualties, defender_casualties)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attrition_never_wipes() {
        let mut att = 100;
        let mut def = 100;

        // Even with terrible roll and high base casualties
        let roll = CombatRoll { dice_type: DiceType::D10, roll: 1, modifier: 0 };
        apply_combat_attrition(&mut att, 100, &mut def, 100, 200, 200, &roll);

        // Neither should go below 10% (floor of 10)
        assert!(att >= 10, "Attacker should not drop below floor");
        assert!(def >= 10, "Defender should not drop below floor");
    }

    #[test]
    fn test_d10_outcomes() {
        // Bad roll
        let roll = CombatRoll { dice_type: DiceType::D10, roll: 1, modifier: 0 };
        let (att, def) = roll.attrition_outcome();
        assert!(att > 1.0, "Bad roll: attacker should take more");
        assert!(def < 1.0, "Bad roll: defender should take less");

        // Good roll
        let roll = CombatRoll { dice_type: DiceType::D10, roll: 10, modifier: 0 };
        let (att, def) = roll.attrition_outcome();
        assert!(att < 1.0, "Good roll: attacker should take less");
        assert!(def > 1.0, "Good roll: defender should take more");
    }

    #[test]
    fn test_2d10_bell_curve() {
        // Average roll (11) should be 1:1
        let roll = CombatRoll { dice_type: DiceType::TwoD10, roll: 11, modifier: 0 };
        let (att, def) = roll.attrition_outcome();
        assert_eq!(att, 1.0);
        assert_eq!(def, 1.0);
    }

    #[test]
    fn test_floor_calculation() {
        let mut att = 15;
        let mut def = 15;
        let roll = CombatRoll { dice_type: DiceType::D10, roll: 5, modifier: 0 };

        // With max of 100, floor is 10
        apply_combat_attrition(&mut att, 100, &mut def, 100, 50, 50, &roll);

        // Should hit floor
        assert_eq!(att, 10);
        assert_eq!(def, 10);
    }
}
