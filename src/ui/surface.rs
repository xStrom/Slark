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

use std::path::{Path, PathBuf};

use druid::kurbo::{Point, Rect, Vec2};
use druid::widget::prelude::*;
use druid::{commands, Command, KbKey, Selector, Target, WidgetPod};

use crate::project::{Image as ProjectImage, Project};
use crate::ui::view::{View, ViewData};

pub const COMMAND_ADD_IMAGE: Selector<String> = Selector::new("slark.add_image");

pub struct Surface {
    project: Project,
    view_trackers: Vec<ViewTracker>,
    active_view: Option<usize>,
    drag: Option<Drag>,
}

impl Surface {
    pub fn new(project: Project) -> Surface {
        let mut view_trackers = Vec::new();
        for project_image in project.images() {
            view_trackers.push(ViewTracker::new(project.path(), project_image));
        }
        Surface {
            project: project,
            view_trackers: view_trackers,
            active_view: None,
            drag: None,
        }
    }

    pub fn set_project(&mut self, project: Project) {
        self.project = project;
        self.view_trackers = {
            let mut view_trackers = Vec::new();
            for project_image in self.project.images() {
                view_trackers.push(ViewTracker::new(self.project.path(), project_image));
            }
            view_trackers
        };
        self.active_view = None;
        self.drag = None;
    }

    pub fn add(&mut self, filename: PathBuf) {
        self.project.add(filename);
        let project_image = self.project.images().last().unwrap();
        self.view_trackers
            .push(ViewTracker::new(self.project.path(), project_image));
    }

    // Super fragile function, must be same as the project removal.
    pub fn remove(&mut self, view_id: usize) {
        if self.view_trackers.is_empty() || self.view_trackers.len() <= view_id {
            return;
        } else if self.view_trackers.len() == 1 {
            self.view_trackers.clear();
            self.project.remove(view_id);
            self.drag = None;
            self.active_view = None;
        } else {
            let last_id = self.view_trackers.len() - 1;
            self.view_trackers[last_id].id = view_id;
            self.view_trackers.swap(view_id, last_id);
            self.view_trackers.pop();
            self.project.remove(view_id);

            if let Some(drag) = &self.drag {
                if drag.view_id == view_id {
                    self.drag = None;
                } else if drag.view_id == last_id {
                    self.drag = Some(Drag {
                        view_id: view_id,
                        start: drag.start,
                    });
                }
            }

            self.active_view = match self.active_view {
                Some(a_id) if a_id == view_id => None,
                Some(a_id) if a_id == last_id => Some(view_id),
                old => old,
            };
        }
    }
}

impl Widget<u64> for Surface {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut u64, env: &Env) {
        let mut hacky_children_added = false;

        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button.is_left() {
                    // TODO: Move this focus request elsewhere?
                    ctx.request_focus();
                    ctx.set_active(true);
                    // Always clear the currently active view
                    if let Some(view_id) = self.active_view {
                        self.view_trackers[view_id].data.selected = false;
                        self.active_view = None;
                    }
                    // Locate the topmost layer that gets hit
                    for &id in self.project.layers().iter().rev() {
                        let view_tracker = &mut self.view_trackers[id];
                        let rect = view_tracker.widget_pod.layout_rect();
                        if rect.contains(mouse_event.pos) {
                            // Set active view
                            self.active_view = Some(view_tracker.id);
                            view_tracker.data.selected = true;
                            // Start the drag event
                            self.drag = Some(Drag {
                                view_id: view_tracker.id,
                                start: mouse_event.pos,
                            });
                            break;
                        }
                    }
                }
            }
            Event::MouseMove(mouse_event) => {
                if let Some(drag) = &mut self.drag {
                    if let Some(view_tracker) = self.view_trackers.iter_mut().find(|vt| vt.id == drag.view_id) {
                        self.project.set_origin(
                            view_tracker.id,
                            view_tracker.adjust_origin(&ctx.size(), mouse_event.pos - drag.start),
                        );
                        drag.start = mouse_event.pos;
                        ctx.request_layout();
                    }
                }
            }
            Event::MouseUp(mouse_event) => {
                if mouse_event.button.is_left() {
                    if let Some(drag) = &self.drag {
                        let view_tracker = &mut self.view_trackers[drag.view_id];
                        self.project.set_origin(
                            view_tracker.id,
                            view_tracker.adjust_origin(&ctx.size(), mouse_event.pos - drag.start),
                        );
                        self.drag = None;
                        ctx.request_layout();
                    }
                }
            }
            Event::Wheel(mouse_event) => {
                if let Some(view_id) = self.active_view {
                    if mouse_event.wheel_delta.y < 0.0 {
                        self.view_trackers[view_id].data.zoom(1);
                    } else if mouse_event.wheel_delta.y > 0.0 {
                        self.view_trackers[view_id].data.zoom(-1);
                    }
                    self.project
                        .set_zoom(self.view_trackers[view_id].id, self.view_trackers[view_id].data.zoom);
                    ctx.request_update();
                    println!("Scale factor now: {}", self.view_trackers[view_id].data.scale_factor());
                }
            }
            Event::KeyUp(key_event) => match &key_event.key {
                KbKey::Delete => {
                    if let Some(view_id) = self.active_view {
                        self.remove(view_id);
                        ctx.children_changed();
                    }
                }
                KbKey::PageUp => {
                    if let Some(view_id) = self.active_view {
                        self.project.shift_layer(view_id, 1);
                    }
                }
                KbKey::PageDown => {
                    if let Some(view_id) = self.active_view {
                        self.project.shift_layer(view_id, -1);
                    }
                }
                KbKey::Character(ch) => {
                    if key_event.mods.ctrl() {
                        match ch.as_str() {
                            "s" => {
                                ctx.submit_command(Command::new(
                                    commands::SHOW_SAVE_PANEL,
                                    self.project.file_dialog_options(),
                                    Target::Auto,
                                ));
                            }
                            "o" => {
                                ctx.submit_command(Command::new(
                                    commands::SHOW_OPEN_PANEL,
                                    self.project.file_dialog_options(),
                                    Target::Auto,
                                ));
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            },
            Event::Command(command) => {
                if command.is(commands::SAVE_FILE_AS) {
                    let info = command.get_unchecked(commands::SAVE_FILE_AS);
                    self.project.save(info.path());
                } else if command.is(commands::OPEN_FILE) {
                    let info = command.get_unchecked(commands::OPEN_FILE);
                    self.set_project(Project::open(PathBuf::from(info.path())));
                    // Need to inform of children changes
                    ctx.children_changed();
                    hacky_children_added = true;
                } else if command.is(COMMAND_ADD_IMAGE) {
                    let filename = command.get_unchecked(COMMAND_ADD_IMAGE);
                    self.add(filename.into());
                    // Need to inform of children changes
                    ctx.children_changed();
                    hacky_children_added = true;
                }
            }
            _ => (),
        }

        if !hacky_children_added {
            // Pass the event to all the views
            for view_tracker in self.view_trackers.iter_mut() {
                view_tracker.widget_pod.event(ctx, event, &mut view_tracker.data, env);
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &u64, env: &Env) {
        // Pass the lifecycle to all the views
        for view_tracker in self.view_trackers.iter_mut() {
            view_tracker.widget_pod.lifecycle(ctx, event, &view_tracker.data, env);
        }
        match event {
            LifeCycle::HotChanged(hot) => {
                //println!("Hot changed: {}", hot);
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &u64, _data: &u64, env: &Env) {
        // Pass the update to all the views
        for view_tracker in self.view_trackers.iter_mut() {
            view_tracker.widget_pod.update(ctx, &view_tracker.data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &u64, env: &Env) -> Size {
        bc.debug_check("Surface");

        // Determine the layout for all the views
        for view_tracker in self.view_trackers.iter_mut() {
            // We give unbounded constraints as we'll clip everything at the surface level
            view_tracker
                .widget_pod
                .layout(ctx, &BoxConstraints::UNBOUNDED, &view_tracker.data, env);
            view_tracker
                .widget_pod
                .set_origin(ctx, &view_tracker.data, env, view_tracker.origin);
        }

        // The surface always uses the whole area provided to it
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &u64, env: &Env) {
        // Clip the overflow
        let size = ctx.size();
        ctx.render_ctx.clip(Rect::from_origin_size(Point::ZERO, size));

        // Paint all the views in the configured layer order
        for &id in self.project.layers().iter() {
            let view_tracker = &mut self.view_trackers[id];
            view_tracker.widget_pod.paint(ctx, &view_tracker.data, env);
        }
    }
}

struct ViewTracker {
    id: usize,
    widget_pod: WidgetPod<ViewData, View>,
    origin: Point, // View's origin in relation to Surface
    data: ViewData,
}

impl ViewTracker {
    fn new(project_path: Option<&Path>, project_image: &ProjectImage) -> ViewTracker {
        let image_full_path = match project_path {
            Some(path) => match path.parent() {
                Some(path) => path.join(project_image.path()).canonicalize().unwrap(), // TODO: This is a common unwrap panic, if .ark contains path which doesn't exist
                None => project_image.path().to_path_buf(),
            },
            None => project_image.path().to_path_buf(),
        };

        ViewTracker {
            id: project_image.id(),
            widget_pod: WidgetPod::new(View::new(&image_full_path)),
            origin: *project_image.origin(),
            data: ViewData {
                selected: false,
                zoom: project_image.zoom(),
            },
        }
    }

    fn adjust_origin(&mut self, surface_size: &Size, delta: Vec2) -> Point {
        // Make sure there remains at least 5dp visible on each axis
        let mut origin = self.origin + delta;
        let rect = self.widget_pod.layout_rect();
        let min_x = -(rect.width()) + 5.0;
        let min_y = -(rect.height()) + 5.0;
        let max_x = surface_size.width - 5.0;
        let max_y = surface_size.height - 5.0;
        origin.x = origin.x.max(min_x).min(max_x);
        origin.y = origin.y.max(min_y).min(max_y);
        self.origin = origin;
        origin
    }
}

struct Drag {
    view_id: usize,
    start: Point,
}
