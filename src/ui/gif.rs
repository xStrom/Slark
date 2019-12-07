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

use druid::{ BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget };
use druid::kurbo::{Rect, Size };
use druid::piet::{RenderContext, Image, ImageFormat, InterpolationMode};

use gif::{Reader, Decoder, SetParameter};
use gif_dispose::*;
use rgb::*;

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

		Gif{ width: width, height: height, source: Some(source), frames: Vec::new(), current_frame: 0, current_delay: 0 }
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

impl Widget<u32> for Gif {
	fn paint(&mut self, ctx: &mut PaintCtx, base_state: &BaseState, _data: &u32, _env: &Env) {
		// Determine the area of the frame to paint
		let size = base_state.size();
		let rect = Rect::new(0.0, 0.0, size.width, size.height);

		if self.current_delay > 0 {
			// Still more waiting to do, just paint the current frame
			let img = self.current_frame(ctx);
			ctx.render_ctx.draw_image(img, rect, rect, InterpolationMode::Bilinear);
		} else {
			// Paint until there's a delay specified
			let start_frame = self.current_frame;
			while self.current_delay <= 0 {
				// Paint the next frame
				let img = self.next_frame(ctx);
				ctx.render_ctx.draw_image(img, rect, rect, InterpolationMode::Bilinear);
				// Detect infinite loops due to GIFs with only 0-delay frames
				if self.current_frame == start_frame {
					break;
				}
			}
		}
	}

	fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &u32, _env: &Env) -> Size {
		bc.debug_check("Gif");
		bc.constrain((self.width as f64, self.height as f64))
	}

	fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut u32, _env: &Env) {
		match event {
			Event::MouseDown(_) => {
				ctx.request_anim_frame();
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

	fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&u32>, _data: &u32, _env: &Env) {}
}