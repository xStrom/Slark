/*
    Copyright 2019 Kaur Kuut <admin@kaurkuut.com>

    This file is part of slark.

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

use druid::widget::{Align, Button, Flex, Label, DynLabel, Padding, WidgetExt};

use druid::kurbo::{Line, Point, Rect, Size, Vec2};
use druid::piet::{Color, RenderContext, ImageFormat, InterpolationMode};
use druid::{
    AppLauncher, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx,
    Widget, WindowDesc, LocalizedString, WidgetPod,
};

use gif::{Frame, Decoder, Encoder, Reader, Repeat, SetParameter};

use gif_dispose::*;
use imgref::*;
use rgb::*;

struct Stats {
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
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        _base_state: &BaseState,
        _data: &u32,
        _env: &Env,
    ) {
        self.label_fps.paint(paint_ctx, _base_state, &self.average_fps(), _env);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &u32,
        _env: &Env,
    ) -> Size {
        bc.constrain((100.0, 50.0))
    }

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
}

struct Gif {
    width: usize,
    height: usize,
    frames: Vec<ImgVec<RGBA8>>,
    current_frame: usize,
}

impl Gif {
    pub fn new(filename: &str) -> Gif {
        let file = File::open(filename).expect("Failed to open file");
        let mut decoder = Decoder::new(file);
    
        // Important:
        decoder.set(gif::ColorOutput::Indexed);
    
        let mut reader = decoder.read_info().expect("Failed to read info");

        let width = reader.width() as usize;
        let height = reader.height() as usize;
        let global_palette = reader.global_palette().map(Gif::convert_pixels);

        let mut screen = Screen::new(width, height, RGBA8::default(), global_palette);

        let mut frames: Vec<ImgVec<RGBA8>> = Vec::new();

        while let Some(frame) = reader.read_next_frame().expect("Failed to read next frame") {
            screen.blit_frame(&frame).expect("Failed to blit frame");
            frames.push(screen.pixels.clone());
        }

        Gif{ width: width, height: height, frames: frames, current_frame: 0 }
    }

    fn convert_pixels<T: From<RGB8>>(palette_bytes: &[u8]) -> Vec<T> {
        palette_bytes.chunks(3).map(|byte| RGB8{r: byte[0], g: byte[1], b: byte[2]}.into()).collect()
    }
}

impl Widget<u32> for Gif {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        _base_state: &BaseState,
        _data: &u32,
        _env: &Env,
    ) {
        let img = paint_ctx.make_image(self.width, self.height, self.frames[self.current_frame].buf().as_bytes(), ImageFormat::RgbaPremul).expect("Failed to create image");
        paint_ctx.draw_image(&img, Rect::new(0.0, 0.0, self.width as f64, self.height as f64), InterpolationMode::NearestNeighbor);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &u32,
        _env: &Env,
    ) -> Size {
        bc.constrain((self.width as f64, self.height as f64))
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut u32, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                /*
                self.current_frame += 1;
                if self.current_frame == self.frames.len() {
                    self.current_frame = 0;
                }
                */
                ctx.request_anim_frame();
            }
            Event::AnimFrame(interval) => {
                self.current_frame += 1;
                if self.current_frame == self.frames.len() {
                    self.current_frame = 0;
                }
                ctx.request_anim_frame();
                // When we do fine-grained invalidation,
                // no doubt this will be required:
                //ctx.invalidate();
            }
            Event::Command(cmd) => {
                println!("Command: {}", cmd.selector);
            }
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&u32>, _data: &u32, _env: &Env) {}
}

fn main_ui() -> impl Widget<u32> {
    let mut col = Flex::column();

    col.add_child(Stats::new(), 0.0);
    col.add_child(Gif::new("fw.gif"), 1.0);

    col
}

fn main() {
    let window = WindowDesc::new(main_ui)
        .title(LocalizedString::new("app_title").with_placeholder("Slark".to_string()))
        .window_size((800.0, 600.0));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0)
        .expect("launch failed");
}