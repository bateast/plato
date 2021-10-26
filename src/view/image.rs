use anyhow::Error;
use crate::framebuffer::{Framebuffer, UpdateMode, Pixmap};
use crate::view::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue, RenderData};
use crate::color::{WHITE, BLACK};
use crate::geom::Rectangle;
use crate::app::Context;
use crate::font::Fonts;

pub struct Image {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    pixmap: Pixmap,
    blended: bool,
    blended_color: u8,
}

impl Image {
    pub fn new(rect: Rectangle, pixmap: Pixmap) -> Image {
        Image {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            pixmap,
            blended: false,
            blended_color: BLACK,
        }
    }

    pub fn update(&mut self, pixmap: Pixmap, rq: &mut RenderQueue) {
        self.pixmap = pixmap;
        rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
    }

    pub fn set_blended(&mut self, blended: bool, color: u8) {
        self.blended = blended;
        self.blended_color = color;
    }

    pub fn pixmap(&self) -> &Pixmap {
        &self.pixmap
    }
}

impl View for Image {
    fn handle_event(&mut self, _evt: &Event, _hub: &Hub, _bus: &mut Bus, _rq: &mut RenderQueue, _context: &mut Context) -> bool {
        false
    }

    fn render(&self, fb: &mut dyn Framebuffer, rect: Rectangle, _fonts: &mut Fonts) {
        let x0 = if self.rect.width() > self.pixmap.width {
            self.rect.min.x + (self.rect.width() - self.pixmap.width) as i32 / 2
        } else {self.rect.min.x as i32 / 2 };
        let y0 = if self.rect.height() > self.pixmap.height {
            self.rect.min.y + (self.rect.height() - self.pixmap.height) as i32 / 2
        } else {self.rect.min.y as i32 / 2};
        let x1 = x0 + self.pixmap.width as i32;
        let y1 = y0 + self.pixmap.height as i32;
        if ! self.blended {
            if let Some(r) = rect![self.rect.min, pt!(x1, y0)].intersection(&rect) {
                fb.draw_rectangle(&r, WHITE);
            }
            if let Some(r) = rect![self.rect.min.x, y0, x0, self.rect.max.y].intersection(&rect) {
                fb.draw_rectangle(&r, WHITE);
            }
            if let Some(r) = rect![pt!(x0, y1), self.rect.max].intersection(&rect) {
                fb.draw_rectangle(&r, WHITE);
            }
            if let Some(r) = rect![x1, self.rect.min.y, self.rect.max.x, y1].intersection(&rect) {
                fb.draw_rectangle(&r, WHITE);
            }
        }
        if let Some(r) = rect![x0, y0, x1, y1].intersection(&rect) {
            let frame = r - pt!(x0, y0);
            if ! self.blended {
                fb.draw_framed_pixmap(&self.pixmap, &frame, r.min);
            } else {
                fb.draw_framed_pixmap_blended(&self.pixmap, &frame, r.min, self.blended_color);
            }
        }
    }

    fn render_rect(&self, rect: &Rectangle) -> Rectangle {
        rect.intersection(&self.rect)
            .unwrap_or(self.rect)
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

    fn id(&self) -> Id {
        self.id
    }
}

impl Framebuffer for Image {
    fn set_pixel(&mut self, x: u32, y: u32, color: u8) {
        self.pixmap.set_pixel(x, y, color);
    }
    fn set_blended_pixel(&mut self, x: u32, y: u32, color: u8, alpha: f32){
        self.pixmap.set_blended_pixel(x, y, color, alpha);
    }
    fn invert_region(&mut self, rect: &Rectangle){
        self.pixmap.invert_region(rect);
    }
    fn shift_region(&mut self, rect: &Rectangle, drift: u8){
        self.pixmap.shift_region(rect, drift);
    }
    fn update(&mut self, rect: &Rectangle, mode: UpdateMode) -> Result<u32, Error>{
        self.pixmap.update(rect, mode)
    }
    fn wait(&self, token: u32) -> Result<i32, Error>{
        self.pixmap.wait(token)
    }
    fn save(&self, path: &str) -> Result<(), Error>{
        self.pixmap.save(path)
    }
    fn set_rotation(&mut self, n: i8) -> Result<(u32, u32), Error>{
        self.pixmap.set_rotation(n)
    }
    fn set_monochrome(&mut self, enable: bool){
        self.pixmap.set_monochrome(enable);
    }
    fn set_dithered(&mut self, enable: bool){
        self.pixmap.set_dithered(enable);
    }
    fn set_inverted(&mut self, enable: bool){
        self.pixmap.set_inverted(enable)
    }
    fn monochrome(&self) -> bool{
        self.pixmap.monochrome()
    }
    fn dithered(&self) -> bool{
        self.pixmap.dithered()
    }
    fn inverted(&self) -> bool{
        self.pixmap.inverted()
    }

    fn width(&self) -> u32 {
        self.pixmap.width()
    }

    fn height(&self) -> u32 {
        self.pixmap.height()
    }

    fn dims(&self) -> (u32, u32) {
        self.pixmap.dims()
    }

}
