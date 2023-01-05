/*
    Copyright 2022 Kaur Kuut <admin@kaurkuut.com>

    This file is part of Slark.

    Slark is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use druid::kurbo::{Point, Size, Vec2};

use crate::ui::Zoom;

pub struct Tileize {
    surface: Size,
    tiles: Vec<Tile>,
}

impl Tileize {
    pub fn new(surface: Size) -> Tileize {
        Tileize {
            surface,
            tiles: Vec::new(),
        }
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    pub fn add(&mut self, tile: Tile) {
        self.tiles.push(tile);
    }

    pub fn fit(&mut self) {
        let surface_area = self.surface.area();

        // TODO: Determine if the area is enough to fit all the images with current zoom levels,
        //       reduce some zoom levels if not.

        let tiles_len = self.tiles.len();
        for i in 0..tiles_len {
            if i == 0 {
                self.tiles[i].origin = Point::ZERO;
                continue;
            }
            let effective_tile_size = self.tiles[i - 1].effective_size();
            self.tiles[i].origin = self.tiles[i - 1].origin + Vec2::new(effective_tile_size.width, 0.0);
        }
    }
}

pub struct Tile {
    id: usize,
    origin: Point,
    size: Size,
    zoom: Zoom,
}

impl Tile {
    pub fn new(id: usize, origin: Point, size: Size, zoom: Zoom) -> Tile {
        Tile { id, origin, size, zoom }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn origin(&self) -> Point {
        self.origin
    }

    pub fn effective_size(&self) -> Size {
        self.size * self.zoom.scale_factor()
    }

    pub fn zoom(&self) -> Zoom {
        self.zoom
    }
}
