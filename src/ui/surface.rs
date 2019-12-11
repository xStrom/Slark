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

use druid::kurbo::{Point, Rect, Size, Vec2};
use druid::piet::{PaintBrush, RenderContext};
use druid::{
    BaseState, BoxConstraints, Env, Event, EventCtx, KeyCode, LayoutCtx, PaintCtx, UpdateCtx, Widget, WidgetPod,
};

use crate::ui::gif::{Gif, ImageData};

pub struct Surface {
    next_id: usize,
    images: Vec<Image>,
    images_area: Size,
    border: Option<Border>,
    drag: Option<Drag>,
    active_image: Option<usize>,
    layers: Vec<usize>,
}

struct Image {
    id: usize,
    widget_pod: WidgetPod<ImageData, Gif>,
    origin: Point,
    data: ImageData,
}

impl Image {
    fn adjust_origin(&mut self, images_area: &Size, delta: Vec2) {
        // Calculate the new origin in relation to the surface's image display area
        let mut origin = self.origin - self.data.origin + delta;
        // Make sure there remains at least 1px visible on each axis
        let min_x = -(self.widget_pod.widget().width() as f64) + 1.0;
        let min_y = -(self.widget_pod.widget().height() as f64) + 1.0;
        let max_x = images_area.width - 1.0;
        let max_y = images_area.height - 1.0;
        origin.x = origin.x.max(min_x).min(max_x);
        origin.y = origin.y.max(min_y).min(max_y);
        // Split the new origin between surface images area origin and image origin
        if origin.x < 0.0 {
            self.origin.x = 0.0;
            self.data.origin.x = origin.x.abs();
        } else {
            self.origin.x = origin.x;
            self.data.origin.x = 0.0;
        };
        if origin.y < 0.0 {
            self.origin.y = 0.0;
            self.data.origin.y = origin.y.abs();
        } else {
            self.origin.y = origin.y;
            self.data.origin.y = 0.0;
        };
    }
}

pub struct Border {
    width: f64,
    brush: PaintBrush,
}

impl Border {
    pub fn new(width: f64, brush: PaintBrush) -> Border {
        Border {
            width: width,
            brush: brush,
        }
    }
}

struct Drag {
    start: Point,
    image_id: usize,
}

impl Surface {
    pub fn new() -> Surface {
        Surface {
            next_id: 0,
            images: Vec::new(),
            images_area: Size::ZERO,
            border: None,
            drag: None,
            active_image: None,
            layers: Vec::new(),
        }
    }

    pub fn set_border(&mut self, border: Option<Border>) {
        self.border = border;
    }

    pub fn add(&mut self, filename: &str) {
        self.images.push(Image {
            id: self.next_id,
            widget_pod: WidgetPod::new(Gif::new(filename)),
            origin: Point::ZERO,
            data: ImageData { origin: Point::ZERO },
        });
        self.layers.push(self.next_id);
        self.next_id += 1;
    }
}

impl Widget<u32> for Surface {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut u32, env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button.is_left() {
                    // TODO: Move this request elsewhere?
                    ctx.request_focus();
                    // Always clear the currently active image
                    if let Some(active_image) = self.active_image {
                        self.images[active_image].widget_pod.set_active(false);
                        self.active_image = None;
                    }
                    // Locate the topmost layer that gets hit
                    for &id in self.layers.iter().rev() {
                        let image = &mut self.images[id];
                        let rect = image.widget_pod.get_layout_rect();
                        if rect.contains(mouse_event.pos) {
                            // Set active image
                            self.active_image = Some(image.id);
                            image.widget_pod.set_active(true);
                            // Start the drag event
                            self.drag = Some(Drag {
                                start: mouse_event.pos,
                                image_id: image.id,
                            });
                            // Send the event to the image as well
                            image.widget_pod.event(ctx, event, &mut image.data, env);
                            break;
                        }
                    }
                }
            }
            Event::MouseMoved(mouse_event) => {
                if let Some(drag) = &mut self.drag {
                    if let Some(image) = self.images.iter_mut().find(|image| image.id == drag.image_id) {
                        image.adjust_origin(&self.images_area, mouse_event.pos - drag.start);
                        drag.start = mouse_event.pos;
                    }
                }
            }
            Event::MouseUp(mouse_event) => {
                if mouse_event.button.is_left() {
                    if let Some(drag) = &self.drag {
                        let image = &mut self.images[drag.image_id];
                        image.adjust_origin(&self.images_area, mouse_event.pos - drag.start);
                        self.drag = None;
                    }
                }
            }
            Event::KeyUp(key_event) => match key_event.key_code {
                KeyCode::PageUp => {
                    if let Some(active_image) = self.active_image {
                        if let Some(layer) = self.layers.iter().position(|&id| id == active_image) {
                            if layer < self.layers.len() - 1 {
                                self.layers[layer] = self.layers[layer + 1];
                                self.layers[layer + 1] = active_image;
                            }
                        }
                    }
                }
                KeyCode::PageDown => {
                    if let Some(active_image) = self.active_image {
                        if let Some(layer) = self.layers.iter().position(|&id| id == active_image) {
                            if layer > 0 {
                                self.layers[layer] = self.layers[layer - 1];
                                self.layers[layer - 1] = active_image;
                            }
                        }
                    }
                }
                _ => (),
            },
            Event::AnimFrame(_) => {
                // Pass the event to all the images
                for image in self.images.iter_mut() {
                    image.widget_pod.event(ctx, event, &mut image.data, env);
                }
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&u32>, _data: &u32, env: &Env) {
        // Pass the update to all the images
        for image in self.images.iter_mut() {
            image.widget_pod.update(ctx, &image.data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &u32, env: &Env) -> Size {
        bc.debug_check("Surface");

        // Reserve border area for the surface to make sure things work well
        let border_width = match &self.border {
            Some(border) => border.width,
            None => 0.0,
        };
        let images_area = bc.shrink((2.0 * border_width, 2.0 * border_width)).loosen();
        self.images_area = images_area.max();

        // Set the layout for all the images
        for image in self.images.iter_mut() {
            let origin = image.origin + Vec2::new(border_width, border_width);
            let area = images_area.shrink(image.origin.to_vec2().to_size());
            let size = image.widget_pod.layout(ctx, &area, &image.data, env);
            image.widget_pod.set_layout_rect(Rect::from_origin_size(origin, size));
        }

        // The surface always uses the whole area provided to it
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, base_state: &BaseState, _data: &u32, env: &Env) {
        // Clip the overflow
        let size = base_state.size();
        //ctx.render_ctx.clip(Rect::from_origin_size(Point::ZERO, size));

        // Paint border, if there is one
        // TODO: Eventually move this after painting the images to cover up any anti-aliasing overflow
        if let Some(border) = &self.border {
            let offset = border.width / 2.0;
            let rect = Rect::from_origin_size(
                (offset, offset),
                Size::new(size.width - border.width, size.height - border.width),
            );
            ctx.render_ctx.stroke(rect, &border.brush, border.width);
        }

        // Paint all the images
        for &id in self.layers.iter() {
            let image = &mut self.images[id];
            image.widget_pod.paint_with_offset(ctx, &image.data, env);
        }
    }
}
