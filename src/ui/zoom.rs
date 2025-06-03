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

use druid::Data;
use serde::{Deserialize, Serialize};

#[derive(Default, Data, Copy, Clone, Serialize, Deserialize)]
pub struct Zoom {
    knob: i32, // 0 means no zoom, negative zooms out
    #[serde(skip)]
    extra: i32, // Global zoom
}

impl PartialEq for Zoom {
    fn eq(&self, other: &Self) -> bool {
        self.knob == other.knob
    }
}

impl Zoom {
    pub fn scale_factor(&self) -> f64 {
        let scale = if self.knob < 0 {
            let mut scale = 1.1f64.powi(self.knob);
            if scale < 0.1 {
                scale = 0.1
            }
            scale
        } else if self.knob > 0 {
            1.1f64.powi(self.knob)
        } else {
            1.0
        };
        self.extra_scale(scale)
    }

    fn extra_scale(&self, scale: f64) -> f64 {
        if self.extra != 0 {
            scale * 1.01f64.powi(self.extra)
        } else {
            scale
        }
    }

    pub fn turn_the_knob(&mut self, delta: i32) {
        let old_knob = self.knob;
        let old_scale = self.scale_factor();
        self.knob = self.knob + delta;
        // If the scale factor didn't change, revert the zoom change
        if self.scale_factor() == old_scale {
            self.knob = old_knob;
        }
    }

    pub fn turn_the_global_knob(&mut self, delta: i32) {
        let old_knob = self.extra;
        let old_scale = self.scale_factor();
        self.extra = self.extra + delta;
        // If the scale factor didn't change, revert the zoom change
        if self.scale_factor() == old_scale {
            self.extra = old_knob;
        }
    }
}
