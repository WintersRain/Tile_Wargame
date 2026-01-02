"""
Minimal tile-based wargame UI.
Focus on tiles, resources, and territory - no character management.
"""

import curses
from game_state import GameState
from tile_system import Direction, Terrain


class Game:
    def __init__(self, stdscr):
        self.stdscr = stdscr
        self.gs = GameState()
        self.messages = []
        self.mode = "map"  # map, tile_detail
        self.selected_tile = None

        # Setup curses
        curses.curs_set(0)  # Hide cursor
        self.stdscr.nodelay(0)  # Blocking input
        self.setup_colors()

    def setup_colors(self):
        """Setup color pairs for different terrains"""
        curses.init_pair(1, curses.COLOR_GREEN, curses.COLOR_BLACK)  # Plains
        curses.init_pair(2, curses.COLOR_WHITE, curses.COLOR_BLACK)  # Mountain
        curses.init_pair(3, curses.COLOR_YELLOW, curses.COLOR_BLACK)  # Forest
        curses.init_pair(4, curses.COLOR_CYAN, curses.COLOR_BLACK)   # Swamp
        curses.init_pair(5, curses.COLOR_RED, curses.COLOR_BLACK)    # Desert
        curses.init_pair(6, curses.COLOR_WHITE, curses.COLOR_BLUE)   # Cursor
        curses.init_pair(7, curses.COLOR_YELLOW, curses.COLOR_BLACK) # Gold/resources

    def safe_addstr(self, y, x, text, attr=curses.A_NORMAL):
        """Safely add string to screen"""
        max_y, max_x = self.stdscr.getmaxyx()
        if 0 <= y < max_y and 0 <= x < max_x:
            try:
                # Truncate text if needed
                available = max_x - x
                if available > 0:
                    self.stdscr.addstr(y, x, text[:available], attr)
            except curses.error:
                pass

    def draw_map(self):
        """Draw the tile map"""
        # Map viewport
        map_height = 20
        map_width = 40

        # Draw map border
        self.safe_addstr(0, 0, "+" + "-" * map_width + "+")
        for y in range(map_height):
            self.safe_addstr(y + 1, 0, "|")
            self.safe_addstr(y + 1, map_width + 1, "|")
        self.safe_addstr(map_height + 1, 0, "+" + "-" * map_width + "+")

        # Draw tiles
        for y in range(min(self.gs.height, map_height)):
            for x in range(min(self.gs.width, map_width)):
                tile = self.gs.tiles.get((x, y))
                if not tile:
                    continue

                # Determine display character and color
                if tile.terrain == Terrain.PLAINS:
                    char = '.'
                    color = curses.color_pair(1)
                elif tile.terrain == Terrain.MOUNTAIN:
                    char = '^'
                    color = curses.color_pair(2)
                elif tile.terrain == Terrain.FOREST:
                    char = 'T'
                    color = curses.color_pair(3)
                elif tile.terrain == Terrain.SWAMP:
                    char = '~'
                    color = curses.color_pair(4)
                elif tile.terrain == Terrain.DESERT:
                    char = 's'
                    color = curses.color_pair(5)
                else:
                    char = '?'
                    color = curses.A_NORMAL

                # Mark chokepoints
                if tile.is_chokepoint:
                    char = 'X'

                # Mark if tile has mining
                if "mining" in tile.upgrades:
                    char = 'M'

                # Draw cursor position
                if x == self.gs.cursor_x and y == self.gs.cursor_y:
                    color = curses.color_pair(6)

                # Draw the character
                self.safe_addstr(y + 1, x * 2 + 1, char + ' ', color)

    def draw_resources(self):
        """Draw player resources"""
        y_offset = 23
        self.safe_addstr(y_offset, 0, "Resources:", curses.A_BOLD)

        resources_display = []
        for resource, amount in self.gs.resources.items():
            resources_display.append(f"{resource.capitalize()}: {amount}")

        # Display in two rows
        row1 = " | ".join(resources_display[:3])
        row2 = " | ".join(resources_display[3:])

        self.safe_addstr(y_offset + 1, 2, row1, curses.color_pair(7))
        if row2:
            self.safe_addstr(y_offset + 2, 2, row2, curses.color_pair(7))

    def draw_tile_info(self):
        """Draw information about selected tile"""
        x_offset = 45
        y_offset = 1

        tile = self.gs.tiles.get((self.gs.cursor_x, self.gs.cursor_y))
        if not tile:
            return

        self.safe_addstr(y_offset, x_offset, f"Tile ({tile.x}, {tile.y})", curses.A_BOLD)
        self.safe_addstr(y_offset + 1, x_offset, f"Terrain: {tile.terrain.value}")
        self.safe_addstr(y_offset + 2, x_offset, f"Defense: +{tile.defense_bonus}%")
        self.safe_addstr(y_offset + 3, x_offset, f"Move Cost: {tile.movement_cost}x")

        if tile.is_chokepoint:
            self.safe_addstr(y_offset + 4, x_offset, "CHOKEPOINT", curses.A_BOLD | curses.color_pair(5))

        # Show resources if prospected
        y_offset += 6
        if tile.prospected:
            self.safe_addstr(y_offset, x_offset, "Resources:", curses.A_BOLD)
            for i, (resource, amount) in enumerate(tile.resources.items()):
                if amount > 0:
                    self.safe_addstr(y_offset + 1 + i, x_offset + 2, f"{resource}: {amount}")
        else:
            self.safe_addstr(y_offset, x_offset, "Not Prospected", curses.A_DIM)

        # Show upgrades
        y_offset += 5
        if tile.upgrades:
            self.safe_addstr(y_offset, x_offset, "Upgrades:", curses.A_BOLD)
            for i, (upgrade, level) in enumerate(tile.upgrades.items()):
                self.safe_addstr(y_offset + 1 + i, x_offset + 2, f"{upgrade}: Level {level}")

        # Show ownership
        y_offset += 4
        owner = tile.get_owner()
        if owner:
            self.safe_addstr(y_offset, x_offset, f"Owner: {owner}", curses.A_BOLD)
        elif tile.is_contested():
            self.safe_addstr(y_offset, x_offset, "CONTESTED", curses.A_BOLD | curses.color_pair(5))

    def draw_commands(self):
        """Draw available commands"""
        y_offset = 27
        self.safe_addstr(y_offset, 0, "Commands:", curses.A_BOLD)
        self.safe_addstr(y_offset + 1, 2, "[Arrow Keys] Move cursor")
        self.safe_addstr(y_offset + 2, 2, "[P] Prospect tile | [M] Start mining | [U] Upgrade")
        self.safe_addstr(y_offset + 3, 2, "[C] Claim section | [Space] Next turn | [Q] Quit")

    def draw_messages(self):
        """Draw recent messages"""
        y_offset = 32
        for i, msg in enumerate(self.messages[-3:]):
            self.safe_addstr(y_offset + i, 0, msg, curses.A_DIM)

    def add_message(self, msg):
        """Add a message to the log"""
        self.messages.append(msg)
        if len(self.messages) > 10:
            self.messages.pop(0)

    def handle_prospect(self):
        """Handle prospecting command"""
        tile = self.gs.tiles.get((self.gs.cursor_x, self.gs.cursor_y))
        if not tile:
            return

        if tile.prospected:
            self.add_message("Already prospected this tile")
            return

        # Cost to prospect
        if self.gs.resources["gold"] < 10:
            self.add_message("Need 10 gold to prospect")
            return

        self.gs.resources["gold"] -= 10
        resources = tile.prospect()

        # Report findings
        resource_str = ", ".join([f"{r}: {a}" for r, a in resources.items() if a > 0])
        self.add_message(f"Prospected: {resource_str}")

    def handle_mining(self):
        """Handle mining setup"""
        tile = self.gs.tiles.get((self.gs.cursor_x, self.gs.cursor_y))
        if not tile:
            return

        if not tile.prospected:
            self.add_message("Must prospect first")
            return

        # Find best resource to mine
        best_resource = None
        best_amount = 0
        for resource, amount in tile.resources.items():
            if amount > best_amount:
                best_resource = resource
                best_amount = amount

        if not best_resource:
            self.add_message("No resources to mine")
            return

        if self.gs.start_mining_operation(tile, best_resource):
            self.add_message(f"Started mining {best_resource}")
        else:
            self.add_message("Cannot afford mining operation")

    def handle_upgrade(self):
        """Handle upgrade building"""
        tile = self.gs.tiles.get((self.gs.cursor_x, self.gs.cursor_y))
        if not tile:
            return

        # For now, just upgrade fortification
        if self.gs.build_upgrade(tile, "fortification"):
            level = tile.upgrades["fortification"]
            self.add_message(f"Built fortification level {level}")
        else:
            self.add_message("Cannot afford fortification")

    def handle_claim(self):
        """Handle claiming tile section"""
        tile = self.gs.tiles.get((self.gs.cursor_x, self.gs.cursor_y))
        if not tile:
            return

        # Claim first unclaimed section
        for direction in Direction:
            if tile.sections[direction].owner is None:
                self.gs.claim_tile_section(tile, direction)
                self.add_message(f"Claimed {direction.value} section")
                return

        self.add_message("All sections already claimed")

    def run(self):
        """Main game loop"""
        running = True
        while running:
            self.stdscr.clear()

            # Draw everything
            self.draw_map()
            self.draw_resources()
            self.draw_tile_info()
            self.draw_commands()
            self.draw_messages()

            self.stdscr.refresh()

            # Handle input
            key = self.stdscr.getch()

            if key == ord('q') or key == ord('Q'):
                running = False
            elif key == curses.KEY_UP:
                self.gs.cursor_y = max(0, self.gs.cursor_y - 1)
            elif key == curses.KEY_DOWN:
                self.gs.cursor_y = min(self.gs.height - 1, self.gs.cursor_y + 1)
            elif key == curses.KEY_LEFT:
                self.gs.cursor_x = max(0, self.gs.cursor_x - 1)
            elif key == curses.KEY_RIGHT:
                self.gs.cursor_x = min(self.gs.width - 1, self.gs.cursor_x + 1)
            elif key == ord('p') or key == ord('P'):
                self.handle_prospect()
            elif key == ord('m') or key == ord('M'):
                self.handle_mining()
            elif key == ord('u') or key == ord('U'):
                self.handle_upgrade()
            elif key == ord('c') or key == ord('C'):
                self.handle_claim()
            elif key == ord(' '):  # Space for next turn
                self.gs.process_mining_turn()
                self.add_message("Turn processed")


def main(stdscr):
    game = Game(stdscr)
    game.run()


if __name__ == "__main__":
    curses.wrapper(main)