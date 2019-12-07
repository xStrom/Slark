/*
    Copyright 2019 Kaur Kuut <admin@kaurkuut.com>

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

use druid::{ BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget, WidgetPod };
use druid::kurbo::{Point, Rect, Size, Vec2};
use druid::piet::{PaintBrush, RenderContext};

use crate::ui::gif::{Gif};

pub struct Surface {
    next_id: usize,
    images: Vec<Image>,
    border: Option<Border>,
    drag: Option<Drag>,
}

struct Image {
    id: usize,
    widget_pod: WidgetPod<u32, Gif>,
    origin: Point,
}

pub struct Border {
    width: f64,
    brush: PaintBrush,
}

impl Border {
    pub fn new(width: f64, brush: PaintBrush) -> Border {
        Border{width: width, brush: brush}
    }
}

struct Drag {
    start: Point,
    image_id: usize,
}

impl Surface {
    pub fn new() -> Surface {
        Surface{next_id: 0, images: Vec::new(), border: None, drag: None}
    }

    pub fn set_border(&mut self, border: Option<Border>) {
        self.border = border;
    }

    pub fn add(&mut self, filename: &str) {
        self.images.push(Image{
            id: self.next_id,
            widget_pod: WidgetPod::new(Gif::new(filename)),
            origin: Point::ZERO,
        });
        self.next_id += 1;
    }
}

impl Widget<u32> for Surface {
    fn paint(&mut self, ctx: &mut PaintCtx, base_state: &BaseState, data: &u32, env: &Env) {
        // Clip the overflow
        let size = base_state.size();
        ctx.render_ctx.clip(Rect::from_origin_size(Point::ZERO, size));

        // Paint all the images
        for image in self.images.iter_mut() {
            image.widget_pod.paint_with_offset(ctx, data, env);
        }

        // Paint border, if there is one
        if let Some(border) = &self.border {
            let offset = border.width / 2.0;
            let rect = Rect::from_origin_size(
                (offset, offset),
                Size::new(size.width - border.width, size.height - border.width),
            );
            ctx.render_ctx.stroke(rect, &border.brush, border.width);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &u32, env: &Env) -> Size {
        bc.debug_check("Surface");

        // Reserve border area for the surface to make sure things work well
        let border_width = match &self.border {
            Some(border) => border.width,
            None => 0.0,
        };
        let child_bc = bc.shrink((2.0 * border_width, 2.0 * border_width)).loosen();

        // Set the layout for all the images
        for image in self.images.iter_mut() {
            let origin = image.origin + Vec2::new(border_width, border_width);
            let size = image.widget_pod.layout(ctx, &child_bc, data, env);
            image.widget_pod.set_layout_rect(Rect::from_origin_size(origin, size));
        }

        // The surface always uses the whole area provided to it
        bc.max()
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut u32, env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button.is_left() {
                    for image in self.images.iter_mut() {
                        let rect = image.widget_pod.get_layout_rect();
                        if rect.contains(mouse_event.pos) {
                            // Start the drag event
                            self.drag = Some(Drag{start: mouse_event.pos, image_id: image.id});
                            // Send the event to the image as well
                            image.widget_pod.event(ctx, event, data, env);
                            break;
                        }
                    }
                }
            },
            Event::MouseMoved(mouse_event) => {
                if let Some(drag) = &mut self.drag {
                    if let Some(image) = self.images.iter_mut().find(|image| image.id == drag.image_id) {
                        image.origin += mouse_event.pos - drag.start;
                        drag.start = mouse_event.pos;
                    }
                }
            },
            Event::MouseUp(mouse_event) => {
                if mouse_event.button.is_left() {
                    if let Some(drag) = &self.drag {
                        if let Some(image) = self.images.iter_mut().find(|image| image.id == drag.image_id) {
                            image.origin += mouse_event.pos - drag.start;
                            // Send the event to the image as well
                            image.widget_pod.event(ctx, event, data, env);
                        }
                        self.drag = None;
                    }
                }
            },
            _ => {
                // Pass the event to all the images
                for image in self.images.iter_mut() {
                    image.widget_pod.event(ctx, event, data, env);
                }
            },
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&u32>, data: &u32, env: &Env) {
        // Pass the update to all the images
        for image in self.images.iter_mut() {
            image.widget_pod.update(ctx, data, env);
        }
    }
}