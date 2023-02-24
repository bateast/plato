use crate::geom::{Rectangle, Point, Vec2};

pub use crate::view::Align;
#[derive(Debug)]
pub enum VAlign {
    Top(i32),
    Bottom(i32),
    Center,
}

#[derive(Debug)]
pub enum Pack {
    /// Object has fixed size
    Fixed(Point),
    /// Object take available place in % of the outer rect
    Percent(Vec2),
    /// fill available space from left top sibling to bottom right one.
    Fill,
}

#[derive(Debug)]
pub struct Position {
    /// Place in outer rect, including inner margin
    pub pack: Pack,
    /// Inner margin
    pub margin: Rectangle,
    /// Horizontal position in outer rect, plus outer margin
    pub align: Align,
    /// Vertical position in outer rect, plus outer margin
    pub valign: VAlign,
}
