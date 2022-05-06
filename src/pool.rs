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

use std::io::{self, prelude::*, BufReader};
use std::sync::mpsc::Receiver;
use std::thread;

use druid::ExtEventSink;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};

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
pub fn initialize(receiver: Receiver<ExtEventSink>, filenames: &[String]) -> bool {
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

fn claim_primacy(receiver: Receiver<ExtEventSink>) {
    let listener = LocalSocketListener::bind(PIPE_NAME).expect("Couldn't bind");

    thread::spawn(move || {
        match receiver.recv() {
            Ok(event_sink) => {
                for conn in listener.incoming().filter_map(handle_error) {
                    //conn.write_all(b"Hello from server!\n").expect("Couldn't write");
                    let mut conn = BufReader::new(conn);
                    let mut buffer = String::new();
                    match conn.read_line(&mut buffer) {
                        Ok(line_len) => {
                            let filename = String::from(buffer.trim());
                            event_sink
                                .submit_command(crate::ui::COMMAND_ADD_IMAGE, filename, druid::Target::Global)
                                .expect("Couldn't submit command");
                        }
                        Err(error) => {
                            eprintln!("Couldn't read line: {}", error);
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("Failed to get event sink: {}", error);
            }
        }
    });
}
