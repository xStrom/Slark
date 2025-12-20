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

use std::fs::read_dir;

use masonry::{
    core::{NewWidget, Properties, Widget, WidgetTag},
    properties::{types::CrossAxisAlignment, Gap},
    widgets::Flex,
};

use super::{Stats, Surface};
use crate::project::Project;

pub fn ui_root(filenames: Vec<String>) -> (impl Widget, Properties) {
    let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    col = col.with_fixed(NewWidget::new(Stats::new()));

    let mut project;
    if filenames.len() > 0 && filenames[0].ends_with(".ark") {
        project = Project::open((&filenames[0]).into());
    } else {
        project = Project::new();
        filenames.iter().for_each(|filename| project.add(filename.into()));
    };

    //project.add("images/fw.gif".into());
    //project.add("images/fw-alpha.gif".into());
    //project.add("images/large.gif".into());

    //project.add("images/tree.webp".into());
    //project.add("images/unicorn-space.webp".into());
    //project.add("images/animated.webp".into());

    //project.add("images/unicorn.jpg".into());
    //project.add("images/d3-unicorn.jpeg".into());

    //project.add("images/hamster.png".into());
    //project.add("images/walking.png".into());
    //project.add("images/fire.png".into());
    //project.add("images/explosion.png".into());

    //load_x(&mut project);

    let surface = Surface::new(project);
    col = col.with(NewWidget::new_with_tag(surface, WidgetTag::named("surface")), 1.0);
    (col, Properties::one(Gap::ZERO))
}

fn load_x(project: &mut Project) {
    let dir = read_dir("images/y").expect("Failed read_dir");
    for entry in dir {
        let entry = entry.expect("Entry failed");
        let path = entry.path();
        if !path.is_dir() {
            if let Some(ext) = path.extension() {
                if ext == "gif" {
                    project.add(path);
                }
            }
        }
    }
}
