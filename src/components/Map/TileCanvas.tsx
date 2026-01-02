import React, { useEffect, useRef, useState } from 'react';
import * as PIXI from 'pixi.js';

export enum Direction {
    N = "N",
    NE = "NE",
    E = "E",
    SE = "SE",
    S = "S",
    SW = "SW",
    W = "W",
    NW = "NW",
    Core = "Core",
}

interface SectionData {
    direction: string;
    terrain: string;
    ownership: string | null;
}

interface TileCanvasProps {
    width: number;
    height: number;
    sections?: Record<string, SectionData>;
    onSectionSelect: (dir: Direction) => void;
}

const TERRAIN_COLORS: Record<string, number> = {
    "Plains": 0x2a2a2a,
    "Forest": 0x14532d,
    "Mountain": 0x475569,
    "Swamp": 0x1e3a34,
    "Desert": 0x78350f,
};

const TileCanvas: React.FC<TileCanvasProps> = ({ width, height, sections, onSectionSelect }) => {
    const canvasRef = useRef<HTMLDivElement>(null);
    const appRef = useRef<PIXI.Application | null>(null);
    const containerRef = useRef<PIXI.Container | null>(null);
    const [hoveredDir, setHoveredDir] = useState<Direction | null>(null);

    useEffect(() => {
        let isMounted = true;

        const initPixi = async () => {
            const app = new PIXI.Application();
            await app.init({
                width,
                height,
                backgroundColor: 0x151515,
                antialias: true,
            });

            if (!isMounted) {
                app.destroy(true, { children: true });
                return;
            }

            if (canvasRef.current) {
                canvasRef.current.appendChild(app.canvas);
            }
            appRef.current = app;

            const container = new PIXI.Container();
            app.stage.addChild(container);
            containerRef.current = container;

            renderGrid();
        };

        if (!appRef.current) {
            initPixi();
        }

        return () => {
            isMounted = false;
            if (appRef.current) {
                appRef.current.destroy(true, { children: true });
                appRef.current = null;
                containerRef.current = null;
            }
        };
    }, []);

    useEffect(() => {
        renderGrid();
    }, [sections, width, height, hoveredDir]);

    const renderGrid = () => {
        if (!appRef.current || !containerRef.current) return;

        const container = containerRef.current;
        container.removeChildren();

        // 1. Integer-perfect math to prevent sub-pixel overlap
        const totalSize = Math.floor(Math.min(width, height) * 0.8 / 3) * 3;
        const sectionSize = totalSize / 3;
        const startX = Math.floor((width - totalSize) / 2);
        const startY = Math.floor((height - totalSize) / 2);

        const directions: Direction[][] = [
            [Direction.NW, Direction.N, Direction.NE],
            [Direction.W, Direction.Core, Direction.E],
            [Direction.SW, Direction.S, Direction.SE],
        ];

        // 2. Fills Layer
        const fills = new PIXI.Container();
        container.addChild(fills);

        for (let row = 0; row < 3; row++) {
            for (let col = 0; col < 3; col++) {
                const dir = directions[row][col];
                const sectionX = startX + col * sectionSize;
                const sectionY = startY + row * sectionSize;

                const sectionData = sections ? sections[dir] : null;
                const terrainType = sectionData?.terrain || "Plains";
                let baseColor = TERRAIN_COLORS[terrainType] || 0x2a2a2a;

                // Subtle Core tint
                if (dir === Direction.Core && terrainType === "Plains") {
                    baseColor = 0x313131;
                }

                const graphic = new PIXI.Graphics();
                graphic.rect(0, 0, sectionSize, sectionSize);
                graphic.fill({ color: baseColor, alpha: 1 });

                // Interaction
                graphic.interactive = true;
                graphic.cursor = 'pointer';
                graphic.on('pointerover', () => setHoveredDir(dir));
                graphic.on('pointerout', () => setHoveredDir(null));
                graphic.on('pointerdown', () => onSectionSelect(dir));

                graphic.position.set(sectionX, sectionY);
                fills.addChild(graphic);

                // Hover Highlight on top of fill
                if (hoveredDir === dir) {
                    const hoverOverlay = new PIXI.Graphics();
                    hoverOverlay.rect(0, 0, sectionSize, sectionSize);
                    hoverOverlay.fill({ color: 0x6336f1, alpha: 0.2 });
                    hoverOverlay.position.set(sectionX, sectionY);
                    fills.addChild(hoverOverlay);
                }
            }
        }

        // 3. Shared Grid Layer (Avoids double-thickness lines)
        const grid = new PIXI.Graphics();
        for (let i = 0; i <= 3; i++) {
            // Vertical lines
            grid.moveTo(startX + i * sectionSize, startY);
            grid.lineTo(startX + i * sectionSize, startY + totalSize);
            // Horizontal lines
            grid.moveTo(startX, startY + i * sectionSize);
            grid.lineTo(startX + totalSize, startY + i * sectionSize);
        }
        grid.stroke({ width: 1, color: 0x333333, alpha: 0.8 });
        container.addChild(grid);

        // 4. Highlight Border Layer (Drawn last to prevent overlap bleed)
        if (hoveredDir) {
            let hRow = -1, hCol = -1;
            for (let r = 0; r < 3; r++) {
                const c = directions[r].indexOf(hoveredDir);
                if (c !== -1) { hRow = r; hCol = c; break; }
            }

            if (hRow !== -1) {
                const highlight = new PIXI.Graphics();
                const hX = startX + hCol * sectionSize;
                const hY = startY + hRow * sectionSize;
                highlight.rect(0, 0, sectionSize, sectionSize);
                highlight.stroke({ width: 2, color: 0x6366f1, alignment: 1 }); // alignment 1 is outer, but since we are integer-snapped, its fine
                highlight.position.set(hX, hY);
                container.addChild(highlight);
            }
        }
    };

    return <div ref={canvasRef} style={{ width, height, overflow: 'hidden' }} />;
};

export default TileCanvas;
