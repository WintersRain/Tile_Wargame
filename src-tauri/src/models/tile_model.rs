use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Coordinate of a tile on the map
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
    Core,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terrain {
    Plains,
    Forest,
    Mountain,
    Swamp,
    Desert,
}

impl Terrain {
    /// Get mobility modifier (multiplier for closing chance)
    pub fn mobility_modifier(&self) -> f32 {
        match self {
            Terrain::Plains => 1.0,
            Terrain::Forest => 0.8,    // -20% mobility
            Terrain::Swamp => 0.5,     // -50% mobility (unless Amphibian trait)
            Terrain::Mountain => 0.6,  // -40% mobility (unless Climbing trait)
            Terrain::Desert => 0.9,    // -10% mobility
        }
    }

    /// Get accuracy modifier for ranged attacks
    pub fn accuracy_modifier(&self) -> f32 {
        match self {
            Terrain::Plains => 1.2,    // +20% accuracy (clear LoS)
            Terrain::Forest => 0.7,    // -30% accuracy (obstructed)
            Terrain::Swamp => 0.8,     // -20% accuracy
            Terrain::Mountain => 1.1,  // +10% accuracy (elevation)
            Terrain::Desert => 1.0,    // No modifier
        }
    }

    /// Get stealth modifier (defender concealment bonus)
    pub fn stealth_modifier(&self) -> f32 {
        match self {
            Terrain::Plains => 0.8,    // -20% stealth (nowhere to hide)
            Terrain::Forest => 1.5,    // +50% stealth
            Terrain::Swamp => 1.3,     // +30% stealth
            Terrain::Mountain => 1.2,  // +20% stealth
            Terrain::Desert => 0.9,    // -10% stealth
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Human,
    Lamia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingType {
    // Human Buildings
    Keep,
    Barracks,       // Enables unit recruitment
    Forge,
    Granary,
    GuildHall,
    Farm,
    LoggingCamp,
    Quarry,
    Watchtower,
    Wall,           // +30% defense bonus
    HumanOutpost,

    // Lamia Buildings
    BroodHive,
    MoltingPit,
    VerminPit,
    SpawningNest,   // Lamia equivalent of Barracks
    BonePile,
    AmbushBlinds,
    FloodHatchery,
    ToxicBog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    pub building_type: BuildingType,
    pub level: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trait {
    pub name: String,
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ability {
    pub name: String,
    pub aoe_radius: i32,
    pub effect_value: i32,
    pub stamina_cost: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub direction: Direction,
    pub terrain: Terrain,
    pub ownership: Option<String>,
    pub ownership_faction: Option<Faction>,
    pub population: i32,
    pub traits: Vec<Trait>,
    pub buildings: Vec<Building>,
    pub abilities: Vec<Ability>,
    pub hidden_threats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub sections: HashMap<Direction, Section>,
}

impl Tile {
    pub fn new(x: i32, y: i32, terrain: Terrain) -> Self {
        let mut sections = HashMap::new();
        let dirs = [
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

        for dir in dirs {
            sections.insert(
                dir,
                Section {
                    direction: dir,
                    terrain: terrain, // Base terrain for the whole tile
                    ownership: None,
                    ownership_faction: None,
                    population: 0, // Population is managed by spawn logic
                    traits: Vec::new(),
                    buildings: Vec::new(),
                    abilities: Vec::new(),
                    hidden_threats: false,
                },
            );
        }

        Tile { x, y, sections }
    }

    pub fn are_sections_adjacent(d1: Direction, d2: Direction) -> bool {
        if d1 == Direction::Core || d2 == Direction::Core {
            return true;
        }

        match (d1, d2) {
            (Direction::N, Direction::NW) | (Direction::NW, Direction::N) => true,
            (Direction::N, Direction::NE) | (Direction::NE, Direction::N) => true,
            (Direction::E, Direction::NE) | (Direction::NE, Direction::E) => true,
            (Direction::E, Direction::SE) | (Direction::SE, Direction::E) => true,
            (Direction::S, Direction::SE) | (Direction::SE, Direction::S) => true,
            (Direction::S, Direction::SW) | (Direction::SW, Direction::S) => true,
            (Direction::W, Direction::SW) | (Direction::SW, Direction::W) => true,
            (Direction::W, Direction::NW) | (Direction::NW, Direction::W) => true,
            _ => false,
        }
    }

    pub fn get_neighbors(x: i32, y: i32, dir: Direction) -> Vec<((i32, i32), Direction)> {
        let mut neighbors = Vec::new();

        // Internal neighbors (Core)
        if dir != Direction::Core {
            neighbors.push(((x, y), Direction::Core));
        } else {
            let outer_dirs = [
                Direction::N,
                Direction::NE,
                Direction::E,
                Direction::SE,
                Direction::S,
                Direction::SW,
                Direction::W,
                Direction::NW,
            ];
            for od in outer_dirs {
                neighbors.push(((x, y), od));
            }
        }

        // Internal neighbors (Adjacency)
        let outer_dirs = [
            Direction::N,
            Direction::NE,
            Direction::E,
            Direction::SE,
            Direction::S,
            Direction::SW,
            Direction::W,
            Direction::NW,
        ];
        for od in outer_dirs {
            if Tile::are_sections_adjacent(dir, od) {
                neighbors.push(((x, y), od));
            }
        }

        // External neighbors (Tile boundaries)
        match dir {
            Direction::N => neighbors.push(((x, y - 1), Direction::S)),
            Direction::S => neighbors.push(((x, y + 1), Direction::N)),
            Direction::E => neighbors.push(((x + 1, y), Direction::W)),
            Direction::W => neighbors.push(((x - 1, y), Direction::E)),
            Direction::NE => {
                neighbors.push(((x + 1, y), Direction::NW));
                neighbors.push(((x, y - 1), Direction::SE));
                neighbors.push(((x + 1, y - 1), Direction::SW));
            }
            Direction::NW => {
                neighbors.push(((x - 1, y), Direction::NE));
                neighbors.push(((x, y - 1), Direction::SW));
                neighbors.push(((x - 1, y - 1), Direction::SE));
            }
            Direction::SE => {
                neighbors.push(((x + 1, y), Direction::SW));
                neighbors.push(((x, y + 1), Direction::NE));
                neighbors.push(((x + 1, y + 1), Direction::NW));
            }
            Direction::SW => {
                neighbors.push(((x - 1, y), Direction::SE));
                neighbors.push(((x, y + 1), Direction::NW));
                neighbors.push(((x - 1, y + 1), Direction::NE));
            }
            Direction::Core => {}
        }

        neighbors
    }
}
