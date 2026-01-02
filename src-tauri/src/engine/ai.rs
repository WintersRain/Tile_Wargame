use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPersonality {
    pub name: String,
    pub weights: HashMap<AIObjective, f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AIObjective {
    Aggression,  // Weight for attacking known enemies
    Expansion,   // Weight for claiming neutral territory
    Defense,     // Weight for reinforcing existing territory
    Exploration, // Weight for scouting unknown regions
    Diplomacy,   // Weight for trade and neutral interaction
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAction {
    pub objective: AIObjective,
    pub description: String,
    pub score: f32,
    pub location: (i32, i32),
}

impl AIPersonality {
    pub fn new_aggressive() -> Self {
        let mut weights = HashMap::new();
        weights.insert(AIObjective::Aggression, 1.2);
        weights.insert(AIObjective::Expansion, 0.8);
        weights.insert(AIObjective::Defense, 0.5);
        weights.insert(AIObjective::Exploration, 0.7);
        weights.insert(AIObjective::Diplomacy, 0.2);

        Self {
            name: "Aggressive".to_string(),
            weights,
        }
    }

    pub fn new_expansionist() -> Self {
        let mut weights = HashMap::new();
        weights.insert(AIObjective::Aggression, 0.5);
        weights.insert(AIObjective::Expansion, 1.5);
        weights.insert(AIObjective::Defense, 0.7);
        weights.insert(AIObjective::Exploration, 1.2);
        weights.insert(AIObjective::Diplomacy, 1.0);

        Self {
            name: "Expansionist".to_string(),
            weights,
        }
    }

    pub fn calculate_utility(&self, objective: AIObjective, base_value: f32) -> f32 {
        let weight = self.weights.get(&objective).unwrap_or(&1.0);
        base_value * weight
    }

    pub fn evaluate_actions(
        &self,
        available_actions: Vec<(AIObjective, f32, String, (i32, i32))>,
    ) -> Vec<AIAction> {
        let mut scored_actions: Vec<AIAction> = available_actions
            .into_iter()
            .map(|(obj, val, desc, loc)| AIAction {
                objective: obj,
                description: desc,
                score: self.calculate_utility(obj, val),
                location: loc,
            })
            .collect();

        // Sort by highest score
        scored_actions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored_actions
    }
}
