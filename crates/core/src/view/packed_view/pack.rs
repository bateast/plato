use crate::geom::{Rectangle, Point, Vec2};


const NULL_RECT : Rectangle = Rectangle {
    min : Point {
        x: 0,
        y: 0,
    },
    max : Point {
        x: 0,
        y: 0,
    },
};

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

impl Position {
    pub fn squared_top_left(a : i32) -> Self {
        Position{
            pack: Pack::Fixed(pt!(a, a)),
            margin: NULL_RECT,
            align: Align::Left(0),
            valign: VAlign::Top(0),
        }
    }
    pub fn squared_top_right(a : i32) -> Self {
        Position{
            pack: Pack::Fixed(pt!(a, a)),
            margin: NULL_RECT,
            align: Align::Right(0),
            valign: VAlign::Top(0),
        }
    }
    pub fn top_left(x : i32, y: i32) -> Self {
        Position{
            pack: Pack::Fixed(pt!(x, y)),
            margin: NULL_RECT,
            align: Align::Left(0),
            valign: VAlign::Top(0),
        }
    }
    pub fn top_right(x : i32, y: i32) -> Self {
        Position{
            pack: Pack::Fixed(pt!(x, y)),
            margin: NULL_RECT,
            align: Align::Right(0),
            valign: VAlign::Top(0),
        }
    }
    pub fn filled_top_left() -> Self {
        Position {
            pack: Pack::Fill,
            margin: NULL_RECT,
            align: Align::Left(0),
            valign: VAlign::Top(0)
        }
    }
}
