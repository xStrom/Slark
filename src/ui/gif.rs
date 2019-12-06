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
use druid::piet::{RenderContext, ImageFormat, InterpolationMode};

use gif::{Decoder, SetParameter};
use gif_dispose::*;
use imgref::*;
use rgb::*;

pub struct Gif {
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
	fn paint(&mut self, ctx: &mut PaintCtx, base_state: &BaseState, _data: &u32, _env: &Env) {
		// TODO: Cache this image
		let img = ctx.make_image(self.width, self.height, self.frames[self.current_frame].buf().as_bytes(), ImageFormat::RgbaPremul).expect("Failed to create image");

		let size = base_state.size();
		let rect = Rect::new(0.0, 0.0, size.width, size.height);
		ctx.render_ctx.draw_image(&img, rect, rect, InterpolationMode::Bilinear);
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
				self.current_frame += 1;
				if self.current_frame == self.frames.len() {
					self.current_frame = 0;
				}
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