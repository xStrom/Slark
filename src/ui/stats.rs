// Copyright 2019 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Stats will show FPS based on AnimFrame.

use masonry::{
    accesskit::{Node, Role},
    core::{
        AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, PaintCtx, PropertiesMut, PropertiesRef,
        RegisterCtx, UpdateCtx, Widget, WidgetPod,
    },
    kurbo::{Point, Size},
    vello::Scene,
    widgets::Label,
};

pub struct Stats {
    frame_times: [u64; Stats::FRAME_TIME_COUNT],
    frame_time_index: usize,
    fps: u64,
    initializing: bool,
    label_fps: WidgetPod<Label>,
}

impl Stats {
    const FRAME_TIME_COUNT: usize = 360;

    pub fn new() -> Stats {
        Stats {
            frame_times: [0; Stats::FRAME_TIME_COUNT],
            frame_time_index: 0,
            fps: 0,
            initializing: true,
            label_fps: NewWidget::new(Label::new("FPS: 0")).to_pod(),
        }
    }

    fn add_frame_time(&mut self, frame_time: u64) {
        self.frame_times[self.frame_time_index] = frame_time;
        self.frame_time_index += 1;
        if self.frame_time_index == Stats::FRAME_TIME_COUNT {
            self.initializing = false;
            self.frame_time_index = 0;
        }
    }

    fn average_fps(&self) -> u64 {
        let timed_frame_count = if self.initializing {
            self.frame_time_index
        } else {
            Stats::FRAME_TIME_COUNT
        };
        let total_frame_time: u64 = if self.initializing {
            self.frame_times.iter().take(timed_frame_count).sum()
        } else {
            self.frame_times.iter().sum()
        };
        let avg_frame_time = if timed_frame_count > 0 {
            total_frame_time / timed_frame_count as u64
        } else {
            0
        };
        if avg_frame_time > 0 {
            1_000_000_000 / avg_frame_time
        } else {
            0
        }
    }
}

impl Widget for Stats {
    type Action = ();

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.label_fps);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {
        // TODO: Something?
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_elem(self.label_fps.id(), 1)
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        self.add_frame_time(interval);
        let fps = self.average_fps();
        if self.fps != fps {
            self.fps = fps;
            // TODO: Genenrate a cache map of all possible fps-es?
            //       Could do powers of two dynamically, with also automatic garbage collection.
            //       That way there's no allocation happening because of the counter.
            //       Although, perhaps want to cache even more than the string, like internal Parley results.
            let new_text = format!("FPS: {}", self.fps);
            ctx.mutate_later(&mut self.label_fps, move |mut this| {
                Label::set_text(&mut this, new_text);
            });
        }
        ctx.request_anim_frame();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> Size {
        bc.debug_check("Stats");
        let label_bc = bc.loosen();

        let _label_size = ctx.run_layout(&mut self.label_fps, &label_bc);
        ctx.place_child(&mut self.label_fps, Point::ORIGIN);

        bc.constrain((70.0, 20.0))
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {}
}
