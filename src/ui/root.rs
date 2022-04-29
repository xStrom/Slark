/*
    Copyright 2019-2020 Kaur Kuut <admin@kaurkuut.com>

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

use druid::piet::Color;
use druid::widget::Flex;
use druid::widget::{CrossAxisAlignment, MainAxisAlignment};
use druid::Widget;

use druid::widget::{Padding, SizedBox};
use druid::WidgetExt;

use super::{Border, Stats, Surface};
use crate::project::Project;

pub fn ui_rootx() -> impl Widget<u64> {
    let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::End);

    let bc0 = Color::rgb8(255, 255, 255);
    let bc1 = Color::rgb8(255, 0, 0);
    let bc2 = Color::rgb8(0, 255, 0);
    let bc3 = Color::rgb8(0, 0, 255);

    col.add_child(Stats::new());

    col.add_child(SizedBox::empty().width(98.0).height(28.0).border(bc2.clone(), 1.0));

    let mut row = Flex::row()
        .must_fill_main_axis(true)
        .main_axis_alignment(MainAxisAlignment::SpaceBetween);

    row.add_flex_child(SizedBox::empty().width(980.).height(28.0).border(bc1.clone(), 1.0), 1.0);
    //row.add_flex_child(SizedBox::empty().width(980.).height(28.0).border(bc2.clone(), 1.0), 1.0);
    //row.add_flex_child(SizedBox::empty().width(980.).height(28.0).border(bc3.clone(), 1.0), 1.0);

    col.add_flex_child(row, 1.0);

    let mut root_flex = Flex::column();

    let mut col_container = Flex::row();
    col_container.add_flex_child(col, 1.0);

    root_flex.add_child(Padding::new(
        25.0,
        SizedBox::new(col_container)
            .width(100.0)
            .height(150.0)
            .border(bc0.clone(), 1.0),
    ));

    root_flex
}

pub fn ui_root() -> impl Widget<u64> {
    let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    col.add_child(Stats::new());

    let mut project = Project::new();
    project.add("fw.gif".into());
    //project.add("fw-alpha.gif".into());
    //project.add("large.gif".into());

    //load_x(&mut project);

    let mut surface = Surface::new(project);
    surface.set_border(Some(Border::new(0.0, Color::rgb8(47, 98, 237).into())));

    col.add_flex_child(surface, 1.0);

    col
}

fn load_x(project: &mut Project) {
    let dir = read_dir("y").expect("Failed read_dir");
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
