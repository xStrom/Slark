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

use std::env;
use std::sync::mpsc;

use druid::{AppLauncher, LocalizedString, WindowDesc};

mod formats;
mod image;

mod ui;
use ui::ui_root;

mod pool;
mod project;

fn main() {
    let filenames: Vec<String> = env::args().skip(1).collect();

    let (sender, receiver) = mpsc::channel();

    let exit = pool::initialize(receiver, &filenames);
    if exit {
        println!("Pool initialization requested immediate application exit.");
        return;
    }

    let window = WindowDesc::<u64>::new(ui_root(filenames))
        .title(LocalizedString::new("app_title").with_placeholder("Slark".to_string()))
        //.window_size((400.0, 300.0))
        //.with_min_size((300.0, 200.0));
        .window_size((1024.0, 768.0))
        .with_min_size((320.0, 240.0));
    let launcher = AppLauncher::with_window(window).use_simple_logger();

    let event_sink = launcher.get_external_handle();

    match sender.send(event_sink) {
        Ok(_) => (),
        Err(error) => {
            eprintln!("Failed to send event sink: {}", error);
        }
    }

    launcher.launch(0).expect("launch failed");
}
