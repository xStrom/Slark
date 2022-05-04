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

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use druid::kurbo::Size;
use imgref::ImgVec;
use jpeg_decoder::Decoder;
use rgb::RGBA8;

use crate::image::Frame;

pub fn open_async(path: &Path) -> (Receiver<Frame>, Size) {
    let file = File::open(path).expect("Failed to open file");

    let (sender, receiver) = channel();

    let debug_filename = String::from(path.to_str().expect("JPEG path is invalid UTF-8"));

    let mut decoder = Decoder::new(BufReader::new(file));
    decoder.read_info().expect("Failed to read metadata");
    let metadata = decoder.info().unwrap();
    let size = Size::new(metadata.width as f64, metadata.height as f64);

    thread::spawn(move || {
        let start = Instant::now();

        let pixels = decoder.decode().expect("Failed to decode JPEG image");
        // TODO: Look into metadata.pixel_format and whether we need to throw a match statement in here to handle differences.
        let pixels = pixels
            .chunks(3)
            .map(|bytes| RGBA8 {
                r: bytes[0],
                g: bytes[1],
                b: bytes[2],
                a: 255,
            })
            .collect();
        let image = ImgVec::new(pixels, metadata.width as usize, metadata.height as usize);

        sender
            .send(Frame { image: image, delay: 0 })
            .expect("Failed to send frame source");

        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
    });

    (receiver, size)
}
