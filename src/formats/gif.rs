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

use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use druid::kurbo::Size;
use gif_dispose::Screen;
use imgref::ImgVec;
use rgb::{RGB8, RGBA8};

use crate::image::Frame;

pub fn open_async(path: &Path) -> (Receiver<Frame>, Size) {
    let file = File::open(path).expect("Failed to open file");
    let mut gif_opts = gif::DecodeOptions::new();
    gif_opts.set_color_output(gif::ColorOutput::Indexed);

    let mut decoder = gif_opts.read_info(file).expect("Failed to read info");
    let width = decoder.width() as usize;
    let height = decoder.height() as usize;
    let global_palette = decoder.global_palette().map(convert_pixels);

    let mut screen = Screen::new(width, height, RGBA8::default(), global_palette);

    let (sender, receiver) = channel();

    let debug_filename = String::from(path.to_str().expect("GIF path is invalid UTF-8"));

    thread::spawn(move || {
        let start = Instant::now();
        // NOTE: The decoding/bliting is surprisingly slow, especially in debug builds
        while let Some(frame) = decoder.read_next_frame().expect("Failed to read next frame") {
            screen.blit_frame(frame).expect("Failed to blit frame");
            let pixel_ref = screen.pixels.as_ref();
            let (buf, width, height) = pixel_ref.to_contiguous_buf();
            let image = ImgVec::<RGBA8>::new(Vec::from(buf), width, height);
            sender
                .send(Frame {
                    image: image,
                    delay: frame.delay as i64 * 10_000_000,
                })
                .expect("Failed to send frame source");
        }
        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
    });

    (receiver, Size::new(width as f64, height as f64))
}

#[rustfmt::skip]
fn convert_pixels<T: From<RGB8>>(palette_bytes: &[u8]) -> Vec<T> {
	palette_bytes.chunks(3).map(|byte| {RGB8{r: byte[0], g: byte[1], b: byte[2]}.into()}).collect()
}
