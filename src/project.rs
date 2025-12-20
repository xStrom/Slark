/*
    Copyright 2019-2022 Kaur Kuut <admin@kaurkuut.com>

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

use masonry::kurbo::Point;
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};

use crate::ui::Zoom;

pub const FILE_DESC: &str = "Slark project";
pub const FILE_EXT: &str = "ark";

pub fn image_path_abs(project_dir: Option<&Path>, image_path: &Path) -> PathBuf {
    if image_path.is_absolute() {
        return image_path.to_path_buf();
    }
    if let Some(project_dir) = project_dir {
        return project_dir.join(image_path).canonicalize().unwrap();
    }
    image_path.canonicalize().unwrap()
}

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
        project.set_path(&path);
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

    pub fn dir(&self) -> Option<&Path> {
        match &self.state.dir {
            Some(dir) => Some(dir.as_path()),
            None => None,
        }
    }

    /// Sets the project save path and the derived work directory.
    fn set_path(&mut self, path: &Path) {
        self.state.dir = Some(path.parent().unwrap().canonicalize().unwrap());
        self.state.path = Some(self.state.dir.as_deref().unwrap().join(path.file_name().unwrap()));
    }

    /// Change the project save path.
    ///
    /// This calls [`set_path`] and then also updates all the relative image paths.
    fn change_path(&mut self, path: &Path) {
        if self.state.path.as_deref().is_some_and(|p| p == path) {
            return;
        }
        let old_project_dir = self.state.dir.clone();
        self.set_path(path);
        let project_dir = self.state.dir.as_deref().unwrap();
        for img in &mut self.images {
            let image_path = image_path_abs(old_project_dir.as_deref(), &img.path);
            img.path = diff_paths(&image_path, project_dir).unwrap();
        }
    }

    pub fn save(&mut self, path: &Path) {
        self.change_path(path);
        if let Ok(json) = serde_json::to_string(self) {
            let mut file = File::create(path).expect("Failed to create file");
            file.write_all(json.as_bytes()).expect("Failed to write file");
            file.sync_all().expect("Failed to sync file");
            self.state.dirty = false;
        }
    }

    pub fn add(&mut self, path: PathBuf) {
        let mut path = path.canonicalize().unwrap();
        if let Some(project_dir) = self.state.dir.as_deref() {
            path = diff_paths(&path, project_dir).unwrap();
        }
        let next_id = self.images.len();
        self.images.push(Image {
            id: next_id,
            path: path,
            origin: Point::ZERO,
            zoom: Zoom::default(),
        });
        self.layers.push(next_id);
        self.state.dirty = true;
    }

    pub fn remove(&mut self, image_id: usize) {
        if self.images.is_empty() || self.images.len() <= image_id {
            return;
        } else if self.images.len() == 1 {
            self.images.clear();
            self.layers.clear();
        } else {
            let last_id = self.images.len() - 1;
            self.images[last_id].id = image_id;
            self.images.swap(image_id, last_id);
            self.images.pop();
            self.layers
                .remove(self.layers.iter().position(|id| *id == image_id).unwrap());
            if let Some(pos) = self.layers.iter().position(|id| *id == last_id) {
                self.layers[pos] = image_id;
            }
        }
    }

    pub fn set_origin(&mut self, image_id: usize, origin: Point) {
        if let Some(image) = self.images.iter_mut().find(|image| image.id == image_id) {
            if image.origin != origin {
                image.origin = origin;
                self.state.dirty = true;
            }
        }
    }

    pub fn set_zoom(&mut self, image_id: usize, zoom: Zoom) {
        if let Some(image) = self.images.iter_mut().find(|image| image.id == image_id) {
            if image.zoom != zoom {
                image.zoom = zoom;
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
    #[serde(default)]
    zoom: Zoom,
}

impl Image {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn path_abs(&self, project_dir: Option<&Path>) -> PathBuf {
        image_path_abs(project_dir, &self.path)
    }

    pub fn origin(&self) -> &Point {
        &self.origin
    }

    pub fn zoom(&self) -> &Zoom {
        &self.zoom
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
    dir: Option<PathBuf>,
    dirty: bool,
}
