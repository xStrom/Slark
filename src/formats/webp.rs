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

use imgref::ImgVec;
use rgb::RGBA8;

use crate::image::Frame;

// TODO: Figure out if we can determine the whole image dimensions before the first frame and return it as Size
pub fn open_async(path: &Path) -> Receiver<Frame> {
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
            let pixels = frame
                .data()
                .chunks(4)
                .map(|bytes| RGBA8 {
                    r: bytes[0],
                    g: bytes[1],
                    b: bytes[2],
                    a: bytes[3],
                })
                .collect();
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

    receiver
}
