use crate::framebuffer::{Framebuffer, UpdateMode};
use crate::gesture::GestureEvent;
use crate::input::DeviceEvent;
use crate::view::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue, RenderData, ViewId, Align};
use crate::view::icon::Icon;
use crate::view::clock::Clock;
use crate::view::battery::Battery;
use crate::view::label::Label;
use crate::view::packed_view::{PackedView, Position, VAlign, Pack};
use crate::geom::{Rectangle};
use crate::font::Fonts;
use crate::context::Context;

#[derive(Debug)]
pub struct TopBar {
    id: Id,
    rect: Rectangle,
    views: PackedView,
}

const SEARCH : usize = 0;
const MENU : usize = 1;
const BATTERY : usize = 2;
const LIGHT : usize = 3;
const CLOCK : usize = 4;
const TITLE : usize = 5;

impl TopBar {
    pub fn new(rect: Rectangle, root_event: Event, title: String, hub: &Hub, rq: &mut RenderQueue, context : &mut Context) -> TopBar {
        let id = ID_FEEDER.next();

        let side = rect.height() as i32;
        let icon_name = match root_event {
            Event::Back => "back",
            _ => "search",
        };

        let null_rect = rect!(0, 0, 0, 0);
        let capacity = context.battery.capacity().map_or(0.0, |v| v[0]);
        let status = context.battery.status().map_or(crate::battery::Status::Discharging, |v| v[0]);
        let name = if context.settings.frontlight { "frontlight" } else { "frontlight-disabled" };
        let clock_width = Clock::compute_width(context);

        let views : PackedView = PackedView::new(rect)
            .push(Box::new(Icon::new(icon_name, null_rect, root_event)),
                  Position::squared_top_left(side), hub, rq, context)
            .push(Box::new(Icon::new("menu", null_rect, Event::ToggleNear(ViewId::MainMenu, null_rect))),
                  Position::squared_top_right(side), hub, rq, context)
            .push(Box::new(Battery::new(null_rect, capacity, status)),
                  Position::squared_top_right(side), hub, rq, context)
            .push(Box::new(Icon::new(name, null_rect, Event::Show(ViewId::Frontlight))),
                  Position::squared_top_right(side), hub, rq, context)
            .push(Box::new(Clock::new(null_rect, context)),
                  Position::top_right(clock_width as i32, side), hub, rq, context)
            .push(Box::new(Label::new(null_rect, title, Align::Center)
                           .event(Some(Event::ToggleNear(ViewId::TitleMenu, null_rect)))),
                  Position::filled_top_left(), hub, rq, context);

        TopBar {
            id,
            rect,
            views,
        }
    }

    pub fn update_root_icon(&mut self, name: &str, rq: &mut RenderQueue) {
        let icon = self.child_mut(SEARCH).downcast_mut::<Icon>().unwrap();
        if icon.name != name {
            icon.name = name.to_string();
            rq.add(RenderData::new(icon.id(), *icon.rect(), UpdateMode::Gui));
        }
    }

    pub fn update_title_label(&mut self, title: &str, rq: &mut RenderQueue) {
        let title_label = self.child_mut(TITLE).downcast_mut::<Label>().unwrap();
        title_label.update(title, rq);
    }

    pub fn update_frontlight_icon(&mut self, rq: &mut RenderQueue, context: &mut Context) {
        let name = if context.settings.frontlight { "frontlight" } else { "frontlight-disabled" };
        let icon = self.child_mut(LIGHT).downcast_mut::<Icon>().unwrap();
        icon.name = name.to_string();
        rq.add(RenderData::new(icon.id(), *icon.rect(), UpdateMode::Gui));
    }

    pub fn update_clock_label(&mut self, rq: &mut RenderQueue) {
        if let Some(clock_label) = self.child_mut(CLOCK).downcast_mut::<Clock>() {
            clock_label.update(rq);
        }
    }

    pub fn update_battery_widget(&mut self, rq: &mut RenderQueue, context: &mut Context) {
        if let Some(battery_widget) = self.child_mut(BATTERY).downcast_mut::<Battery>() {
            battery_widget.update(rq, context);
        }
    }

    pub fn reseed(&mut self, rq: &mut RenderQueue, context: &mut Context) {
        self.update_frontlight_icon(rq, context);
        self.update_clock_label(rq);
        self.update_battery_widget(rq, context);
    }
}

impl View for TopBar {
    fn handle_event(&mut self, evt: &Event, _hub: &Hub, _bus: &mut Bus, _rq: &mut RenderQueue, _context: &mut Context) -> bool {
        match *evt {
            Event::Gesture(GestureEvent::Tap(center)) |
            Event::Gesture(GestureEvent::HoldFingerShort(center, ..)) if self.rect.includes(center) => true,
            Event::Gesture(GestureEvent::Swipe { start, end, .. }) if self.rect.includes(start) && self.rect.includes(end) => true,
            Event::Device(DeviceEvent::Finger { position, .. }) if self.rect.includes(position) => true,
            _ => false,
        }
    }

    fn render(&self, _fb: &mut dyn Framebuffer, _rect: Rectangle, _fonts: &mut Fonts) {
    }

    fn resize(&mut self, rect: Rectangle, hub: &Hub, rq: &mut RenderQueue, context: &mut Context) {
        let side = rect.height() as i32;
        let clock_width = Clock::compute_width(context);
        self.views.update_position(SEARCH, Position::squared_top_left(side), hub, rq, context);
        self.views.update_position(MENU, Position::squared_top_right(side), hub, rq, context);
        self.views.update_position(BATTERY, Position::squared_top_right(side), hub, rq, context);
        self.views.update_position(LIGHT, Position::squared_top_right(side), hub, rq, context);
        self.views.update_position(CLOCK, Position::top_right(clock_width as i32, side), hub, rq, context);
        self.views.update_position(TITLE, Position::filled_top_left(), hub, rq, context);

        self.views.resize(rect, hub, rq, context);
        self.rect = rect;
    }

    fn rect(&self) -> &Rectangle {
        &self.rect
    }

    fn rect_mut(&mut self) -> &mut Rectangle {
        &mut self.rect
    }

    fn children(&self) -> &Vec<Box<dyn View>> {
        self.views.children()
    }

    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> {
        self.views.children_mut()
    }

    fn id(&self) -> Id {
        self.id
    }
}
