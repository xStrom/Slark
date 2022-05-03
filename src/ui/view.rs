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
use std::path::Path;
use std::sync::mpsc::Receiver;

use druid::piet::{Color, ImageFormat, InterpolationMode, RenderContext};
use druid::widget::prelude::*;
use druid::Data;
use rgb::ComponentBytes;

use crate::formats::{gif, jpeg, png, webp};
use crate::image::Frame;

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
    pending_frames: Option<Receiver<Frame>>,
    image_size: Option<Size>,
    frames: Vec<CachedFrame>,
    current_frame: usize,
    current_delay: i64,

    need_legit_layout: bool, // true when we've had to give a fake size in layout
}

struct CachedFrame {
    image: druid::piet::d2d::Bitmap, // TODO: Get druid::piet::Image working for cross-platform support
    delay: i64,
}

impl View {
    pub fn new(path: &Path) -> View {
        let gif_ext = OsStr::new("gif");
        let webp_ext = OsStr::new("webp");
        let jpg_ext = OsStr::new("jpg");
        let jpeg_ext = OsStr::new("jpeg");
        let png_ext = OsStr::new("png");

        let (receiver, image_size) = match path.extension() {
            Some(ext) => {
                if ext == gif_ext {
                    let (receiver, image_size) = gif::open_async(path);
                    (Some(receiver), Some(image_size))
                } else if ext == webp_ext {
                    let receiver = webp::open_async(path);
                    (Some(receiver), None)
                } else if ext == jpg_ext || ext == jpeg_ext {
                    let receiver = jpeg::open_async(path);
                    (Some(receiver), None)
                } else if ext == png_ext {
                    let receiver = png::open_async(path);
                    (Some(receiver), None)
                } else {
                    println!("WARNING: Unsupported file extension: {}", ext.to_str().unwrap());
                    (None, None)
                }
            }
            _ => {
                println!(
                    "WARNING: Slark needs a proper file extension for format detection. {}",
                    path.to_str().unwrap()
                );
                (None, None)
            }
        };

        View {
            pending_frames: receiver,
            image_size: image_size,
            frames: Vec::new(),
            current_frame: 0,
            current_delay: 0,
            need_legit_layout: false,
        }
    }

    // Returns `true` if a new frame was loaded.
    fn load_frame(&mut self, ctx: &mut PaintCtx) -> bool {
        if self.pending_frames.is_some() {
            let receiver = self.pending_frames.as_ref().unwrap();
            if let Ok(frame) = receiver.recv() {
                let (buf, width, height) = frame.image.into_contiguous_buf();
                let image = ctx
                    .render_ctx
                    .make_image(width, height, buf.as_bytes(), ImageFormat::RgbaSeparate)
                    .expect("Failed to create image");
                self.frames.push(CachedFrame {
                    image: image,
                    delay: frame.delay,
                });
                // Set the image's dimensions based on the first frame, unless we already have that info
                if self.image_size.is_none() {
                    self.image_size = Some(Size::new(width as f64, height as f64));
                } else if self.image_size.unwrap() != Size::new(width as f64, height as f64) {
                    println!("WARNING: Probably a broken image format import code path. View expects all frames to be with full dimensions. {} != {} ", self.image_size.unwrap(), Size::new(width as f64, height as f64));
                }
                return true;
            } else {
                self.pending_frames = None;
            }
        }
        false
    }

    fn current_frame(&mut self, ctx: &mut PaintCtx) -> Option<&druid::piet::d2d::Bitmap> {
        self.load_frame(ctx);

        if self.frames.is_empty() {
            None
        } else {
            Some(&self.frames[self.current_frame].image)
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
        Some(&self.frames[self.current_frame].image)
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
