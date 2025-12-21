// Copyright 2022 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::sync::mpsc;
use std::{env, thread};

use masonry::core::{ErasedAction, NewWidget, WidgetId, WidgetMut, WidgetTag};
use masonry::theme::default_property_set;
use masonry_winit::app::{AppDriver, DriverCtx, EventLoop, EventLoopProxy, MasonryUserEvent, NewWindow, WindowId};
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

    let event_loop = EventLoop::with_user_event().build().unwrap();
    let event_proxy = event_loop.create_proxy();

    let driver = Driver {
        event_proxy: event_proxy.clone(),
        dummy_widget_id: WidgetId::next(),
    };

    match sender.send((event_proxy, window_id, widget_id)) {
        Ok(_) => (),
        Err(error) => {
            eprintln!("Failed to send event proxy: {}", error);
        }
    }

    masonry_winit::app::run_with(event_loop, vec![new_window], driver, default_property_set()).expect("run failed");
}

#[derive(Debug)]
pub enum FileDialogResult {
    Open(PathBuf),
    Save(PathBuf),
}

struct Driver {
    event_proxy: EventLoopProxy,
    dummy_widget_id: WidgetId,
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
        } else if action.is::<FileDialogResult>() {
            match *action.downcast::<FileDialogResult>().unwrap() {
                FileDialogResult::Open(path) => {
                    let render_root = ctx.render_root(window_id);
                    render_root.edit_widget_with_tag(
                        WidgetTag::named("surface"),
                        |mut this: WidgetMut<'_, Surface>| {
                            this.widget.set_project(Project::open(path));
                            this.ctx.children_changed();
                        },
                    );
                }
                FileDialogResult::Save(path) => {
                    let render_root = ctx.render_root(window_id);
                    // TODO: Doesn't actually need mut widget?
                    render_root.edit_widget_with_tag(WidgetTag::named("surface"), |this: WidgetMut<'_, Surface>| {
                        this.widget.project.save(&path);
                    });
                }
            }
        } else if let Some(action) = action.downcast_ref::<SurfaceAction>() {
            let window_handle = ctx.window(window_id).handle();
            // TODO: File dialogs seem a bit slower, at least on cold launch, than Druid.
            //       Figure out if maybe some lazy initialization could be done earlier.
            match action {
                SurfaceAction::ShowOpenPanel => {
                    let dialog = FileDialog::new()
                        .set_parent(window_handle)
                        .add_filter(FILE_DESC, &[FILE_EXT]);
                    let event_proxy = self.event_proxy.clone();
                    let dummy_widget_id = self.dummy_widget_id;
                    thread::spawn(move || {
                        if let Some(path) = dialog.pick_file() {
                            event_proxy
                                .send_event(MasonryUserEvent::Action(
                                    window_id,
                                    Box::new(FileDialogResult::Open(path)),
                                    dummy_widget_id,
                                ))
                                .expect("Couldn't send file dialog open result event");
                        } else {
                            tracing::debug!("Canceled open panel");
                        }
                    });
                }
                SurfaceAction::ShowSavePanel => {
                    let dialog = FileDialog::new()
                        .set_parent(window_handle)
                        .add_filter(FILE_DESC, &[FILE_EXT]);
                    let event_proxy = self.event_proxy.clone();
                    let dummy_widget_id = self.dummy_widget_id;
                    thread::spawn(move || {
                        if let Some(path) = dialog.save_file() {
                            event_proxy
                                .send_event(MasonryUserEvent::Action(
                                    window_id,
                                    Box::new(FileDialogResult::Save(path)),
                                    dummy_widget_id,
                                ))
                                .expect("Couldn't send file dialog save result event");
                        } else {
                            tracing::debug!("Canceled save panel");
                        }
                    });
                }
            }
        } else {
            println!("Got some action: {:?}", action);
        }
    }
}
