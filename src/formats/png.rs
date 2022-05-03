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
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use imgref::ImgVec;
use png::ColorType;
use rgb::RGBA8;

use crate::image::Frame;

// TODO: Figure out if we can determine the whole image dimensions before the first frame and return it as Size
pub fn open_async(path: &Path) -> Receiver<Frame> {
    let file = File::open(path).expect("Failed to open file");

    let (sender, receiver) = channel();

    let debug_filename = String::from(path.to_str().expect("PNG path is invalid UTF-8"));

    thread::spawn(move || {
        let start = Instant::now();

        // TODO: Make sure that transparency works properly in APNG.
        // TODO: Figure out the issues with the walking APNG.

        let decoder = png::Decoder::new(file);
        let mut reader = decoder.read_info().unwrap();

        let info = reader.info();
        println!("PNG tRNS: {:?}", info.trns);
        println!("PNG palette: {:?}", info.palette);

        let trns = if let Some(trns) = &info.trns {
            let mut vec: Vec<u8> = Vec::new();
            for b in trns.iter() {
                vec.push(*b);
            }
            Some(vec)
        } else {
            None
        };

        // Allocate the output buffer.
        let mut buf = vec![0; reader.output_buffer_size()];
        // Read the next frame. An APNG might contain multiple frames.
        loop {
            match reader.next_frame(&mut buf) {
                Ok(info) => {
                    let (mut width, mut height) = (info.width as usize, info.height as usize);

                    // Grab the bytes of the image.
                    let bytes = &buf[..info.buffer_size()];

                    let mut delay = 0;
                    // Inspect more details of the last read frame.
                    if let Some(more_info) = reader.info().frame_control {
                        let mut den = more_info.delay_den as u64;
                        if den == 0 {
                            den = 100;
                        }
                        delay = (1_000_000_000 * (more_info.delay_num as u64) / den) as i64;

                        (width, height) = (more_info.width as usize, more_info.height as usize);

                        if more_info.x_offset != 0 || more_info.y_offset != 0 {
                            println!("Saw offsets: {} {}", more_info.x_offset, more_info.y_offset);
                        }
                    }

                    println!(
                        "Found another PNG frame for {} which has {} bytes of {:?} and {} x {}",
                        debug_filename,
                        bytes.len(),
                        info.color_type,
                        info.width,
                        info.height
                    );

                    let mut data = Vec::<u8>::with_capacity(info.width as usize * info.height as usize * 4);

                    match info.color_type {
                        ColorType::Grayscale => {
                            println!("Unimplemented color type {:?} for PNG.", info.color_type)
                        }
                        ColorType::GrayscaleAlpha => {
                            println!("Unimplemented color type {:?} for PNG.", info.color_type)
                        }
                        ColorType::Indexed => {
                            println!("Unimplemented color type {:?} for PNG.", info.color_type)
                        }
                        ColorType::Rgb => {
                            let mut i = 0;
                            for b in bytes.iter() {
                                data.push(*b);
                                i += 1;
                                if i == 3 {
                                    i = 0;
                                    match &trns {
                                        Some(trns) => {
                                            let len = data.len();
                                            if trns[0] == data[len - 3]
                                                && trns[1] == data[len - 2]
                                                && trns[2] == data[len - 1]
                                            {
                                                data.push(0);
                                            } else {
                                                data.push(255);
                                            }
                                        }
                                        None => data.push(255),
                                    }
                                }
                            }
                        }
                        ColorType::Rgba => {
                            for b in bytes.iter() {
                                data.push(*b);
                            }
                        }
                    }

                    let pixels = data
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
                            delay: delay,
                        })
                        .expect("Failed to send frame source");
                }
                Err(error) => {
                    println!("PNG reader error: {}", error);
                    break;
                }
            }
        }

        println!("Fully decoded {} in {:?}", debug_filename, start.elapsed());
    });

    receiver
}
