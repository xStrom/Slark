// Copyright 2022 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::{self, prelude::*, BufReader};
use std::sync::mpsc::Receiver;
use std::thread;

use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use masonry::core::WidgetId;
use masonry_winit::app::{EventLoopProxy, MasonryUserEvent, WindowId};

fn handle_error(conn: io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
    match conn {
        Ok(val) => Some(val),
        Err(error) => {
            eprintln!("Incoming connection failed: {}", error);
            None
        }
    }
}

const PIPE_NAME: &str = "/tmp/slark.sock";

/// Application should exit when this function returns `true`.
pub fn initialize(receiver: Receiver<(EventLoopProxy, WindowId, WidgetId)>, filenames: &[String]) -> bool {
    // Attempt to connect to an existing Slark instance
    let conn = LocalSocketStream::connect(PIPE_NAME);

    match conn {
        Ok(mut conn) => {
            if filenames.len() > 0 {
                // TODO: Add support for more than one filename
                conn.write_all(filenames[0].as_bytes())
                    .expect("Couldn't write the filename");
                conn.write_all(b"\n").expect("Couldn't write the newline");
                /*
                let mut conn = BufReader::new(conn);
                let mut buffer = String::new();
                conn.read_line(&mut buffer).expect("couldn't read");
                println!("Server answered: {}", buffer);
                */
                return true;
            }
        }
        Err(error) => match error.kind() {
            io::ErrorKind::NotFound => {
                // Not found? Let's be primary!
                claim_primacy(receiver);
            }
            _ => {
                eprintln!("Failed to connect to the primary Slark instance. {}", error);
            }
        },
    }

    false
}

#[derive(Debug)]
pub struct AddImage {
    pub filename: String,
}

fn claim_primacy(receiver: Receiver<(EventLoopProxy, WindowId, WidgetId)>) {
    let listener = LocalSocketListener::bind(PIPE_NAME).expect("Couldn't bind");

    thread::spawn(move || {
        match receiver.recv() {
            Ok((event_proxy, window_id, widget_id)) => {
                for conn in listener.incoming().filter_map(handle_error) {
                    //conn.write_all(b"Hello from server!\n").expect("Couldn't write");
                    let mut conn = BufReader::new(conn);
                    let mut buffer = String::new();
                    match conn.read_line(&mut buffer) {
                        Ok(_line_len) => {
                            let filename = String::from(buffer.trim());
                            event_proxy
                                .send_event(MasonryUserEvent::Action(
                                    window_id,
                                    Box::new(AddImage { filename }),
                                    widget_id,
                                ))
                                .expect("Couldn't send event");
                        }
                        Err(error) => {
                            eprintln!("Couldn't read line: {}", error);
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("Failed to get event proxy: {}", error);
            }
        }
    });
}
