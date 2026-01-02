import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';
import TileCanvas, { Direction } from './components/Map/TileCanvas';

interface Trait {
  name: string;
  value: number;
}

interface Building {
  buildingType: string;
  level: number;
  isActive: boolean;
}

interface Section {
  direction: string;
  terrain: string;
  ownership: string | null;
  ownershipFaction: string | null;
  population: number;
  traits: Trait[];
  buildings: Building[];
  abilities: any[];
  hiddenThreats: boolean;
}

interface Tile {
  x: number;
  y: number;
  sections: Record<string, Section>;
}

interface Task {
  name: string;
  buildingType: string;
  dayCost: number;
  progress: number;
  location: string;
  section: string;
}

interface GameState {
  map: Record<string, Tile>;
  currentWeek: number;
  currentDay: number;
  taskQueue: Task[];
  playerFaction: string;
}

const HUMAN_BUILDINGS = {
  core: ["Keep", "Forge", "Granary", "GuildHall"],
  outer: ["Farm", "LoggingCamp", "Quarry", "Watchtower", "HumanOutpost"]
};

const LAMIA_BUILDINGS = {
  core: ["BroodHive", "MoltingPit", "VerminPit"],
  outer: ["SpawningNest", "BonePile", "AmbushBlinds", "FloodHatchery", "ToxicBog"]
};

function App() {
  const [selectedDir, setSelectedDir] = useState<Direction | null>(null);
  const [selectedTile, setSelectedTile] = useState<{ x: number, y: number }>({ x: 0, y: 0 });
  const [gameState, setGameState] = useState<GameState | null>(null);

  const fetchMap = async () => {
    try {
      const state = await invoke<GameState>('get_map');
      setGameState(state);

      // Auto-select the player's starting tile on first load
      if (!gameState) {
        const spawnTile = Object.values(state.map).find(tile =>
          tile.sections["Core"]?.ownership === "Player"
        );
        if (spawnTile) {
          setSelectedTile({ x: spawnTile.x, y: spawnTile.y });
        }
      }
    } catch (e) {
      console.error("Failed to fetch map:", e);
    }
  };

  useEffect(() => {
    fetchMap();
  }, []);

  const handleNextTurn = async () => {
    try {
      const state = await invoke<GameState>('advance_turn');
      setGameState(state);
    } catch (e) {
      console.error("Failed to advance turn:", e);
    }
  }

  const handleAddTask = async (buildingType: string) => {
    if (!selectedDir || !gameState) return;
    try {
      const tileKey = `(${selectedTile.x}, ${selectedTile.y})`;
      const state = await invoke<GameState>('add_task', {
        name: buildingType,
        buildingType: buildingType,
        dayCost: 3, // default for prototype
        location: tileKey,
        section: selectedDir
      });
      setGameState(state);
    } catch (e) {
      console.error("Failed to add task:", e);
    }
  };

  const getSelectedSectionInfo = () => {
    if (!selectedDir || !gameState) return null;
    const tileKey = `(${selectedTile.x}, ${selectedTile.y})`;
    const tile = gameState.map[tileKey];
    if (!tile) return null;
    return tile.sections[selectedDir];
  };

  const selectedInfo = getSelectedSectionInfo();

  const getAvailableBuildings = () => {
    if (!selectedDir) return [];
    const faction = gameState?.playerFaction || "Human";
    const pools = faction === "Human" ? HUMAN_BUILDINGS : LAMIA_BUILDINGS;
    return selectedDir === Direction.Core ? pools.core : pools.outer;
  };

  const activeTileData = gameState?.map[`(${selectedTile.x}, ${selectedTile.y})`];

  return (
    <div className="app-container">
      <header className="game-header">
        <h1>Tile Wargame</h1>
        <div className="tile-selector">
          Tile:
          {[0, 1, 2].map(x => (
            <div key={`row-${x}`} className="tile-row">
              {[0, 1, 2].map(y => (
                <button
                  key={`${x}-${y}`}
                  className={`tile-node ${selectedTile.x === x && selectedTile.y === y ? 'active' : ''}`}
                  onClick={() => setSelectedTile({ x, y })}
                >
                  {x},{y}
                </button>
              ))}
            </div>
          ))}
        </div>
        <div className="time-display">
          <span>Week {gameState?.currentWeek || 1}</span>
          <span>Day {gameState?.currentDay || 1}</span>
          <button className="turn-btn" onClick={handleNextTurn}>End Day</button>
        </div>
      </header>
      <div className="game-layout">
        <main className="game-viewport">
          <TileCanvas
            width={800}
            height={600}
            sections={activeTileData?.sections}
            onSectionSelect={(dir) => setSelectedDir(dir)}
          />
        </main>
        <aside className="ui-sidebar">
          <div className="info-panel">
            <div className="panel-header">
              <h3>{selectedDir ? `Section: ${selectedDir}` : "Select a Section"}</h3>
            </div>
            {selectedInfo ? (
              <div className="section-stats">
                <p><strong>Terrain:</strong> {selectedInfo.terrain}</p>
                <p><strong>Ownership:</strong> {selectedInfo.ownership || "Neutral"}</p>
                <p><strong>Population:</strong> {selectedInfo.population.toLocaleString()}</p>

                <div className="build-menu">
                  <strong>Build:</strong>
                  <div className="build-options">
                    {getAvailableBuildings().map(b => (
                      <button
                        key={b}
                        className="build-btn"
                        onClick={() => handleAddTask(b)}
                        disabled={gameState?.taskQueue.some(t => t.section === selectedDir && t.location === `(${selectedTile.x}, ${selectedTile.y})`)}
                      >
                        + {b}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="structures-list">
                  <strong>Buildings:</strong>
                  {selectedInfo.buildings.length > 0 ? (
                    <ul>
                      {selectedInfo.buildings.map((b, i) => (
                        <li key={i}>{b.buildingType} (Lvl {b.level})</li>
                      ))}
                    </ul>
                  ) : <p>None</p>}
                </div>
              </div>
            ) : (
              <p>Hover and click any section on the map to see its logistical and tactical data.</p>
            )}
          </div>

          <div className="info-panel task-list">
            <h3>Scheduled Tasks</h3>
            {gameState?.taskQueue.length === 0 ? (
              <p className="empty-msg">No active tasks in the queue.</p>
            ) : (
              <ul>
                {gameState?.taskQueue.map((task, i) => (
                  <li key={i} className="task-item">
                    <div className="task-header">
                      <span>{task.name} ({task.section})</span>
                      <span>Tile: {task.location}</span>
                      <span>{task.progress}/{task.dayCost}d</span>
                    </div>
                    <div className="progress-bar">
                      <div className="progress-fill" style={{ width: `${(task.progress / task.dayCost) * 100}%` }}></div>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}

export default App;
