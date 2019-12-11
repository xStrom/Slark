/*
    Copyright 2019 Kaur Kuut <admin@kaurkuut.com>

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

use std::path::{Path, PathBuf};

use druid::kurbo::Point;

pub struct Project {
    images: Vec<Image>,
    layers: Vec<usize>,
    dirty: bool,
}

impl Project {
    pub fn new() -> Project {
        Project {
            images: Vec::new(),
            layers: Vec::new(),
            dirty: false,
        }
    }

    pub fn images(&self) -> &Vec<Image> {
        &self.images
    }

    pub fn layers(&self) -> &Vec<usize> {
        &self.layers
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn add(&mut self, path: PathBuf) {
        let next_id = self.images.len();
        self.images.push(Image {
            id: next_id,
            path: path,
            origin: Point::ZERO,
        });
        self.layers.push(next_id);
        self.dirty = true;
    }

    pub fn set_origin(&mut self, image_id: usize, origin: Point) {
        if let Some(image) = self.images.iter_mut().find(|image| image.id == image_id) {
            if image.origin != origin {
                image.origin = origin;
                self.dirty = true;
            }
        }
    }

    pub fn shift_layer(&mut self, image_id: usize, delta: isize) {
        if let Some(current_layer) = self.layers.iter().position(|&id| id == image_id) {
            let new_layer = {
                let new_layer = current_layer as isize + delta;
                if new_layer < 0 {
                    0
                } else if new_layer as usize >= self.layers.len() {
                    self.layers.len() - 1
                } else {
                    new_layer as usize
                }
            };
            if new_layer != current_layer {
                self.layers[current_layer] = self.layers[new_layer];
                self.layers[new_layer] = image_id;
                self.dirty = true;
            }
        }
    }
}

pub struct Image {
    id: usize,
    path: PathBuf,
    origin: Point,
}

impl Image {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn origin(&self) -> &Point {
        &self.origin
    }
}
