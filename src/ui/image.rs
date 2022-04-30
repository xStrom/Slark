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

use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use druid::kurbo::{Line, Point, Rect};
use druid::piet::{Color, ImageFormat, InterpolationMode, RenderContext};
use druid::widget::prelude::*;
use druid::Data;

use gif::{Decoder, SetParameter};
use gif_dispose::*;
use imgref::*;
use rgb::*;

#[derive(Data, Clone)]
pub struct ImageData {
    pub origin: Point,
    pub selected: bool,
}

pub struct Image {
    source: Option<Receiver<FrameSource>>,
    frames: Vec<Frame>,
    current_frame: usize,
    current_delay: i64,
}

struct FrameSource {
    data: Vec<u8>,
    width: usize,
    height: usize,
    delay: i64,
}

struct Frame {
    delay: i64,
    width: usize,
    height: usize,
    img: druid::piet::d2d::Bitmap, // TODO: Get druid::piet::Image working for cross-platform support
}

impl Image {
    pub fn new(path: &Path) -> Image {
        let gif_ext = OsStr::new("gif");
        let webp_ext = OsStr::new("webp");

        match path.extension() {
            Some(ext) => {
                if ext == gif_ext {
                    let file = File::open(path).expect("Failed to open file");
                    let mut decoder = Decoder::new(file);
                    decoder.set(gif::ColorOutput::Indexed);

                    let mut reader = decoder.read_info().expect("Failed to read info");
                    let width = reader.width() as usize;
                    let height = reader.height() as usize;
                    let global_palette = reader.global_palette().map(Image::convert_pixels);

                    let mut screen = Screen::new(width, height, RGBA8::default(), global_palette);

                    let (sender, receiver) = channel();

                    let debug_filename = String::from(path.to_str().expect("GIF path is invalid UTF-8"));

                    thread::spawn(move || {
                        let start = Instant::now();
                        // NOTE: The decoding/bliting is surprisingly slow, especially in debug builds
                        while let Some(frame) = reader.read_next_frame().expect("Failed to read next frame") {
                            screen.blit_frame(&frame).expect("Failed to blit frame");
                            sender
                                .send(FrameSource {
                                    data: Vec::from(screen.pixels.buf().as_bytes()),
                                    width: screen.pixels.width(),
                                    height: screen.pixels.height(),
                                    delay: frame.delay as i64 * 10_000_000,
                                })
                                .expect("Failed to send frame source");
                        }
                        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
                    });

                    Image {
                        source: Some(receiver),
                        frames: Vec::new(),
                        current_frame: 0,
                        current_delay: 0,
                    }
                } else if ext == webp_ext {
                    let buffer = std::fs::read(path).unwrap();

                    let (sender, receiver) = channel();

                    let debug_filename = String::from(path.to_str().expect("WEBP path is invalid UTF-8"));

                    thread::spawn(move || {
                        let start = Instant::now();
                        let decoder = webp_animation::Decoder::new(&buffer).unwrap();
                        let mut dec_iter = decoder.into_iter();
                        let mut prev_timestamp = 0;
                        while let Some(frame) = dec_iter.next() {
                            let (width, height) = frame.dimensions();
                            println!(
                                "Calculated {} frame delay: {} ms",
                                debug_filename,
                                (frame.timestamp() - prev_timestamp)
                            );
                            sender
                                .send(FrameSource {
                                    data: Vec::from(frame.data()),
                                    width: width as usize,
                                    height: height as usize,
                                    delay: (frame.timestamp() - prev_timestamp) as i64 * 1_000_000,
                                })
                                .expect("Failed to send frame source");
                            prev_timestamp = frame.timestamp();
                        }
                        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
                    });

                    Image {
                        source: Some(receiver),
                        frames: Vec::new(),
                        current_frame: 0,
                        current_delay: 0,
                    }
                } else {
                    panic!("Not a supported extension!");
                }
            }
            _ => {
                panic!("Not a supported extension!");
            }
        }
    }

    pub fn width(&self) -> usize {
        match self.frames.first() {
            Some(frame) => frame.width,
            None => 1,
        }
    }

    pub fn height(&self) -> usize {
        match self.frames.first() {
            Some(frame) => frame.height,
            None => 1,
        }
    }

    #[rustfmt::skip]
    fn convert_pixels<T: From<RGB8>>(palette_bytes: &[u8]) -> Vec<T> {
        palette_bytes.chunks(3).map(|byte| {RGB8{r: byte[0], g: byte[1], b: byte[2]}.into()}).collect()
    }

    fn current_frame(&mut self, ctx: &mut PaintCtx) -> &druid::piet::d2d::Bitmap {
        if self.frames.is_empty() {
            self.next_frame(ctx)
        } else {
            &self.frames[self.current_frame].img
        }
    }

    fn next_frame(&mut self, ctx: &mut PaintCtx) -> &druid::piet::d2d::Bitmap {
        if self.source.is_some() {
            let receiver = self.source.as_ref().unwrap();
            if let Ok(source) = receiver.recv() {
                let img = ctx
                    .render_ctx
                    .make_image(source.width, source.height, &source.data, ImageFormat::RgbaPremul)
                    .expect("Failed to create image");
                self.frames.push(Frame {
                    img: img,
                    width: source.width,
                    height: source.height,
                    delay: source.delay,
                });
            } else {
                self.source = None;
            }
        }
        // Progress to the next frame
        self.current_frame += 1;
        if self.current_frame == self.frames.len() {
            self.current_frame = 0;
        }
        // Add the post-frame delay to our counter
        self.current_delay += self.frames[self.current_frame].delay;
        // Return the frame
        &self.frames[self.current_frame].img
    }
}

impl Widget<ImageData> for Image {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut ImageData, _env: &Env) {
        match event {
            Event::AnimFrame(interval) => {
                // TODO: Think about clamping it to zero -- comapre how it works.
                //       There might be underflows with 0-delay GIFs.
                self.current_delay -= *interval as i64;
                ctx.request_anim_frame();
                // When we do fine-grained invalidation,
                // no doubt this will be required:
                //ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &ImageData, _env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.request_anim_frame();
            }
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &ImageData, _data: &ImageData, _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &ImageData, _env: &Env) -> Size {
        bc.debug_check("Image");
        bc.constrain((
            self.width() as f64 - data.origin.x,
            self.height() as f64 - data.origin.y,
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ImageData, _env: &Env) {
        // Determine the area of the frame to paint
        let size = ctx.size();
        let src_rect = Rect::from_origin_size(data.origin, size);
        let dst_rect = Rect::from_origin_size(Point::ZERO, size);

        if self.current_delay > 0 {
            // Still more waiting to do, just paint the current frame
            let img = self.current_frame(ctx);
            ctx.render_ctx
                .draw_image_area(img, src_rect, dst_rect, InterpolationMode::Bilinear);
        } else {
            // Paint until there's a delay specified
            let start_frame = self.current_frame;
            while self.current_delay <= 0 {
                // Paint the next frame
                let img = self.next_frame(ctx);
                ctx.render_ctx
                    .draw_image_area(img, src_rect, dst_rect, InterpolationMode::Bilinear);
                // Detect infinite loops due to GIFs with only 0-delay frames
                if self.current_frame == start_frame {
                    break;
                }
            }
        }

        // If active, paint a border on top of the edge of the image
        // TODO: What if it's a 1px image?
        if data.selected {
            let brush = ctx.render_ctx.solid_brush(Color::rgb8(245, 132, 66));
            let width = 1.0;

            // Top
            if data.origin.y == 0.0 {
                let line = Line::new((dst_rect.x0, dst_rect.y0 + 0.5), (dst_rect.x1, dst_rect.y0 + 0.5));
                ctx.render_ctx.stroke(line, &brush, width);
            }
            // Right
            if data.origin.x == self.width() as f64 - size.width {
                let line = Line::new((dst_rect.x1 - 0.5, dst_rect.y0), (dst_rect.x1 - 0.5, dst_rect.y1));
                ctx.render_ctx.stroke(line, &brush, width);
            }
            // Bottom
            if data.origin.y == self.height() as f64 - size.height {
                let line = Line::new((dst_rect.x0, dst_rect.y1 - 0.5), (dst_rect.x1, dst_rect.y1 - 0.5));
                ctx.render_ctx.stroke(line, &brush, width);
            }
            // Left
            if data.origin.x == 0.0 {
                let line = Line::new((dst_rect.x0 + 0.5, dst_rect.y0), (dst_rect.x0 + 0.5, dst_rect.y1));
                ctx.render_ctx.stroke(line, &brush, width);
            }
        }
    }
}
