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
use druid::kurbo::{Point, Rect, Size };
use druid::piet::{PaintBrush, RenderContext};

use crate::ui::gif::{Gif};

pub struct Border {
    width: f64,
    brush: PaintBrush,
}

impl Border {
    pub fn new(width: f64, brush: PaintBrush) -> Border {
        Border{width: width, brush: brush}
    }
}

pub struct Surface {
    images: Vec<WidgetPod<u32, Gif>>,
    border: Option<Border>,
}

impl Surface {
    pub fn new() -> Surface {
        Surface{images: Vec::new(), border: None}
    }

    pub fn set_border(&mut self, border: Option<Border>) {
        self.border = border;
    }

    pub fn add(&mut self, filename: &str) {
        self.images.push(WidgetPod::new(Gif::new(filename)));
    }
}

impl Widget<u32> for Surface {
    fn paint(&mut self, ctx: &mut PaintCtx, base_state: &BaseState, data: &u32, env: &Env) {
        // Paint border, if there is one
        if let Some(border) = &self.border {
            let offset = border.width / 2.0;
            let size = Size::new(base_state.size().width - border.width, base_state.size().height - border.width);
            let rect = Rect::from_origin_size((offset, offset), size);
            ctx.render_ctx.stroke(rect, &border.brush, border.width);
        }

        // Paint all the images
        for image in self.images.iter_mut() {
            image.paint_with_offset(ctx, data, env);
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
            // TODO: Give a slightly different default origin to each image
            let origin = Point::new(border_width, border_width);
            let size = image.layout(ctx, &child_bc, data, env);
            image.set_layout_rect(Rect::from_origin_size(origin, size));
        }

        // The surface always uses the whole area provided to it
        bc.max()
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut u32, env: &Env) {
        match event {
            _ => (),
        }
        // Pass the event to all the images
        for image in self.images.iter_mut() {
            image.event(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&u32>, data: &u32, env: &Env) {
        // Pass the update to all the images
        for image in self.images.iter_mut() {
            image.update(ctx, data, env);
        }
    }
}