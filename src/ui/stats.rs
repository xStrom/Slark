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

use druid::{ BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget };
use druid::widget::{ DynLabel };
use druid::kurbo::{Size};

pub struct Stats {
    frame_times: [u64; Stats::FRAME_TIME_COUNT],
    frame_time_index: usize,
    initializing: bool,
    label_fps: DynLabel<u64>,
}

impl Stats {
    const FRAME_TIME_COUNT: usize = 288;

    pub fn new() -> Stats {
        Stats{
            frame_times: [0; Stats::FRAME_TIME_COUNT],
            frame_time_index: 0,
            initializing: true,
            label_fps: DynLabel::new(|data, _| {
                format!("FPS: {}", *data)
            }),
        }
    }

    fn add_frame_time(&mut self, frame_time: u64) {
        self.frame_times[self.frame_time_index] = frame_time;
        self.frame_time_index += 1;
        if self.frame_time_index == Stats::FRAME_TIME_COUNT {
            self.initializing = false;
            self.frame_time_index = 0;
        }
    }

    fn average_fps(&self) -> u64 {
        let timed_frame_count = if self.initializing {
            self.frame_time_index
        } else {
            Stats::FRAME_TIME_COUNT
        };
        let total_frame_time: u64 = if self.initializing {
            self.frame_times.iter().take(timed_frame_count).sum()
        } else {
            self.frame_times.iter().sum()
        };
        let avg_frame_time = if timed_frame_count > 0 {
            total_frame_time / timed_frame_count as u64
        } else {
            0
        };
        let avg_fps = if avg_frame_time > 0 {
            1_000_000_000 / avg_frame_time
        } else {
            0
        };
        avg_fps
    }
}

impl Widget<u32> for Stats {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut u32, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.request_anim_frame();
            }
            Event::AnimFrame(interval) => {
                self.add_frame_time(*interval);
                ctx.request_anim_frame();
                // When we do fine-grained invalidation,
                // no doubt this will be required:
                //ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&u32>, _data: &u32, _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &u32, _env: &Env) -> Size {
        bc.constrain((100.0, 50.0))
    }

    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        _base_state: &BaseState,
        _data: &u32,
        _env: &Env,
    ) {
        self.label_fps.paint(paint_ctx, _base_state, &self.average_fps(), _env);
    }
}