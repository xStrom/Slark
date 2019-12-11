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

use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use druid::kurbo::Point;
use druid::{FileDialogOptions, FileSpec};
use serde::{Deserialize, Serialize};

const PROJECT_FILE_TYPE: FileSpec = FileSpec::new("Slark project", &["ark"]);

#[derive(Serialize, Deserialize)]
pub struct Project {
    images: Vec<Image>,
    layers: Vec<usize>,
    #[serde(skip)]
    state: State,
}

impl Project {
    pub fn new() -> Project {
        Project {
            images: Vec::new(),
            layers: Vec::new(),
            state: State::default(),
        }
    }

    pub fn open(path: PathBuf) -> Project {
        let file = File::open(&path).expect("Failed to open file");
        let reader = BufReader::new(file);
        let mut project: Project = serde_json::from_reader(reader).expect("Failed to read file");
        project.state.path = Some(path);
        project
    }

    pub fn images(&self) -> &Vec<Image> {
        &self.images
    }

    pub fn layers(&self) -> &Vec<usize> {
        &self.layers
    }

    pub fn dirty(&self) -> bool {
        self.state.dirty
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.state.path {
            Some(path) => Some(path.as_path()),
            None => None,
        }
    }

    pub fn file_dialog_options(&self) -> FileDialogOptions {
        FileDialogOptions::new()
            .allowed_types(vec![PROJECT_FILE_TYPE])
            .default_type(PROJECT_FILE_TYPE)
    }

    pub fn save(&mut self, path: &Path) {
        if let Ok(json) = serde_json::to_string(self) {
            let mut file = File::create(path).expect("Failed to create file");
            file.write_all(json.as_bytes()).expect("Failed to write file");
            file.sync_all().expect("Failed to sync file");
            self.state.dirty = false;
            let path_changed = if let Some(current_path) = &self.state.path {
                path != current_path
            } else {
                true
            };
            if path_changed {
                self.state.path = Some(PathBuf::from(path));
            }
        }
    }

    pub fn add(&mut self, path: PathBuf) {
        let next_id = self.images.len();
        self.images.push(Image {
            id: next_id,
            path: path,
            origin: Point::ZERO,
        });
        self.layers.push(next_id);
        self.state.dirty = true;
    }

    pub fn set_origin(&mut self, image_id: usize, origin: Point) {
        if let Some(image) = self.images.iter_mut().find(|image| image.id == image_id) {
            if image.origin != origin {
                image.origin = origin;
                self.state.dirty = true;
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
                self.state.dirty = true;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Image {
    id: usize,
    path: PathBuf,
    #[serde(with = "PointDef")]
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

#[derive(Serialize, Deserialize)]
#[serde(remote = "Point")]
struct PointDef {
    pub x: f64,
    pub y: f64,
}

#[derive(Default)]
struct State {
    path: Option<PathBuf>,
    dirty: bool,
}
