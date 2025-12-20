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

use masonry::core::{ErasedAction, NewWidget, WidgetId, WidgetMut, WidgetTag};
use masonry::theme::default_property_set;
use masonry_winit::app::{AppDriver, DriverCtx, EventLoop, NewWindow, WindowId};
use masonry_winit::winit::dpi::LogicalSize;
use masonry_winit::winit::window::Window;

mod formats;
mod image;

mod ui;
use rfd::FileDialog;
use ui::ui_root;

use crate::pool::AddImage;
use crate::project::{Project, FILE_DESC, FILE_EXT};
use crate::ui::{Surface, SurfaceAction};

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

    let window_attributes = Window::default_attributes()
        .with_title("Slark")
        .with_resizable(true)
        .with_min_inner_size(LogicalSize::new(320.0, 240.0))
        .with_inner_size(LogicalSize::new(1024, 768));

    let widget_id = WidgetId::next();
    let (root_inner, root_props) = ui_root(filenames);
    let root_widget = NewWidget::new_with(root_inner, widget_id, Default::default(), root_props);

    let window_id = WindowId::next();
    let new_window = NewWindow::new_with_id(window_id, window_attributes, root_widget.erased());

    let driver = Driver {
        // ...
    };

    let event_loop = EventLoop::with_user_event().build().unwrap();
    let event_proxy = event_loop.create_proxy();

    match sender.send((event_proxy, window_id, widget_id)) {
        Ok(_) => (),
        Err(error) => {
            eprintln!("Failed to send event proxy: {}", error);
        }
    }

    masonry_winit::app::run_with(event_loop, vec![new_window], driver, default_property_set()).expect("run failed");
}

struct Driver {
    // ...
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        ctx: &mut DriverCtx<'_, '_>,
        _widget_id: WidgetId,
        action: ErasedAction,
    ) {
        if let Some(add_image) = action.downcast_ref::<AddImage>() {
            let render_root = ctx.render_root(window_id);
            render_root.edit_widget_with_tag(WidgetTag::named("surface"), |mut this: WidgetMut<'_, Surface>| {
                this.widget.add(add_image.filename.clone().into());
                this.ctx.children_changed();
            });
        } else if let Some(action) = action.downcast_ref::<SurfaceAction>() {
            let window = ctx.window(window_id).handle();
            // TODO: Overwriting dialog crashes .. because of winit eventloop being blocked?
            // TODO: Figure out why these are so slow compared to Druid
            // TODO: Figure out how to keep app loop going while waiting for the answer (async? just new thread?)
            match action {
                SurfaceAction::ShowOpenPanel => {
                    if let Some(path) = FileDialog::new()
                        .set_parent(window)
                        .add_filter(FILE_DESC, &[FILE_EXT])
                        .pick_file()
                    {
                        let render_root = ctx.render_root(window_id);
                        render_root.edit_widget_with_tag(
                            WidgetTag::named("surface"),
                            |mut this: WidgetMut<'_, Surface>| {
                                this.widget.set_project(Project::open(path));
                                this.ctx.children_changed();
                            },
                        );
                    } else {
                        tracing::debug!("Canceled open panel");
                    }
                }
                SurfaceAction::ShowSavePanel => {
                    if let Some(path) = FileDialog::new()
                        .set_parent(window)
                        .add_filter(FILE_DESC, &[FILE_EXT])
                        .save_file()
                    {
                        let render_root = ctx.render_root(window_id);
                        // TODO: Doesn't actually need mut widget?
                        render_root.edit_widget_with_tag(
                            WidgetTag::named("surface"),
                            |this: WidgetMut<'_, Surface>| {
                                this.widget.project.save(&path);
                            },
                        );
                    } else {
                        tracing::debug!("Canceled save panel");
                    }
                }
            }
        } else {
            println!("Got some action: {:?}", action);
        }
    }
}
