// Copyright 2022 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use imgref::ImgVec;
use jpeg_decoder::Decoder;
use masonry::kurbo::Size;
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
