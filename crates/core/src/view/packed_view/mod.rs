use crate::view::*;
use crate::geom::{Rectangle, Point};

pub mod pack;
pub use pack::*;

use std::vec::Vec;
use log::{debug, info, warn};

#[derive(Debug)]
pub struct PackedView {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    positions: Vec<Position>
}

impl PackedView {
    pub fn new(rect: Rectangle) -> Self {
        info!("Create new PackedView for rectangle {rect}");

        PackedView {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            positions: Vec::new(),
        }
    }

    pub fn push(mut self, view: Box<dyn View>, position: Position, hub: &Hub, rq: &mut RenderQueue, context: &mut Context) -> Self {
        debug!("Push new children of {}: {} with position {:?}", self.id(), view.id(), position);

        self.children.push(view);
        self.positions.push(position);
        self.resize(self.rect, hub, rq, context);
        self
    }

    fn compute_sizes(&self) -> Vec<Rectangle> {
        let mut sizes = Vec::new();
        let full_size = pt!(self.rect.width() as i32, self.rect.height() as i32);

        let mut availabilities = Vec::new();
        availabilities.push(self.rect);

        for (index, Position{pack, margin, align, valign}) in self.positions.iter().enumerate() {
            if availabilities.is_empty() {
                debug!("** No more space available **");
                break;
            }
            debug!("Packed {} — computing size in {:?}", self.id(), availabilities);
            debug!("Packed {} — ||- placing {:?} (margin {:?}, align {:?}, valign {:?})", self.id(), pack, margin, align, valign);
            
            let outer_h_margin = match align {
                Align::Left(h) | Align::Right(h) => *h,
                _ => 0,
            };
            let outer_v_margin = match valign {
                VAlign::Bottom(v) | VAlign::Top(v) => *v,
                _ => 0,
            };
            let outter_margin = rect!(pt!(outer_h_margin, outer_v_margin), pt!(- outer_h_margin, - outer_v_margin));
            let first = availabilities.first().unwrap();
            let mut largest = (0, *first);
            let mut highest = (0, *first);
            let mut leftmost = (0, *first);
            let mut rightmost = (0, *first);
            let mut topmost = (0, *first);
            let mut bottommost = (0, *first);
            for (index, rect) in availabilities.iter().enumerate() {
                if largest.1.width() + 2 * outer_h_margin.try_into().unwrap_or(0) < rect.width() {
                    largest.0 = index;
                    largest.1 = *rect + outter_margin;
                }
                if highest.1.height() + 2 * outer_v_margin.try_into().unwrap_or(0) < rect.height() {
                    highest.0 = index;
                    highest.1 = *rect + outter_margin;
                }
                if leftmost.1.min.x < rect.min.x + outer_h_margin {
                    leftmost.0 = index;
                    leftmost.1 = *rect + outter_margin;
                }
                if rightmost.1.max.x < rect.max.x - outer_h_margin {
                    rightmost.0 = index;
                    rightmost.1 = *rect + outter_margin
                }
                if topmost.1.min.y < rect.min.y + outer_v_margin {
                    topmost.0 = index;
                    topmost.1 = *rect + outter_margin;
                }
                if bottommost.1.max.y < rect.max.y - outer_v_margin {
                    bottommost.0 = index;
                    bottommost.1 = *rect + outter_margin;
                }
            }

            let mut rect_into = match align {
                Align::Left(_) => (leftmost.0, leftmost.1),
                Align::Right(_) => (rightmost.0, rightmost.1),
                Align::Center => largest,
            };
            if let Pack::Fixed(size) = pack {
                if size.lt(pt!(rect_into.1.width() as i32, rect_into.1.height() as i32)) {
                    rect_into = largest;
                }
            }

            let size = match pack {
                Pack::Fixed(size) if size.le(pt!(rect_into.1.width() as i32, rect_into.1.height() as i32)) => {
                    *size
                },
                Pack::Fixed(size) => {
                    let limited_pt = size.min(pt!(rect_into.1.width() as i32, rect_into.1.height() as i32));
                    warn!("Required space ({:?}) unavailable for packed children {index} of {}. \\
                           Limiting to {:?}", size, limited_pt, self.id());
                    limited_pt
                },
                Pack::Percent(pc) => Point::from(full_size * *pc),
                Pack::Fill => pt!(rect_into.1.width() as i32, rect_into.1.height() as i32),
            };

            let min_x = match align {
                Align::Left(_) => rect_into.1.min.x,
                Align::Right(_) => rect_into.1.max.x - size.x,
                Align::Center => rect_into.1.min.x + rect_into.1.width() as i32 / 2 - size.x / 2,
            };
            let min_y = match valign {
                VAlign::Top(_) => rect_into.1.min.y,
                VAlign::Bottom(_) => rect_into.1.max.y - size.y,
                VAlign::Center => rect_into.1.min.y + rect_into.1.height() as i32 / 2 - size.y / 2,
            };

            // Finally compute children rect
            let rect = rect!(pt!(min_x, min_y), pt!(min_x, min_y) + size) - *margin;

            // Update availabilities
            let mut cutted_availability = Vec::new();
            let original_availability = availabilities[rect_into.0];
            match align {
                Align::Left(_) => cutted_availability.push(original_availability + rect!(pt!(size.x + 2 * outer_h_margin, 0), pt!(0, 0))),
                Align::Right(_) => cutted_availability.push(original_availability + rect!(pt!(0, 0), pt!(- (size.x + 2 * outer_h_margin), 0))),
                Align::Center => {
                    cutted_availability.push(rect!(original_availability.min, pt!(rect.min.x, original_availability.max.y)));
                    cutted_availability.push(rect!(pt!(rect.max.x, original_availability.min.y), original_availability.max));
                },
            }
            match valign {
                VAlign::Top(_) => cutted_availability.push(original_availability + rect!(pt!(0, size.y + 2 * outer_v_margin), pt!(0, 0))),
                VAlign::Bottom(_) => cutted_availability.push(original_availability + rect!(pt!(0, 0), pt!(0, - (size.y + 2 * outer_v_margin)))),
                VAlign::Center => {
                    cutted_availability.push(rect!(pt!(rect.min.x, original_availability.min.y), pt!(rect.max.x, rect.min.y)));
                    cutted_availability.push(rect!(pt!(rect.min.x, rect.max.y), pt!(rect.max.x, original_availability.max.y)));
                },
            };

            availabilities.remove(rect_into.0);
            for rect in cutted_availability {
                if 0 < rect.width() && 0 < rect.height() {
                    availabilities.push(rect);
                }
            }


            debug!("Packed {} — || ** found {:?}", self.id(), rect);
            // push computed size
            sizes.push(rect);
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
