"""
Game state management - just tiles and resources, no character management.
"""

from dataclasses import dataclass, field
from typing import Dict, List, Tuple, Optional
from tile_system import TileData, Terrain, Direction
import random


@dataclass
class GameState:
    """Core game state - focused on tiles and territory"""
    width: int = 20
    height: int = 20
    tiles: Dict[Tuple[int, int], TileData] = field(default_factory=dict)

    # Player resources (global)
    resources: Dict[str, int] = field(default_factory=dict)

    # Current player position
    cursor_x: int = 10
    cursor_y: int = 10

    # Mining operations
    mining_operations: List[Dict] = field(default_factory=list)

    def __post_init__(self):
        if not self.tiles:
            self.generate_map()

        # Initialize player resources
        if not self.resources:
            self.resources = {
                "iron": 100,
                "wood": 200,
                "stone": 150,
                "gold": 50,
                "food": 300
            }

    def generate_map(self):
        """Generate a map with varied terrain"""
        # Create terrain clusters for more realistic maps
        terrain_seeds = []
        for _ in range(15):  # 15 terrain clusters
            terrain_seeds.append({
                'x': random.randint(0, self.width - 1),
                'y': random.randint(0, self.height - 1),
                'terrain': random.choice(list(Terrain))
            })

        # Generate tiles based on proximity to seeds
        for x in range(self.width):
            for y in range(self.height):
                # Find nearest seed
                min_dist = float('inf')
                chosen_terrain = Terrain.PLAINS

                for seed in terrain_seeds:
                    dist = abs(seed['x'] - x) + abs(seed['y'] - y)
                    if dist < min_dist:
                        min_dist = dist
                        chosen_terrain = seed['terrain']

                # Add some randomness
                if random.random() < 0.2:
                    chosen_terrain = random.choice(list(Terrain))

                self.tiles[(x, y)] = TileData(x=x, y=y, terrain=chosen_terrain)

        # Mark some mountain passes as chokepoints
        self._identify_chokepoints()

    def _identify_chokepoints(self):
        """Find natural chokepoints between mountains"""
        for x in range(1, self.width - 1):
            for y in range(1, self.height - 1):
                tile = self.tiles.get((x, y))
                if not tile or tile.terrain == Terrain.MOUNTAIN:
                    continue

                # Check if surrounded by mountains on opposite sides
                north = self.tiles.get((x, y - 1))
                south = self.tiles.get((x, y + 1))
                east = self.tiles.get((x + 1, y))
                west = self.tiles.get((x - 1, y))

                # Vertical chokepoint
                if (north and north.terrain == Terrain.MOUNTAIN and
                    south and south.terrain == Terrain.MOUNTAIN):
                    tile.is_chokepoint = True
                    tile.defense_bonus += 25

                # Horizontal chokepoint
                if (east and east.terrain == Terrain.MOUNTAIN and
                    west and west.terrain == Terrain.MOUNTAIN):
                    tile.is_chokepoint = True
                    tile.defense_bonus += 25

    def get_adjacent_tiles(self, tile: TileData, range: int = 1) -> List[TileData]:
        """Get tiles within range (for mining expansion)"""
        adjacent = []
        for dx in range(-range, range + 1):
            for dy in range(-range, range + 1):
                if dx == 0 and dy == 0:
                    continue

                x = tile.x + dx
                y = tile.y + dy

                if (x, y) in self.tiles:
                    adjacent.append(self.tiles[(x, y)])

        return adjacent

    def start_mining_operation(self, tile: TileData, resource: str) -> bool:
        """Start mining on a tile"""
        if not tile.prospected:
            return False  # Must prospect first

        if resource not in tile.resources or tile.resources[resource] == 0:
            return False

        # Check if we can afford to start mining
        mining_level = tile.upgrades.get("mining", 0)
        cost = 50 * (mining_level + 1)  # Increasing cost per level

        if self.resources.get("gold", 0) < cost:
            return False

        self.resources["gold"] -= cost
        tile.upgrades["mining"] = mining_level + 1

        # Create mining operation
        self.mining_operations.append({
            "tile": tile,
            "resource": resource,
            "range": 1 + mining_level  # Higher level = larger range
        })

        return True

    def process_mining_turn(self):
        """Process one turn of all mining operations"""
        for operation in self.mining_operations:
            tile = operation["tile"]
            resource = operation["resource"]
            mining_level = tile.upgrades.get("mining", 1)

            # Base extraction rate increases with mining level
            extraction_rate = 10 * mining_level

            # Try to extract from main tile
            extracted = tile.extract_resource(resource, extraction_rate)

            # If we couldn't get enough, expand to adjacent tiles
            if extracted < extraction_rate:
                remaining = extraction_rate - extracted
                adjacent_tiles = self.get_adjacent_tiles(tile, operation["range"])

                for adj_tile in adjacent_tiles:
                    if adj_tile.prospected and resource in adj_tile.resources:
                        additional = adj_tile.extract_resource(resource, remaining)
                        extracted += additional
                        remaining -= additional

                        if remaining <= 0:
                            break

            # Add extracted resources to player stockpile
            if extracted > 0:
                self.resources[resource] = self.resources.get(resource, 0) + extracted

    def build_upgrade(self, tile: TileData, upgrade_type: str) -> bool:
        """Build or upgrade something on a tile"""
        current_level = tile.upgrades.get(upgrade_type, 0)

        # Define costs for different upgrade types
        costs = {
            "guard_tower": {"stone": 100, "wood": 50, "gold": 25},
            "border_patrol": {"gold": 50, "food": 100},
            "fortification": {"stone": 200 * (current_level + 1)},
            "mining": {"wood": 100, "stone": 50, "gold": 50 * (current_level + 1)}
        }

        if upgrade_type not in costs:
            return False

        # Check if we can afford it
        cost = costs[upgrade_type]
        for resource, amount in cost.items():
            if self.resources.get(resource, 0) < amount:
                return False

        # Pay the cost
        for resource, amount in cost.items():
            self.resources[resource] -= amount

        # Build the upgrade
        tile.upgrades[upgrade_type] = current_level + 1

        # Apply effects
        if upgrade_type == "fortification":
            tile.defense_bonus += 20

        return True

    def claim_tile_section(self, tile: TileData, direction: Direction, owner: str = "player"):
        """Claim a section of a tile"""
        section = tile.sections[direction]
        section.owner = owner
        section.explored = True

        # Check if we should explore adjacent sections
        # If we own N and W, we also get NW
        if direction == Direction.NORTH:
            if tile.sections[Direction.WEST].owner == owner:
                tile.sections[Direction.NORTHWEST].owner = owner
                tile.sections[Direction.NORTHWEST].explored = True
            if tile.sections[Direction.EAST].owner == owner:
                tile.sections[Direction.NORTHEAST].owner = owner
                tile.sections[Direction.NORTHEAST].explored = True
        elif direction == Direction.SOUTH:
            if tile.sections[Direction.WEST].owner == owner:
                tile.sections[Direction.SOUTHWEST].owner = owner
                tile.sections[Direction.SOUTHWEST].explored = True
            if tile.sections[Direction.EAST].owner == owner:
                tile.sections[Direction.SOUTHEAST].owner = owner
                tile.sections[Direction.SOUTHEAST].explored = True