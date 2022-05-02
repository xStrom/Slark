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
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use druid::piet::{Color, ImageFormat, InterpolationMode, RenderContext};
use druid::widget::prelude::*;
use druid::Data;

use gif::{Decoder, SetParameter};
use gif_dispose::*;
use rgb::*;

#[derive(Data, Clone)]
pub struct ViewData {
    pub selected: bool,
    pub zoom: i32, // Use the zoom method to change
}

impl ViewData {
    pub fn zoom(&mut self, delta: i32) {
        let old_zoom = self.zoom;
        let old_scale = self.scale_factor();
        self.zoom = self.zoom + delta;
        // If the scale factor didn't change, revert the zoom change
        if self.scale_factor() == old_scale {
            self.zoom = old_zoom;
        }
    }

    pub fn scale_factor(&self) -> f64 {
        if self.zoom < 0 {
            let mut scale = 1.1f64.powi(self.zoom);
            if scale < 0.1 {
                scale = 0.1
            }
            scale
        } else if self.zoom > 0 {
            1.1f64.powi(self.zoom)
        } else {
            1.0
        }
    }
}

pub struct View {
    source: Option<Receiver<FrameSource>>,
    image_size: Option<Size>,
    frames: Vec<Frame>,
    current_frame: usize,
    current_delay: i64,

    need_legit_layout: bool, // true when we've had to give a fake size in layout
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

impl View {
    pub fn new(path: &Path) -> View {
        let gif_ext = OsStr::new("gif");
        let webp_ext = OsStr::new("webp");
        let jpg_ext = OsStr::new("jpg");
        let jpeg_ext = OsStr::new("jpeg");
        let png_ext = OsStr::new("png");

        match path.extension() {
            Some(ext) => {
                if ext == gif_ext {
                    let file = File::open(path).expect("Failed to open file");
                    let mut decoder = Decoder::new(file);
                    decoder.set(gif::ColorOutput::Indexed);

                    let mut reader = decoder.read_info().expect("Failed to read info");
                    let width = reader.width() as usize;
                    let height = reader.height() as usize;
                    let global_palette = reader.global_palette().map(View::convert_pixels);

                    let mut screen = Screen::new(width, height, RGBA8::default(), global_palette);

                    let (sender, receiver) = channel();

                    let debug_filename = String::from(path.to_str().expect("GIF path is invalid UTF-8"));

                    thread::spawn(move || {
                        let start = Instant::now();
                        // NOTE: The decoding/bliting is surprisingly slow, especially in debug builds
                        while let Some(frame) = reader.read_next_frame().expect("Failed to read next frame") {
                            screen.blit_frame(frame).expect("Failed to blit frame");
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

                    View {
                        source: Some(receiver),
                        image_size: Some(Size::new(width as f64, height as f64)),
                        frames: Vec::new(),
                        current_frame: 0,
                        current_delay: 0,
                        need_legit_layout: false,
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

                    View {
                        source: Some(receiver),
                        image_size: None, // TODO: Probably should get the total image dimensions here, not depend on the first frame which might be smaller.
                        frames: Vec::new(),
                        current_frame: 0,
                        current_delay: 0,
                        need_legit_layout: false,
                    }
                } else if ext == jpg_ext || ext == jpeg_ext {
                    let file = File::open(path).expect("Failed to open file");

                    let (sender, receiver) = channel();

                    let debug_filename = String::from(path.to_str().expect("JPEG path is invalid UTF-8"));

                    thread::spawn(move || {
                        let start = Instant::now();

                        let mut decoder = jpeg_decoder::Decoder::new(BufReader::new(file));
                        let pixels = decoder.decode().expect("Failed to decode JPEG image");
                        let metadata = decoder.info().unwrap();

                        let mut data = Vec::<u8>::with_capacity(metadata.width as usize * metadata.height as usize * 4);
                        let mut i = 0;
                        for b in pixels.iter() {
                            data.push(*b);
                            i += 1;
                            if i == 3 {
                                i = 0;
                                data.push(255);
                            }
                        }

                        sender
                            .send(FrameSource {
                                data: data,
                                width: metadata.width as usize,
                                height: metadata.height as usize,
                                delay: 0,
                            })
                            .expect("Failed to send frame source");

                        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
                    });

                    View {
                        source: Some(receiver),
                        image_size: None,
                        frames: Vec::new(),
                        current_frame: 0,
                        current_delay: 0,
                        need_legit_layout: false,
                    }
                } else if ext == png_ext {
                    let file = File::open(path).expect("Failed to open file");

                    let (sender, receiver) = channel();

                    let debug_filename = String::from(path.to_str().expect("JPEG path is invalid UTF-8"));

                    thread::spawn(move || {
                        let start = Instant::now();

                        // TODO: Make sure that transparency works properly.
                        // TODO: Figure out the issues with the walking APNG.

                        let decoder = png::Decoder::new(file);
                        let mut reader = decoder.read_info().unwrap();
                        // Allocate the output buffer.
                        let mut buf = vec![0; reader.output_buffer_size()];
                        // Read the next frame. An APNG might contain multiple frames.
                        loop {
                            match reader.next_frame(&mut buf) {
                                Ok(info) => {
                                    let (mut width, mut height) = (info.width as usize, info.height as usize);

                                    // Grab the bytes of the image.
                                    let bytes = &buf[..info.buffer_size()];

                                    let mut delay = 0;
                                    // Inspect more details of the last read frame.
                                    if let Some(more_info) = reader.info().frame_control {
                                        let mut den = more_info.delay_den as u64;
                                        if den == 0 {
                                            den = 100;
                                        }
                                        delay = (1_000_000_000 * (more_info.delay_num as u64) / den) as i64;

                                        (width, height) = (more_info.width as usize, more_info.height as usize);
                                    }

                                    println!(
                                        "Found another PNG frame for {} which has {} bytes and {} x {}",
                                        debug_filename,
                                        bytes.len(),
                                        info.width,
                                        info.height
                                    );

                                    let mut data =
                                        Vec::<u8>::with_capacity(info.width as usize * info.height as usize * 4);
                                    let mut i = 0;
                                    for b in bytes.iter() {
                                        data.push(*b);
                                        i += 1;
                                        if i == 3 {
                                            i = 0;
                                            data.push(40);
                                        }
                                    }

                                    sender
                                        .send(FrameSource {
                                            data: data,
                                            width: width,
                                            height: height,
                                            delay: delay,
                                        })
                                        .expect("Failed to send frame source");
                                }
                                Err(error) => {
                                    println!("PNG reader error: {}", error);
                                    break;
                                }
                            }
                        }

                        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
                    });

                    View {
                        source: Some(receiver),
                        image_size: None,
                        frames: Vec::new(),
                        current_frame: 0,
                        current_delay: 0,
                        need_legit_layout: false,
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

    #[rustfmt::skip]
    fn convert_pixels<T: From<RGB8>>(palette_bytes: &[u8]) -> Vec<T> {
        palette_bytes.chunks(3).map(|byte| {RGB8{r: byte[0], g: byte[1], b: byte[2]}.into()}).collect()
    }

    // Returns `true` if a new frame was loaded.
    fn load_frame(&mut self, ctx: &mut PaintCtx) -> bool {
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
                // Set the image's dimensions based on the first frame, unless we already have that info
                if self.image_size.is_none() {
                    self.image_size = Some(Size::new(source.width as f64, source.height as f64));
                }
                return true;
            } else {
                self.source = None;
            }
        }
        false
    }

    fn current_frame(&mut self, ctx: &mut PaintCtx) -> Option<&druid::piet::d2d::Bitmap> {
        self.load_frame(ctx);

        if self.frames.is_empty() {
            None
        } else {
            Some(&self.frames[self.current_frame].img)
        }
    }

    fn next_frame(&mut self, ctx: &mut PaintCtx) -> Option<&druid::piet::d2d::Bitmap> {
        self.load_frame(ctx);

        if self.frames.len() == 0 {
            return None;
        }

        // Progress to the next frame
        self.current_frame += 1;
        if self.current_frame >= self.frames.len() {
            self.current_frame = 0;
        }

        // Add the post-frame delay to our counter
        self.current_delay += self.frames[self.current_frame].delay;
        // Return the frame
        Some(&self.frames[self.current_frame].img)
    }
}

impl Widget<ViewData> for View {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut ViewData, _env: &Env) {
        match event {
            Event::AnimFrame(interval) => {
                // TODO: Think about clamping it to zero -- comapre how it works.
                //       There might be underflows with 0-delay GIFs.
                self.current_delay -= *interval as i64;
                ctx.request_anim_frame();
                // When we do fine-grained invalidation,
                // no doubt this will be required:
                //ctx.request_paint();

                if self.need_legit_layout && self.image_size.is_some() {
                    ctx.request_layout();
                    self.need_legit_layout = false;
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &ViewData, _env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.request_anim_frame();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &ViewData, data: &ViewData, _env: &Env) {
        if data.zoom != old_data.zoom {
            ctx.request_layout();
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &ViewData, _env: &Env) -> Size {
        bc.debug_check("Image");
        let size = match self.image_size {
            Some(size) => size * data.scale_factor(),
            None => {
                self.need_legit_layout = true;
                Size::new(100.0, 100.0) * data.scale_factor()
            }
        };
        // TODO: Should we ignore constraints to be able to return a non-integer HiDPI-aware size?
        bc.constrain(size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &ViewData, _env: &Env) {
        // TODO: Implement fancier resizing and cache the frames for recent scale factors.
        //       Think about scaling quality+speed here .. do we want to source from an already-scaled cached image instead?

        let src_rect = self.image_size.unwrap_or_default().to_rect();
        let dst_rect = ctx.size().to_rect();

        if self.current_delay > 0 {
            // Still more waiting to do, just paint the current frame
            if let Some(img) = self.current_frame(ctx) {
                ctx.render_ctx
                    .draw_image_area(img, src_rect, dst_rect, InterpolationMode::Bilinear);
            }
        } else {
            // Paint until there's a delay specified
            let start_frame = self.current_frame;
            while self.current_delay <= 0 {
                // Paint the next frame
                if let Some(img) = self.next_frame(ctx) {
                    ctx.render_ctx
                        .draw_image_area(img, src_rect, dst_rect, InterpolationMode::Bilinear);
                }
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
            let stroke_width = 1.0;

            // TODO: Double check the pixel perfect nature of this after HiDPI awareness is implemented
            let stroke_rect = dst_rect.inset(-stroke_width / 2.0);
            ctx.render_ctx.stroke(stroke_rect, &brush, stroke_width);
        }
    }
}
