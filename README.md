# Tile Wargame

**STATUS: Early Development - Not Playable**

## What Is This?

A strategy wargame where capturing territory is intentionally difficult. Instead of painting the map, you maneuver around fortified positions, probe for weaknesses, and exploit gaps.

## Core Concept

**Tiles are hard to take.** This is the point.

When tiles are easy to capture, strategy devolves into "more resources wins." When tiles are hard to capture, you have to think:

- Can I bypass this fortress and cut their supply?
- Should I feint north to pull defenders, then strike south?
- Do I invest 30 turns fortifying this border, or expand faster and risk a raid?

## How Tiles Work

Each tile has 9 zones: 8 directional sections (N, NE, E, SE, S, SW, W, NW) and a central Core.

Each section has its own terrain. The northern edge might be cliffs. The south might be open plains. This means:

- Attacking from the north = massive penalties (climbing cliffs under fire)
- Attacking from the south = standard engagement
- Defenders can't be equally strong everywhere

You scout to find the weak approach. You set ambushes on predicted bypass routes. You build where it matters.

## Turn Structure

Turns represent weeks. Tasks are measured in days.

1. Construction & maintenance resolve
2. Scouts move and trigger traps/ambushes
3. Armies move, combat resolves (WeGo - both sides planned last turn)
4. End-of-turn scouting

Poor scheduling wastes production days. Planning matters.

## Tech Stack

| Layer | Tech | Purpose |
|-------|------|---------|
| Shell | Tauri | Native window, file access |
| Simulation | Rust | Combat math, AI, pathfinding |
| Graphics | PixiJS | Tile rendering (planned) |
| UI | React + TypeScript | Info panels, building trees |

## Running Locally

Requires: Node.js, Rust, Tauri CLI

```bash
npm install
npm run tauri dev
```

## Current State

This project is in early development. The foundational code structure is in place, but the core gameplay loop hasn't been implemented yet. If you're looking for something playable, check back later.
