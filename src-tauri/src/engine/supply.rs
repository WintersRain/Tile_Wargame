use crate::engine::GameState;
use crate::models::tile_model::{Direction, Tile};
use std::collections::{HashSet, VecDeque};

pub fn is_supplied(
    game: &GameState,
    start_pos: (i32, i32),
    start_dir: Direction,
    faction_id: &str,
) -> bool {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back((start_pos, start_dir));
    visited.insert((start_pos, start_dir));

    while let Some((pos, dir)) = queue.pop_front() {
        // 1. Check if this section is a supply source
        // For now, any Core section owned by the faction is a potential source/hub
        let pos_key = format!("({}, {})", pos.0, pos.1);
        if let Some(tile) = game.map.get(&pos_key) {
            if let Some(section) = tile.sections.get(&dir) {
                if section.ownership.as_deref() == Some(faction_id) {
                    if dir == Direction::Core {
                        // In a real game, we'd check for a 'Supply Hub' building here
                        return true;
                    }

                    // 2. Iterate neighbors
                    for (neigh_pos, neigh_dir) in Tile::get_neighbors(pos.0, pos.1, dir) {
                        if !visited.contains(&(neigh_pos, neigh_dir)) {
                            // Only traverse through owned sections
                            let neigh_key = format!("({}, {})", neigh_pos.0, neigh_pos.1);
                            if let Some(neigh_tile) = game.map.get(&neigh_key) {
                                if let Some(neigh_section) = neigh_tile.sections.get(&neigh_dir) {
                                    if neigh_section.ownership.as_deref() == Some(faction_id) {
                                        visited.insert((neigh_pos, neigh_dir));
                                        queue.push_back((neigh_pos, neigh_dir));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    false
}
