// Copyright 2019 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

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
