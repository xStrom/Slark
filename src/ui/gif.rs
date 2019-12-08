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

use std::fs::File;

use druid::{ BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget };
use druid::kurbo::{Rect, Line, Size, Point};
use druid::piet::{Color, RenderContext, Image, ImageFormat, InterpolationMode};

use gif::{Reader, Decoder, SetParameter};
use gif_dispose::*;
use rgb::*;

#[derive(Data, Clone)]
pub struct ImageData {
	pub origin: Point,
}

pub struct Gif {
	width: usize,
	height: usize,
	source: Option<Source>,
	frames: Vec<Frame>,
	current_frame: usize,
	current_delay: i64,
}

struct Source {
	reader: Reader<File>,
	screen: Screen,
}

struct Frame {
	img: druid::piet::Image,
	delay: i64,
}

impl Gif {
	pub fn new(filename: &str) -> Gif {
		let file = File::open(filename).expect("Failed to open file");
		let mut decoder = Decoder::new(file);
		decoder.set(gif::ColorOutput::Indexed);
	
		let reader = decoder.read_info().expect("Failed to read info");
		let width = reader.width() as usize;
		let height = reader.height() as usize;
		let global_palette = reader.global_palette().map(Gif::convert_pixels);

		let screen = Screen::new(width, height, RGBA8::default(), global_palette);

		let source = Source{ reader: reader, screen: screen };

		Gif{
			width: width,
			height: height,
			source: Some(source),
			frames: Vec::new(),
			current_frame: 0,
			current_delay: 0,
		}
	}

	pub fn width(&self) -> usize {
		self.width
	}

	pub fn height(&self) -> usize {
		self.height
	}

	fn convert_pixels<T: From<RGB8>>(palette_bytes: &[u8]) -> Vec<T> {
		palette_bytes.chunks(3).map(|byte| RGB8{r: byte[0], g: byte[1], b: byte[2]}.into()).collect()
	}

	fn current_frame(&mut self, ctx: &mut PaintCtx) -> &Image {
		if self.frames.is_empty() {
			self.next_frame(ctx)
		} else {
			&self.frames[self.current_frame].img
		}
	}

	fn next_frame(&mut self, ctx: &mut PaintCtx) -> &Image {
		// Do GIF decoding on-demand here
		// NOTE: This is surprisingly slow, especially in debug builds
		if self.source.is_some() {
			let source = self.source.as_mut().unwrap();
			if let Some(frame) = source.reader.read_next_frame().expect("Failed to read next frame") {
				source.screen.blit_frame(&frame).expect("Failed to blit frame");
				let img = ctx.render_ctx.make_image(
					source.screen.pixels.width(),
					source.screen.pixels.height(),
					source.screen.pixels.buf().as_bytes(),
					ImageFormat::RgbaPremul,
				).expect("Failed to create image");
				self.frames.push(Frame{img: img, delay: frame.delay as i64 * 10_000_000});
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

impl Widget<ImageData> for Gif {
	fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut ImageData, _env: &Env) {
		match event {
			Event::MouseDown(_) => {
				// TODO: Get rid of this request_anim_frame here
				ctx.request_anim_frame();
				ctx.set_active(true);
			},
			Event::MouseUp(_) => {
				ctx.set_active(false);
			}
			Event::AnimFrame(interval) => {
				self.current_delay -= *interval as i64;
				ctx.request_anim_frame();
				// When we do fine-grained invalidation,
				// no doubt this will be required:
				//ctx.invalidate();
			}
			_ => (),
		}
	}

	fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&ImageData>, _data: &ImageData, _env: &Env) {}

	fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &ImageData, _env: &Env) -> Size {
		bc.debug_check("Gif");
		bc.constrain((self.width as f64 - data.origin.x, self.height as f64 - data.origin.y))
	}

	fn paint(&mut self, ctx: &mut PaintCtx, base_state: &BaseState, data: &ImageData, _env: &Env) {
		// Determine the area of the frame to paint
		let size = base_state.size();
		let src_rect = Rect::from_origin_size(data.origin, size);
		let dst_rect = Rect::from_origin_size(Point::ZERO, size);

		if self.current_delay > 0 {
			// Still more waiting to do, just paint the current frame
			let img = self.current_frame(ctx);
			ctx.render_ctx.draw_image(img, src_rect, dst_rect, InterpolationMode::Bilinear);
		} else {
			// Paint until there's a delay specified
			let start_frame = self.current_frame;
			while self.current_delay <= 0 {
				// Paint the next frame
				let img = self.next_frame(ctx);
				ctx.render_ctx.draw_image(img, src_rect, dst_rect, InterpolationMode::Bilinear);
				// Detect infinite loops due to GIFs with only 0-delay frames
				if self.current_frame == start_frame {
					break;
				}
			}
		}

		// If active, paint a border on top of the edge of the image
		// TODO: What if it's a 1px image?
		if base_state.is_active() {
			let brush = ctx.render_ctx.solid_brush(Color::rgb8(245, 132, 66));
			let width = 1.0;

			// Top
			if data.origin.y == 0.0 {
				let line = Line::new((dst_rect.x0, dst_rect.y0 + 0.5), (dst_rect.x1, dst_rect.y0 + 0.5));
				ctx.render_ctx.stroke(line, &brush, width);
			}
			// Right
			if data.origin.x == self.width as f64 - size.width {
				let line = Line::new((dst_rect.x1 - 0.5, dst_rect.y0), (dst_rect.x1 - 0.5, dst_rect.y1));
				ctx.render_ctx.stroke(line, &brush, width);
			}
			// Bottom
			if data.origin.y == self.height as f64 - size.height {
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