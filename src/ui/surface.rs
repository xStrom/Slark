// Copyright 2019 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, PaintCtx, PointerButton, PointerEvent, PropertiesMut,
    PropertiesRef, RegisterCtx, ScrollDelta, TextEvent, Widget, WidgetPod,
};
use masonry::dpi::PhysicalPosition;
use masonry::kurbo::{Point, Rect, Size, Vec2};
use masonry::vello::Scene;

use crate::project::{Image as ProjectImage, Project};
use crate::ui::View;

pub struct Surface {
    pub project: Project, // TODO: Make private, and expose saving
    view_trackers: Vec<ViewTracker>,
    active_view: Option<usize>,
    drag: Option<Drag>,
}

impl Surface {
    pub fn new(project: Project) -> Surface {
        let mut view_trackers = Vec::new();
        for project_image in project.images() {
            view_trackers.push(ViewTracker::new(project.dir(), project_image));
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
                view_trackers.push(ViewTracker::new(self.project.dir(), project_image));
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
            .push(ViewTracker::new(self.project.dir(), project_image));
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

#[derive(Debug)]
pub enum SurfaceAction {
    ShowOpenPanel,
    ShowSavePanel,
}

impl Widget for Surface {
    type Action = SurfaceAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for view_tracker in self.view_trackers.iter_mut() {
            ctx.register_child(&mut view_tracker.widget_pod);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {
        // TODO: Something?
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut cids = ChildrenIds::new();
        for &idx in self.project.layers().iter() {
            cids.push(self.view_trackers[idx].widget_pod.id());
        }
        cids
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(mouse_event) => {
                if mouse_event.button.is_some_and(|btn| btn == PointerButton::Primary) {
                    ctx.request_focus(); // TODO: Worth using focus fallback mechanism instead?
                    ctx.capture_pointer();
                    // Always clear the currently active view
                    if let Some(view_id) = self.active_view {
                        let (c, _) = ctx.get_raw_mut(&mut self.view_trackers[view_id].widget_pod);
                        c.selected = false;
                        self.active_view = None;
                    }
                    // Locate the topmost layer that gets hit
                    for &idx in self.project.layers().iter().rev() {
                        let view_tracker = &mut self.view_trackers[idx];
                        let rect = Rect::from_origin_size(view_tracker.origin_zoomed(), view_tracker.size);
                        // TODO: logic/physical Masonry situation is big questionmark
                        let mouse_pos = ctx.local_position(mouse_event.state.position);
                        if rect.contains(mouse_pos) {
                            // Set active view
                            self.active_view = Some(view_tracker.id);
                            let (c, _) = ctx.get_raw_mut(&mut view_tracker.widget_pod);
                            c.selected = true;
                            // Start the drag event
                            self.drag = Some(Drag {
                                view_id: view_tracker.id,
                                start: mouse_pos,
                            });
                            break;
                        }
                    }
                }
            }
            PointerEvent::Move(mouse_event) => {
                if let Some(drag) = &mut self.drag {
                    if let Some(view_tracker) = self.view_trackers.iter_mut().find(|vt| vt.id == drag.view_id) {
                        // TODO: logic/physical Masonry situation is big questionmark
                        let mouse_pos = ctx.local_position(mouse_event.current.position);
                        self.project.set_origin(
                            view_tracker.id,
                            view_tracker.adjust_origin(&ctx.size(), mouse_pos - drag.start),
                        );
                        drag.start = mouse_pos;
                        ctx.request_layout();
                    }
                }
            }
            PointerEvent::Up(mouse_event) => {
                if mouse_event.button.is_some_and(|btn| btn == PointerButton::Primary) {
                    if let Some(drag) = &self.drag {
                        let view_tracker = &mut self.view_trackers[drag.view_id];
                        // TODO: logic/physical Masonry situation is big questionmark
                        let mouse_pos = ctx.local_position(mouse_event.state.position);
                        self.project.set_origin(
                            view_tracker.id,
                            view_tracker.adjust_origin(&ctx.size(), mouse_pos - drag.start),
                        );
                        self.drag = None;
                        ctx.request_layout();
                    }
                }
            }
            PointerEvent::Scroll(mouse_event) => {
                let delta = match mouse_event.delta {
                    ScrollDelta::PixelDelta(PhysicalPosition::<f64> { x, y }) => -Vec2 { x, y },
                    ScrollDelta::LineDelta(x, y) => {
                        -Vec2 {
                            x: x as f64,
                            y: y as f64,
                        } * 120.0
                    }
                    _ => Vec2::ZERO,
                } * ctx.get_scale_factor();
                if let Some(view_id) = self.active_view {
                    let (c, mut cctx) = ctx.get_raw_mut(&mut self.view_trackers[view_id].widget_pod);
                    if delta.y < 0.0 {
                        c.zoom.turn_the_knob(1);
                        cctx.request_layout();
                    } else if delta.y > 0.0 {
                        c.zoom.turn_the_knob(-1);
                        cctx.request_layout();
                    }
                    self.project.set_zoom(self.view_trackers[view_id].id, c.zoom);
                    println!("Scale factor now: {}", c.zoom.scale_factor());
                } else {
                    if delta.y < 0.0 {
                        self.view_trackers.iter_mut().for_each(|vt| {
                            let (c, mut cctx) = ctx.get_raw_mut(&mut vt.widget_pod);
                            c.zoom.turn_the_global_knob(1);
                            cctx.request_layout();
                            vt.zoom += 1;
                        });
                    } else {
                        self.view_trackers.iter_mut().for_each(|vt| {
                            let (c, mut cctx) = ctx.get_raw_mut(&mut vt.widget_pod);
                            c.zoom.turn_the_global_knob(-1);
                            cctx.request_layout();
                            vt.zoom -= 1;
                        });
                    }
                    ctx.request_layout();
                }
            }
            _ => (), // TODO: Do we need to handle Cancel?
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        match event {
            TextEvent::Keyboard(kb_event) if kb_event.state == KeyState::Down => match kb_event.key {
                Key::Named(named_key) => match named_key {
                    NamedKey::ArrowUp => {
                        self.view_trackers.iter_mut().for_each(|vt| {
                            let (c, mut cctx) = ctx.get_raw_mut(&mut vt.widget_pod);
                            c.zoom.turn_the_global_knob(1);
                            cctx.request_layout();
                            vt.zoom += 1;
                        });
                        ctx.request_layout();
                    }
                    NamedKey::ArrowDown => {
                        self.view_trackers.iter_mut().for_each(|vt| {
                            let (c, mut cctx) = ctx.get_raw_mut(&mut vt.widget_pod);
                            c.zoom.turn_the_global_knob(-1);
                            cctx.request_layout();
                            vt.zoom -= 1;
                        });
                        ctx.request_layout();
                    }
                    _ => (),
                },
                _ => (),
            },
            TextEvent::Keyboard(kb_event) if kb_event.state == KeyState::Up => match &kb_event.key {
                Key::Named(named_key) => match named_key {
                    NamedKey::Delete => {
                        if let Some(view_id) = self.active_view {
                            self.remove(view_id);
                            ctx.children_changed();
                        }
                    }
                    NamedKey::PageUp => {
                        if let Some(view_id) = self.active_view {
                            self.project.shift_layer(view_id, 1);
                        }
                    }
                    NamedKey::PageDown => {
                        if let Some(view_id) = self.active_view {
                            self.project.shift_layer(view_id, -1);
                        }
                    }
                    _ => (),
                },
                // TODO: Using code instead probably makes more sense for non-latin keyboards
                Key::Character(c) if kb_event.modifiers.ctrl() => {
                    if c == "s" {
                        ctx.submit_action::<Self::Action>(SurfaceAction::ShowSavePanel);
                    } else if c == "o" {
                        ctx.submit_action::<Self::Action>(SurfaceAction::ShowOpenPanel);
                    }
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> Size {
        bc.debug_check("Surface");

        // Determine the layout for all the views
        for view_tracker in self.view_trackers.iter_mut() {
            // We give unbounded constraints as we'll clip everything at the surface level
            let size = ctx.run_layout(&mut view_tracker.widget_pod, &BoxConstraints::UNBOUNDED);
            view_tracker.size = size;
            let origin_zoomed = view_tracker.origin_zoomed();
            ctx.place_child(&mut view_tracker.widget_pod, origin_zoomed);
        }

        // The surface always uses the whole area provided to it
        let size = bc.max();
        ctx.set_clip_path(size.to_rect());
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}
}

struct ViewTracker {
    id: usize,
    widget_pod: WidgetPod<View>,
    origin: Point, // View's origin in relation to Surface
    size: Size,
    zoom: i32,
}

impl ViewTracker {
    fn new(project_dir: Option<&Path>, project_image: &ProjectImage) -> ViewTracker {
        let image_full_path = project_image.path_abs(project_dir);
        ViewTracker {
            id: project_image.id(),
            widget_pod: WidgetPod::new(View::new(&image_full_path, *project_image.zoom())),
            origin: *project_image.origin(),
            size: Size::ZERO, // TODO: Init with real size?
            zoom: 0,
        }
    }

    fn adjust_origin(&mut self, surface_size: &Size, delta: Vec2) -> Point {
        // Make sure there remains at least 5dp visible on each axis
        let mut origin = self.origin + delta;
        let rect = Rect::from_origin_size(self.origin, self.size);
        let min_x = -(rect.width()) + 5.0;
        let min_y = -(rect.height()) + 5.0;
        let max_x = surface_size.width - 5.0;
        let max_y = surface_size.height - 5.0;
        origin.x = origin.x.max(min_x).min(max_x);
        origin.y = origin.y.max(min_y).min(max_y);
        self.origin = origin;
        origin
    }

    fn origin_zoomed(&self) -> Point {
        (self.origin.to_vec2() * 1.01f64.powi(self.zoom)).to_point()
    }
}

struct Drag {
    view_id: usize,
    start: Point,
}
