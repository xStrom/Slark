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

//! Stats will show the fps based on AnimFrame, which won't be accurate unless some widget is actually doing painting.

use druid::widget::prelude::*;
use druid::widget::Label;
use druid::Application;
use druid::Data;

pub struct Stats {
    frame_times: [u64; Stats::FRAME_TIME_COUNT],
    frame_time_index: usize,
    fps: u64,
    initializing: bool,
    label_fps: Label<u64>,
}

impl Stats {
    const FRAME_TIME_COUNT: usize = 360;

    pub fn new() -> Stats {
        Stats {
            frame_times: [0; Stats::FRAME_TIME_COUNT],
            frame_time_index: 0,
            fps: 0,
            initializing: true,
            label_fps: Label::new("FPS: 0"),
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
        if avg_frame_time > 0 {
            1_000_000_000 / avg_frame_time
        } else {
            0
        }
    }
}

impl<T: Data> Widget<T> for Stats {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                Application::global().quit();
            }
            Event::AnimFrame(interval) => {
                //println!("Interval: {}", *interval as f64 / 1_000_000.);
                self.add_frame_time(*interval);
                let fps = self.average_fps();
                if self.fps != fps {
                    self.fps = fps;
                    self.label_fps.set_text(format!("FPS: {}", self.fps));
                    ctx.request_update();
                    ctx.request_layout();
                }
                ctx.request_anim_frame();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &T, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.request_anim_frame();
                self.label_fps.lifecycle(ctx, event, &self.fps, env);
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, _data: &T, env: &Env) {
        self.label_fps.update(ctx, &self.fps, &self.fps, env); // We don't care about the data update
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        bc.debug_check("Stats");
        let label_bc = bc.loosen();
        let label_size = self.label_fps.layout(ctx, &label_bc, &self.fps, env);
        bc.constrain((70.0, 20.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        self.label_fps.paint(ctx, &self.fps, env);
    }
}
