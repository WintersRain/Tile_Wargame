from dataclasses import dataclass, field, asdict
from enum import Enum
from typing import List, Dict, Tuple, Optional
import random

# --- Enums & Constants ---

class AgentState(Enum):
    CONTENT = "content"
    HUNGRY = "hungry"
    ANGRY = "angry"
    REBELLIOUS = "rebellious"
    DEAD = "dead"

class AgentType(Enum):
    PEASANT = "peasant"
    SOLDIER = "soldier"
    NOBLE = "noble"

# --- Agent Definitions ---

@dataclass
class Agent:
    id: str
    type: AgentType
    x: int
    y: int
    
    # Needs & Stats (0-100)
    hunger: float = 0
    wealth: float = 10
    loyalty: float = 50
    anger: float = 0
    fear: float = 0
    
    state: AgentState = AgentState.CONTENT

    def tick(self, world_state: 'WorldSimulation') -> List[str]:
        """Process one turn for this agent."""
        events = []
        
        # 1. Metabolism (Hunger)
        self.hunger += 5
        
        # 2. Consumption (Try to buy food from global stock)
        # Using global bread_price from world settings
        bread_price = world_state.policies['bread_price']
        
        if self.hunger > 30:
            if self.wealth >= bread_price:
                # Agent pays wealth
                self.wealth -= bread_price
                # Money goes back to state treasury (simplified circular economy)
                world_state.game_state.resources['gold'] = world_state.game_state.resources.get('gold', 0) + bread_price
                
                # Consume food from global stock if available
                if world_state.game_state.resources.get('food', 0) > 0:
                    world_state.game_state.resources['food'] -= 1
                    self.hunger = 0
                else:
                    # Wealth spent but no food available!
                    self.anger += 10
                    events.append(f"{self.type.value} {self.id} paid for food but found none!")
            else:
                # Can't afford food
                self.hunger += 5
                self.anger += 2
                self.loyalty -= 1
        
        # 3. State Update
        self._update_state(events)
        
        return events

    def _update_state(self, events: List[str]):
        prev_state = self.state
        
        if self.hunger >= 100:
            self.state = AgentState.DEAD
            if prev_state != AgentState.DEAD:
                events.append(f"{self.type.value} {self.id} starved to death.")
        elif self.anger > 80 and self.loyalty < 20:
            self.state = AgentState.REBELLIOUS
        elif self.anger > 50:
            self.state = AgentState.ANGRY
        elif self.hunger > 50:
            self.state = AgentState.HUNGRY
        else:
            self.state = AgentState.CONTENT

@dataclass
class Peasant(Agent):
    """Produces resources based on tile terrain."""
    def tick(self, world_state: 'WorldSimulation') -> List[str]:
        events = super().tick(world_state)
        if self.state == AgentState.DEAD: return events

        # Production Logic
        tile = world_state.game_state.tiles.get((self.x, self.y))
        if tile and tile.prospected:
            # Peasants work the land
            # Simplified: Randomly find a resource on the tile to produce
            for res, amount in tile.resources.items():
                if amount > 0:
                    produced = 1 # Base production
                    # Add to global stock
                    world_state.game_state.resources[res] = world_state.game_state.resources.get(res, 0) + produced
                    # Peasant gets paid wage
                    wage = world_state.policies['wage_rate']
                    self.wealth += wage
                    # State pays wage
                    world_state.game_state.resources['gold'] = max(0, world_state.game_state.resources.get('gold', 0) - wage)
                    break 
        return events

@dataclass
class Soldier(Agent):
    """Consumes upkeep, provides 'fear' (order) to local tile."""
    def tick(self, world_state: 'WorldSimulation') -> List[str]:
        events = super().tick(world_state)
        if self.state == AgentState.DEAD: return events

        # Soldiers act as a suppression force
        # Find other agents on this tile
        local_agents = world_state.get_agents_at(self.x, self.y)
        for agent in local_agents:
            if agent.id != self.id:
                agent.fear += 5
                # Cap fear
                if agent.fear > 100: agent.fear = 100
        
        # Soldiers need regular pay or they defect
        upkeep = world_state.policies['soldier_pay']
        if world_state.game_state.resources.get('gold', 0) >= upkeep:
             world_state.game_state.resources['gold'] -= upkeep
             self.wealth += upkeep
             self.loyalty += 1
        else:
            self.loyalty -= 5
            self.anger += 5
            events.append(f"Soldier {self.id} was not paid!")

        return events

@dataclass
class Noble(Agent):
    """Consumes luxury, boosts loyalty of peasants, plots if ambition high."""
    ambition: float = 50

    def tick(self, world_state: 'WorldSimulation') -> List[str]:
        events = [] # Nobles handle their own hunger differently (luxury)
        
        # Nobles tax their local tile (taking a cut of global tax? or generating wealth?)
        # Let's say they generate 'influence' or 'political capital' (abstracted as gold for now)
        tax_income = 50
        self.wealth += tax_income
        
        # Loyalty aura
        local_agents = world_state.get_agents_at(self.x, self.y)
        for agent in local_agents:
            if agent.type == AgentType.PEASANT:
                agent.loyalty += 1
        
        # Ambition check
        if self.ambition > 80 and self.wealth > 500:
            events.append(f"Noble {self.id} is plotting a coup!")
            
        return events

# --- Simulation Manager ---

class WorldSimulation:
    def __init__(self, game_state):
        self.game_state = game_state # Reference to the existing GameState instance
        self.agents: List[Agent] = []
        self.policies = {
            "bread_price": 5,
            "wage_rate": 2,
            "soldier_pay": 10,
            "tax_rate": 0.1
        }
        self.tick_count = 0

    def spawn_agent(self, type: AgentType, x: int, y: int) -> Agent:
        """Create and register a new agent."""
        uid = f"{type.value}_{len(self.agents) + 1}"
        
        if type == AgentType.PEASANT:
            agent = Peasant(id=uid, type=type, x=x, y=y)
        elif type == AgentType.SOLDIER:
            agent = Soldier(id=uid, type=type, x=x, y=y)
        elif type == AgentType.NOBLE:
            agent = Noble(id=uid, type=type, x=x, y=y)
        else:
            agent = Agent(id=uid, type=type, x=x, y=y)
            
        self.agents.append(agent)
        return agent

    def get_agents_at(self, x: int, y: int) -> List[Agent]:
        """Spatial query helper."""
        return [a for a in self.agents if a.x == x and a.y == y and a.state != AgentState.DEAD]

    def tick(self) -> Dict:
        """Run one simulation cycle."""
        self.tick_count += 1
        turn_events = []
        
        # 1. Agent Ticks
        for agent in self.agents:
            if agent.state != AgentState.DEAD:
                agent_events = agent.tick(self)
                turn_events.extend(agent_events)

        # 2. Cleanup Dead
        # (Optional: remove dead agents or leave them as bodies)
        # self.agents = [a for a in self.agents if a.state != AgentState.DEAD]

        # 3. Aggregate Stats
        stats = {
            "population": len([a for a in self.agents if a.state != AgentState.DEAD]),
            "rebels": len([a for a in self.agents if a.state == AgentState.REBELLIOUS]),
            "deaths": len([a for a in self.agents if a.state == AgentState.DEAD]),
            "avg_loyalty": 0
        }
        
        if stats["population"] > 0:
            total_loyalty = sum(a.loyalty for a in self.agents if a.state != AgentState.DEAD)
            stats["avg_loyalty"] = total_loyalty / stats["population"]

        # 4. Emergent Events
        emergent_events = []
        if stats["rebels"] > stats["population"] * 0.2:
            emergent_events.append("CIVIL UNREST: The peasantry is arming themselves!")
        
        if stats["avg_loyalty"] < 30:
            emergent_events.append("DISSENT: The people despise your rule.")

        return {
            "tick": self.tick_count,
            "stats": stats,
            "agent_events": turn_events,
            "global_events": emergent_events
        }
