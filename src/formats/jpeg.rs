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

use imgref::ImgVec;
use rgb::RGBA8;

use crate::image::Frame;

// TODO: Figure out if we can determine the whole image dimensions before the first frame and return it as Size
pub fn open_async(path: &Path) -> Receiver<Frame> {
    let file = File::open(path).expect("Failed to open file");

    let (sender, receiver) = channel();

    let debug_filename = String::from(path.to_str().expect("JPEG path is invalid UTF-8"));

    thread::spawn(move || {
        let start = Instant::now();

        let mut decoder = jpeg_decoder::Decoder::new(BufReader::new(file));
        let pixels = decoder.decode().expect("Failed to decode JPEG image");
        let metadata = decoder.info().unwrap();

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

    receiver
}
