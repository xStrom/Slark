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

use druid::{Widget};
use druid::widget::{Flex};
use druid::piet::{Color};

use super::{Stats, Surface, Border};

pub fn ui_root() -> impl Widget<u32> {
    let mut col = Flex::column();

    col.add_child(Stats::new(), 0.0);

    let mut surface = Surface::new();
    surface.set_border(Some(Border::new(50.0, Color::rgb8(47, 98, 237).into())));
    surface.add("fw.gif");

    col.add_child(surface, 1.0);

    col
}