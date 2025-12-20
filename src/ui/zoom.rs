// Copyright 2022 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
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
