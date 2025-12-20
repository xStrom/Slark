// Copyright 2022 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use imgref::ImgVec;
use masonry::kurbo::Size;
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
