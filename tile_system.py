"""
Core tile system for the wargame.
Tiles have resources, 8-directional sections, and upgrades.
"""

from dataclasses import dataclass, field
from typing import Dict, Optional
from enum import Enum
import random


class Terrain(Enum):
    PLAINS = "plains"
    MOUNTAIN = "mountain"
    FOREST = "forest"
    SWAMP = "swamp"
    DESERT = "desert"


class Direction(Enum):
    NORTH = "N"
    NORTHEAST = "NE"
    EAST = "E"
    SOUTHEAST = "SE"
    SOUTH = "S"
    SOUTHWEST = "SW"
    WEST = "W"
    NORTHWEST = "NW"


@dataclass
class TileSection:
    """One of 8 sections in a tile"""
    direction: Direction
    owner: Optional[str] = None
    explored: bool = False


@dataclass
class TileData:
    """A single map tile with resources and ownership"""
    x: int
    y: int
    terrain: Terrain

    # Resources with specific quantities
    resources: Dict[str, int] = field(default_factory=dict)
    prospected: bool = False

    # 8 directional sections
    sections: Dict[Direction, TileSection] = field(default_factory=dict)

    # Per-tile upgrades
    upgrades: Dict[str, int] = field(default_factory=dict)

    # Terrain effects
    defense_bonus: int = 0
    movement_cost: int = 1
    is_chokepoint: bool = False

    def __post_init__(self):
        # Initialize all 8 sections if not provided
        if not self.sections:
            for direction in Direction:
                self.sections[direction] = TileSection(direction=direction)

        # Set terrain effects based on type
        if self.terrain == Terrain.MOUNTAIN:
            self.defense_bonus = 50  # +50% defense
            self.movement_cost = 2   # 2x cost to cross
        elif self.terrain == Terrain.FOREST:
            self.defense_bonus = 25
            self.movement_cost = 1.5
        elif self.terrain == Terrain.SWAMP:
            self.defense_bonus = 10
            self.movement_cost = 3

        # Generate initial resources if not provided
        if not self.resources and not self.prospected:
            self._generate_resources()

    def _generate_resources(self):
        """Generate random resources based on terrain"""
        if self.terrain == Terrain.MOUNTAIN:
            self.resources = {
                "iron": random.randint(200, 600),
                "stone": random.randint(500, 1500),
                "gold": random.randint(0, 100)
            }
        elif self.terrain == Terrain.FOREST:
            self.resources = {
                "wood": random.randint(800, 2000),
                "game": random.randint(100, 400),
                "herbs": random.randint(50, 200)
            }
        elif self.terrain == Terrain.PLAINS:
            self.resources = {
                "grain": random.randint(500, 1200),
                "livestock": random.randint(100, 300),
                "stone": random.randint(100, 400)
            }
        elif self.terrain == Terrain.SWAMP:
            self.resources = {
                "herbs": random.randint(200, 500),
                "peat": random.randint(300, 800),
                "fish": random.randint(100, 300)
            }
        elif self.terrain == Terrain.DESERT:
            self.resources = {
                "salt": random.randint(200, 600),
                "gems": random.randint(0, 50),
                "stone": random.randint(300, 700)
            }

    def prospect(self) -> Dict[str, int]:
        """Reveal actual resource quantities"""
        self.prospected = True
        return self.resources

    def extract_resource(self, resource: str, amount: int) -> int:
        """Extract resources from this tile"""
        if resource not in self.resources:
            return 0

        extracted = min(amount, self.resources[resource])
        self.resources[resource] -= extracted
        return extracted

    def get_owner(self) -> Optional[str]:
        """Get majority owner of tile (owns 5+ sections)"""
        owner_counts = {}
        for section in self.sections.values():
            if section.owner:
                owner_counts[section.owner] = owner_counts.get(section.owner, 0) + 1

        for owner, count in owner_counts.items():
            if count >= 5:  # Majority control
                return owner
        return None

    def is_contested(self) -> bool:
        """Check if multiple factions own sections"""
        owners = set()
        for section in self.sections.values():
            if section.owner:
                owners.add(section.owner)
        return len(owners) > 1