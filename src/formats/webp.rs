/*
    Copyright 2022 Kaur Kuut <admin@kaurkuut.com>

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

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use druid::kurbo::Size;
use imgref::ImgVec;
use rgb::RGBA8;
use webp_animation::{ColorMode, Decoder};

use crate::image::Frame;

pub fn open_async(path: &Path) -> (Receiver<Frame>, Size) {
    let buffer = std::fs::read(path).unwrap();

    let (sender, receiver) = channel();

    let debug_filename = String::from(path.to_str().expect("WebP path is invalid UTF-8"));

    let decoder = Decoder::new(&buffer).unwrap();
    let (width, height) = decoder.dimensions();
    let size = Size::new(width as f64, height as f64);

    // We need to drop & re-create the decoder because it doesn't implement Send.
    std::mem::drop(decoder);

    thread::spawn(move || {
        let start = Instant::now();
        let decoder = Decoder::new(&buffer).unwrap();
        let mut prev_timestamp = 0;
        for frame in decoder.into_iter() {
            // The current implementation of webp_animation guarantees using the full image dimensions for every frame.
            if frame.dimensions() != (width, height) {
                println!(
                    "Unexpected frame size for WebP decoding. Expected {} x {} but got {} x {}",
                    width,
                    height,
                    frame.dimensions().0,
                    frame.dimensions().1
                );
            }
            println!(
                "Calculated {} frame delay: {} ms",
                debug_filename,
                (frame.timestamp() - prev_timestamp)
            );
            let pixels = match frame.color_mode() {
                ColorMode::Rgba => frame
                    .data()
                    .chunks(4)
                    .map(|bytes| RGBA8 {
                        r: bytes[0],
                        g: bytes[1],
                        b: bytes[2],
                        a: bytes[3],
                    })
                    .collect(),
                ColorMode::Bgra => frame
                    .data()
                    .chunks(4)
                    .map(|bytes| RGBA8 {
                        r: bytes[2],
                        g: bytes[1],
                        b: bytes[0],
                        a: bytes[3],
                    })
                    .collect(),
            };
            let image = ImgVec::new(pixels, width as usize, height as usize);
            sender
                .send(Frame {
                    image: image,
                    delay: (frame.timestamp() - prev_timestamp) as i64 * 1_000_000,
                })
                .expect("Failed to send frame source");
            prev_timestamp = frame.timestamp();
        }
        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
    });

    (receiver, size)
}
