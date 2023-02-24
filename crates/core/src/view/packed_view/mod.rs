use crate::view::*;
use crate::geom::{Rectangle, Point};

pub mod pack;
pub use pack::*;

use std::vec::Vec;
use log::{debug, info, warn};

pub struct PackedView {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    positions: Vec<Position>
}

impl PackedView {
    pub fn new(rect: Rectangle) -> Self {
        PackedView {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            positions: Vec::new(),
        }
    }

    pub fn push(&mut self, view: Box<dyn View>, position: Position, hub: &Hub, rq: &mut RenderQueue, context: &mut Context) -> &mut Self{
        debug!("Push new children of {}: {} with position {:?}", self.id(), view.id(), position);

        self.children.push(view);
        self.positions.push(position);
        self.resize(self.rect, hub, rq, context);
        self
    }

    fn compute_sizes(&self) -> Vec<Rectangle> {
        let mut sizes = Vec::new();

        let full_size = pt!(self.rect.width() as i32, self.rect.height() as i32);
        let mut availabilities = rect!(pt!(0, 0), pt!(self.rect.width() as i32, self.rect.height() as i32));

        for (index, Position{pack, margin, align, valign}) in self.positions.iter().enumerate() {
            debug!("Packed {} — computing size in {:?}", self.id(), availabilities);
            let outer_h_margin = match align {
                Align::Left(h) | Align::Right(h) => *h,
                _ => 0,
            };
            let outer_v_margin = match valign {
                VAlign::Bottom(v) | VAlign::Top(v) => *v,
                _ => 0,
            };
            let outter_margin = pt!(outer_h_margin, outer_v_margin);
            let max_size_available = pt!(availabilities.width() as i32, availabilities.height() as i32)
                - outter_margin;

            let size = match pack {
                Pack::Fixed(size) if size.le(max_size_available) => {
                    *size
                },
                Pack::Fixed(size) => {
                    let limited_pt = size.min(max_size_available);
                    warn!("Required space ({:?}) unavailable for packed children {index} of {}. \\
                           Limiting to {:?}", size, limited_pt, self.id());
                    limited_pt
                },
                Pack::Percent(pc) => Point::from(full_size * *pc),
                Pack::Fill => max_size_available,
            };
            let min_x = match align {
                Align::Left(_) => availabilities.min.x + outter_margin.x,
                Align::Right(_) => availabilities.max.x - outter_margin.x - size.x,
                Align::Center => availabilities.min.x + max_size_available.x / 2 - size.x / 2,
            };
            let min_y = match valign {
                VAlign::Top(_) => availabilities.min.y + outter_margin.y,
                VAlign::Bottom(_) => availabilities.max.y - outter_margin.y - size.y,
                VAlign::Center => availabilities.min.y + max_size_available.y / 2 - size.y / 2,
            };

            // Finally compute children rect
            let rect = rect!(pt!(min_x, min_y), pt!(min_x, min_y) + size) - *margin;

            // Update available size
            match align {
                Align::Left(_) => availabilities.min.x += outter_margin.x + size.x,
                Align::Right(_) => availabilities.max.x -= outter_margin.x + size.x,
                // When align center, we only keep track of rightward space
                Align::Center => availabilities.min.x += (max_size_available.x / 2 - size.x / 2 ) + outter_margin.x + size.x,
            }
            match valign {
                VAlign::Top(_) => availabilities.min.y += outter_margin.y + size.y,
                VAlign::Bottom(_) => availabilities.max.y -= outter_margin.y + size.y,
                // When align center, we only keep track of rightward space
                VAlign::Center => availabilities.min.y += (max_size_available.y / 2 - size.y / 2 ) + outter_margin.y + size.y,
            }
            availabilities.min = availabilities.min.min(availabilities.max);

            debug!("Packed {} — found {:?}, new availabilities {:?}", self.id(), rect, availabilities);
            // push computed size
            sizes.push(rect)
        }

        sizes
    }
}

impl View for PackedView {
    fn handle_event(&mut self, _evt: &Event, _hub: &Hub, _bus: &mut Bus, _rq: &mut RenderQueue, _context: &mut Context) -> bool
    {
        false
    }

    fn render(&self, fb: &mut dyn Framebuffer, rect: Rectangle, fonts: &mut Fonts) {
        todo!()
    }
    fn id(&self) -> Id {
        self.id
    }

    fn resize(&mut self, rect: Rectangle, hub: &Hub, rq: &mut RenderQueue, context: &mut Context) {
        debug!("Resizing packed {} from {} to {}", self.id(), self.rect, rect);
        self.rect = rect;

        let sizes = self.compute_sizes();
        for (index, size) in sizes.iter().enumerate() {
            self.children[index].resize(*size, hub, rq, context);
        }
    }

    fn rect(&self) -> &Rectangle {
        &self.rect
    }
    fn rect_mut(&mut self) -> &mut Rectangle {
        &mut self.rect
    }
    fn children(&self) -> &Vec<Box<dyn View>> {
        &self.children
    }
    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> {
        &mut self.children
    }
}
