// Copyright 2019 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

// TODO: Rename View to something else in this Xilem world?

use std::ffi::OsStr;
use std::path::Path;
use std::sync::mpsc::Receiver;

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AllowRawMut, BoxConstraints, ChildrenIds, LayoutCtx, PaintCtx, PropertiesMut, PropertiesRef,
    RegisterCtx, UpdateCtx, Widget,
};
use masonry::kurbo::{Affine, Size, Stroke};
use masonry::peniko::{Color, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
use masonry::vello::Scene;
use rgb::ComponentBytes;

use crate::formats::{gif, jpeg, png, webp};
use crate::image::Frame;
use crate::ui::Zoom;

pub struct View {
    pending_frames: Option<Receiver<Frame>>,
    image_size: Option<Size>,
    frames: Vec<CachedFrame>,
    current_frame: usize,
    current_delay: i64,

    need_legit_layout: bool, // true when we've had to give a fake size in layout

    // TODO: Make methods for modifying these? Especially as zoom update needs layout request
    pub selected: bool,
    pub zoom: Zoom,
}

impl AllowRawMut for View {}

struct CachedFrame {
    image: ImageBrush, // TODO: Is this good enough for cache? What about GPU-memory-ref instead?
    delay: i64,
}

impl View {
    pub fn new(path: &Path, zoom: Zoom) -> View {
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
                    let (receiver, image_size) = webp::open_async(path);
                    (Some(receiver), Some(image_size))
                } else if ext == jpg_ext || ext == jpeg_ext {
                    let (receiver, image_size) = jpeg::open_async(path);
                    (Some(receiver), Some(image_size))
                } else if ext == png_ext {
                    let (receiver, image_size) = png::open_async(path);
                    (Some(receiver), Some(image_size))
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
            selected: false,
            zoom: zoom,
        }
    }

    // Returns `true` if a new frame was loaded.
    fn load_frame(&mut self) -> bool {
        if self.pending_frames.is_some() {
            let receiver = self.pending_frames.as_ref().unwrap();
            if let Ok(frame) = receiver.recv() {
                let (buf, width, height) = frame.image.into_contiguous_buf();
                // TODO: Make this a lot less insane
                let buf_bytes = buf.as_bytes();
                let mut buf_vec = Vec::new();
                buf_vec.extend_from_slice(buf_bytes);
                let image_data = ImageData {
                    data: buf_vec.into(),
                    format: ImageFormat::Rgba8,
                    alpha_type: ImageAlphaType::Alpha,
                    width: width as u32,
                    height: height as u32,
                };
                let image_brush = ImageBrush::new(image_data);
                self.frames.push(CachedFrame {
                    image: image_brush,
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

    fn current_frame(&mut self) -> Option<&ImageBrush> {
        self.load_frame();

        if self.frames.is_empty() {
            None
        } else {
            Some(&self.frames[self.current_frame].image)
        }
    }

    fn next_frame(&mut self) -> Option<&ImageBrush> {
        self.load_frame();

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

impl Widget for View {
    type Action = ();

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        // TODO: Think about clamping it to zero -- comapre how it works.
        //       There might be underflows with 0-delay GIFs.
        self.current_delay -= interval as i64;
        ctx.request_anim_frame();
        ctx.request_paint_only();

        if self.need_legit_layout && self.image_size.is_some() {
            ctx.request_layout();
            self.need_legit_layout = false;
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {
        // TODO: Nothing?
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {
        // TODO: Something?
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new_const()
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> Size {
        bc.debug_check("Image");
        let size = match self.image_size {
            Some(size) => size * self.zoom.scale_factor(),
            None => {
                self.need_legit_layout = true;
                Size::new(100.0, 100.0) * self.zoom.scale_factor()
            }
        };
        // TODO: Should we ignore constraints to be able to return a non-integer HiDPI-aware size?
        bc.constrain(size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        // TODO: Implement fancier resizing and cache the frames for recent scale factors.
        //       Think about scaling quality+speed here .. do we want to source from an already-scaled cached image instead?

        let dst_rect = ctx.size().to_rect();
        let affine = Affine::scale(self.zoom.scale_factor());

        if self.current_delay > 0 {
            // Still more waiting to do, just paint the current frame
            if let Some(img) = self.current_frame() {
                scene.draw_image(img, affine);
            }
        } else {
            // Paint until there's a delay specified
            let start_frame = self.current_frame;
            while self.current_delay <= 0 {
                // Paint the next frame
                if let Some(img) = self.next_frame() {
                    scene.draw_image(img, affine);
                }
                // Detect infinite loops due to GIFs with only 0-delay frames
                if self.current_frame == start_frame {
                    break;
                }
            }
        }

        // If active, paint a border on top of the edge of the image
        // TODO: What if it's a 1px image?
        if self.selected {
            let brush = Color::from_rgb8(245, 132, 66);
            let stroke_width = 1.0;
            let style = Stroke::new(stroke_width);

            // TODO: Double check the pixel perfect nature of this after HiDPI awareness is implemented
            let stroke_rect = dst_rect.inset(-stroke_width / 2.0);

            scene.stroke(&style, Affine::IDENTITY, brush, None, &stroke_rect);
        }
    }
}
