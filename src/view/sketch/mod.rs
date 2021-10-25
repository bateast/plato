use std::fs::{self, File};
use std::path::PathBuf;
use rand_core::RngCore;
use fxhash::FxHashMap;
use chrono::Local;
use walkdir::WalkDir;
use globset::Glob;
use anyhow::Error;
use crate::device::CURRENT_DEVICE;
use crate::geom::{Point, Rectangle, CornerSpec};
use crate::input::{DeviceEvent, FingerStatus};
use crate::view::icon::{Icon, ICONS_PIXMAPS};
use crate::view::notification::Notification;
use crate::view::image::Image;
use crate::view::menu::{Menu, MenuKind};
use crate::view::common::{locate_by_id, locate};
use crate::view::{View, Event, Hub, Bus, RenderQueue, RenderData};
use crate::view::{EntryKind, EntryId, ViewId, Id, ID_FEEDER};
use crate::view::{SMALL_BAR_HEIGHT, BORDER_RADIUS_SMALL};
use crate::framebuffer::{Framebuffer, UpdateMode, Pixmap};
use crate::settings::{ImportSettings, Pen, MyscriptSettings};
use crate::helpers::IsHidden;
use crate::font::Fonts;
use crate::unit::scale_by_dpi;
use crate::color::{BLACK, WHITE};
use crate::app::Context;
use crate::document;
use document::{Document, Location};

mod myscript;

// TODO:
// * svg

const FILENAME_PATTERN: &str = "sketch-%Y%m%d_%H%M%S.png";
const ICON_NAME: &str = "enclosed_menu";
const ICON_PEN: &str = "pen";

// https://oeis.org/A000041
const PEN_SIZES: [i32; 12] = [1, 2, 3, 5, 7, 11, 15, 22, 30, 42, 56, 77];

#[derive(Clone, Copy)]
pub struct TouchState {
    pt: Point,
    time: f64,
    radius: f32,
}

impl TouchState {
    fn new(pt: Point, time: f64, radius: f32) -> TouchState {
        TouchState { pt, time, radius }
    }
}

#[derive(PartialEq)]
pub enum SketchMode {
    OneFinger,
    Fast,
    Full,
}

fn load(filename: &PathBuf) -> Option<Pixmap> {
    let mut opt_doc = document::open(filename);
    if let Some(boxed_doc) = &mut opt_doc {
        let opt_pixmap = boxed_doc.pixmap(Location::Exact(0),1.);
        if let Some((pixmap, _)) = opt_pixmap {
            Some(pixmap)
        }
        else {
            None
        }
    } else {
        None
    }

}

fn list(path: PathBuf) -> Vec<(PathBuf, PathBuf)> {
    WalkDir::new(path)
        .sort_by_file_name()
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok().filter(|e| e.file_type().is_file())
                    .and_then(|e| if let Some(f) = e.path().file_name()
                              {Some((PathBuf::from(f), PathBuf::from(e.path())))}
                              else {None}))
        .collect()
}

struct Background {
    rect: Rectangle,
    image: Image,
    drawing : bool,
}

impl Background {

    pub fn new(rect: Rectangle) -> Background {
        let mut pixmap = Pixmap::new(rect.width(), rect.height());
        pixmap.clear(WHITE);
        Background {
            rect,
            image: Image::new(rect, pixmap),
            drawing : false,
        }
    }

    pub fn load(&mut self, filename: &PathBuf, rq: &mut RenderQueue) -> Result<(), Error> {
        let mut pixmap = Pixmap::new(self.rect.width(), self.rect.height());
        pixmap.clear(WHITE);
        if let Some(new_pixmap) = load(filename) {
            pixmap.draw_pixmap(&new_pixmap, self.rect.min);
            self.image.update(pixmap, rq);
        }
        Ok(())
    }

    pub fn set_drawing(&mut self, drawing : bool) {
        self.drawing = drawing;
    }

    fn view_id(&self) -> Option<ViewId> {
        Some(ViewId::SketchBackground)
    }
}

impl View for Background {

    fn render(&self, fb: &mut dyn Framebuffer, rect: Rectangle, fonts: &mut Fonts) {
        if ! self.drawing {
            self.image.render(fb, rect, fonts);
        }
    }

    fn handle_event(&mut self, evt: &Event, hub: &Hub, bus: &mut Bus, rq: &mut RenderQueue, context: &mut Context) -> bool {
        false
    }
    fn rect(&self) -> &Rectangle {
        View::rect(&self.image)
    }
    fn rect_mut(&mut self) -> &mut Rectangle{
        self.image.rect_mut()
    }

    fn children(&self) -> &Vec<Box<dyn View>> {
        self.image.children()
    }
    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> {
        self.image.children_mut()
    }
    fn id(&self) -> Id {
        self.image.id()
    }
    fn might_skip(&self, _evt: &Event) -> bool {
        true
    }
    fn might_rotate(&self) -> bool {
        false
    }
    fn is_background(&self) -> bool {
        true
    }
}

pub struct Sketch {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    random: Pixmap,
    image:Image,
    mode: SketchMode,
    fingers: FxHashMap<i32, Vec<TouchState>>,
    one_finger: Vec<TouchState>,
    one_finger_id: i32,
    drawing: bool,
    pen: Pen,
    recorded_segments: Vec<Vec<TouchState>>,
    myscript: MyscriptSettings,
    save_path: PathBuf,
    filename: String,
}

impl Sketch {
    pub fn new(rect: Rectangle, rq: &mut RenderQueue, context: &mut Context) -> Sketch {
        let id = ID_FEEDER.next();
        let mut children = Vec::new();
        children.push(Box::new(Background::new(rect)) as Box<dyn View>);
        let dpi = CURRENT_DEVICE.dpi;
        let small_height = scale_by_dpi(SMALL_BAR_HEIGHT, dpi) as i32;
        let border_radius = scale_by_dpi(BORDER_RADIUS_SMALL, dpi) as i32;
        let pixmap = &ICONS_PIXMAPS[ICON_NAME];
        let image = Image::new(rect, Pixmap::new(rect.width(), rect.height()));
        children.push(Box::new(image));
        let icon_padding = (small_height - pixmap.width.max(pixmap.height) as i32) / 2;
        let width = pixmap.width as i32 + icon_padding;
        let height = pixmap.height as i32 + icon_padding;
        let dx = (small_height - width) / 2;
        let dy = (small_height - height) / 2;
        let icon_rect = rect![rect.min.x + dx, rect.max.y - dy - height,
                              rect.min.x + dx + width, rect.max.y - dy];
        let icon = Icon::new(ICON_NAME,
                             icon_rect,
                             Event::ToggleNear(ViewId::TitleMenu, icon_rect))
                        .corners(Some(CornerSpec::Uniform(border_radius)));
        children.push(Box::new(icon) as Box<dyn View>);
        let icon_rect = rect![rect.min.x + width + 2 * dx, rect.max.y - dy - height,
                              rect.min.x + 2 * width + 2 * dx, rect.max.y - dy];
        let icon = Icon::new(ICON_PEN,
                             icon_rect,
                             Event::ToggleNear(ViewId::TitleMenu, icon_rect))
            .corners(Some(CornerSpec::Uniform(border_radius)));
        children.push(Box::new(icon) as Box<dyn View>);
        let save_path = context.library.home.join(&context.settings.sketch.save_path);
        rq.add(RenderData::new(id, rect, UpdateMode::Full));
        let mut random = Pixmap::new(rect.width(), rect.height());
        context.rng.fill_bytes(random.data_mut());
        Sketch {
            id,
            rect,
            children,
            random,
            image:Image::new(rect, Pixmap::new(0,0)),
            mode: SketchMode::OneFinger,
            fingers: FxHashMap::default(),
            one_finger: Vec::new(),
            one_finger_id : -1,
            drawing: false,
            pen: context.settings.sketch.pen.clone(),
            myscript: context.settings.myscript.clone(),
            recorded_segments: Vec::new(),
            save_path,
            filename: Local::now().format(FILENAME_PATTERN).to_string(),
        }
    }

    fn toggle_title_menu(&mut self, rect: Rectangle, enable: Option<bool>, rq: &mut RenderQueue, context: &mut Context) {
        if let Some(index) = locate_by_id(self, ViewId::SketchMenu) {
            if let Some(true) = enable {
                return;
            }

            rq.add(RenderData::expose(*self.child(index).rect(), UpdateMode::Gui));
            self.children.remove(index);
        } else {
            if let Some(false) = enable {
                return;
            }

            let glob = Glob::new("**/*.png").unwrap().compile_matcher();
            let mut sizes = vec![
                EntryKind::CheckBox("Dynamic".to_string(),
                                    EntryId::TogglePenDynamism,
                                    self.pen.dynamic),
                EntryKind::Separator,
            ];

            for s in PEN_SIZES.iter() {
                sizes.push(EntryKind::RadioButton(s.to_string(),
                                                  EntryId::SetPenSize(*s),
                                                  self.pen.size == *s));
            }

            let mut colors = vec![
                EntryKind::RadioButton("White".to_string(),
                                       EntryId::SetPenColor(WHITE),
                                       self.pen.color == WHITE),
                EntryKind::RadioButton("Black".to_string(),
                                       EntryId::SetPenColor(BLACK),
                                       self.pen.color == BLACK),
            ];

            for i in 1..=14 {
                let c = i * 17;
                if i % 7 == 1 {
                    colors.push(EntryKind::Separator);
                }
                colors.push(EntryKind::RadioButton(format!("Gray {:02}", i),
                                                   EntryId::SetPenColor(c),
                                                   self.pen.color == c));
            }

            let mut entries = vec![
                EntryKind::SubMenu("Size".to_string(), sizes),
                EntryKind::SubMenu("Color".to_string(), colors),
                EntryKind::Separator,
                EntryKind::Command("Save".to_string(), EntryId::Save),
                EntryKind::Command("Refresh".to_string(), EntryId::Refresh),
                EntryKind::Command("New".to_string(), EntryId::New),
                EntryKind::Command("Quit".to_string(), EntryId::Quit),
            ];

            let loadables = list(context.library.home.join(&context.settings.sketch.save_path));
            if !loadables.is_empty() {
                let mut loads_menu = Vec::new();
                loadables.into_iter().for_each(|(f,e)|
                                               loads_menu.push(
                                                   EntryKind::Command(f.to_string_lossy().into_owned(),
                                                                      EntryId::Load(e))));
                entries.insert(entries.len() -1,
                               EntryKind::SubMenu("Load".to_string(), loads_menu));
            }

            let mut backgrounds_menu = vec!(EntryKind::Command("☒ Clear".to_string(), EntryId::ClearBackground));
            backgrounds_menu.push(EntryKind::Separator);
            list(context.library.home.join(&context.settings.sketch.background_path))
                .into_iter().for_each(|(f,e)|
                                      backgrounds_menu.push(
                                          EntryKind::Command(f.to_string_lossy().into_owned(),
                                                             EntryId::LoadBackground(e))));
            entries.insert(entries.len() - 1,
                           EntryKind::SubMenu("Load Background".to_string(), backgrounds_menu));


            let sketch_menu = Menu::new(rect, ViewId::SketchMenu, MenuKind::Contextual, entries, context);
            rq.add(RenderData::new(sketch_menu.id(), *sketch_menu.rect(), UpdateMode::Gui));
            self.children.push(Box::new(sketch_menu) as Box<dyn View>);
        }
    }

    fn load(&mut self, filename: &PathBuf) -> Result<(), Error> {
        if let Some(index) = dbg!(locate::<Image>(self)) {
            if let Some(image) = self.children[dbg!(index)].downcast_mut::<Image>() {
                image.clear(dbg!(WHITE));
                if let Some(pixmap) = load(dbg!(filename)) {
                    //                    image.draw_pixmap(&pixmap, dbg!(self.rect.min));
                }
            }
        }
        Ok(())
    }

    fn save(&self) -> Result<(), Error> {
        if !self.save_path.exists() {
            fs::create_dir_all(&self.save_path)?;
        }
        let path = self.save_path.join(&self.filename);
        if let Some(index) = locate::<Image>(self) {
            if let Some(image) = self.children[index].downcast_ref::<Image>() {
                image.save(&path.to_string_lossy().into_owned())?;
            }
        }
        Ok(())
    }

    fn quit(&self, context: &mut Context) {
        let import_settings = ImportSettings {
            allowed_kinds: ["png".to_string()].iter().cloned().collect(),
            .. Default::default()
        };
        context.library.import(&import_settings);
    }
}

#[inline]
fn draw_segment(image: &mut Image, ts: TouchState, position: Point, time: f64, pen: &Pen, id: Id, fb_rect: &Rectangle, rq: &mut RenderQueue) {
    let (start_radius, end_radius) = if pen.dynamic {
        if time > ts.time {
            let d = vec2!((position.x - ts.pt.x) as f32,
                          (position.y - ts.pt.y) as f32).length();
            let speed = d / (time - ts.time) as f32;
            let base_radius = pen.size as f32 / 2.0;
            let radius = base_radius + (1.0 + base_radius.sqrt()) * speed.clamp(pen.min_speed, pen.max_speed) / (pen.max_speed - pen.min_speed);
            (ts.radius, radius)
        } else {
            (ts.radius, ts.radius)
        }
    } else {
        let radius = pen.size as f32 / 2.0;
        (radius, radius)
    };

    let rect = Rectangle::from_segment(ts.pt, position,
                                       start_radius.ceil() as i32,
                                       end_radius.ceil() as i32);

    image.draw_segment(ts.pt, position, start_radius, end_radius, pen.color);
    if let Some(render_rect) = rect.intersection(fb_rect) {
        rq.add(RenderData::no_wait(id, render_rect, UpdateMode::Fast));
    }
}

#[inline]
fn draw_fast_segment(image: &mut Image, ts: TouchState, position: Point, pen: &Pen, id: Id, fb_rect: &Rectangle, rq: &mut RenderQueue) {

    image.draw_segment(ts.pt, position, 0.5, 0.5, pen.color);

    let rect = Rectangle::from_segment(ts.pt, position, 1, 1);
    if let Some(render_rect) = rect.intersection(fb_rect) {
        rq.add(RenderData::no_wait(id, render_rect, UpdateMode::FastMono));
    }
}

impl View for Sketch {
    fn handle_event(&mut self, evt: &Event, hub: &Hub, _bus: &mut Bus, rq: &mut RenderQueue, context: &mut Context) -> bool {
        match *evt {
            Event::Device(DeviceEvent::Finger { status: FingerStatus::Motion, id, position, time }) => {
                let corrected_position = position + Point{x: self.pen.offset_x, y: self.pen.offset_y};
                if self.drawing
                {
                    if let Some(ts) =
                        match self.mode {
                            SketchMode::OneFinger if id == self.one_finger_id => Some(&mut self.one_finger),
                            SketchMode::OneFinger => None,
                            _ => self.fingers.get_mut(&id),
                        }
                    {
                        if let Some(last) = ts.last() {
                            let last = *last;
                            let radius = self.pen.size as f32 / 2.0;
                            ts.push(TouchState::new(corrected_position, time, radius));
                            if let Some(index) = locate::<Image>(self) {
                                if let Some(image) = &mut self.children[index].downcast_mut::<Image>() {
                                    match self.mode {
                                        SketchMode::OneFinger | SketchMode::Fast =>
                                            draw_fast_segment(image, last, corrected_position, &self.pen, self.id, &self.rect, rq),
                                        SketchMode::Full =>
                                            draw_segment(image, last, corrected_position, time, &self.pen, self.id, &self.rect, rq),
                                    }
                                }
                            }
                        }
                    }
                }
                true
            },
            Event::Device(DeviceEvent::Finger { status: FingerStatus::Down, id, position, time }) => {
                let corrected_position = position + Point{x: self.pen.offset_x, y: self.pen.offset_y};
                let radius = self.pen.size as f32 / 2.0;
                match self.mode {
                    SketchMode::OneFinger if self.drawing => {},
                    SketchMode::OneFinger => {
                        self.one_finger = vec![TouchState::new(corrected_position, time, radius)];
                        self.one_finger_id = id;
                    },
                    _ => {
                        self.fingers.insert(id, vec![TouchState::new(corrected_position, time, radius)]);
                    },
                };
                self.drawing = true;
                true
            },
            Event::Device(DeviceEvent::Finger { status: FingerStatus::Up, id, position, time }) => {
                let corrected_position = position + Point{x: self.pen.offset_x, y: self.pen.offset_y};
                if let Some(ts) = match self.mode {
                    SketchMode::OneFinger if id == self.one_finger_id => Some(&mut self.one_finger),
                    SketchMode::OneFinger => None,
                    _ => self.fingers.get_mut(&id),
                }
                {
                    let mut record = ts.clone();
                    record.push (TouchState::new(corrected_position, time, 0.));
                    self.recorded_segments.push(record);

                    let (mut current_position, mut current_time) = (corrected_position, time);
                    let mut last_element = ts.pop();
                    // if let Some(index) = locate::<Image>(self) {
                    //     if let Some(image) = &mut self.children[index].downcast_mut::<Image>() {
                            while let Some(last) = last_element {
                                // draw_segment(image, last, current_position, current_time, &self.pen, self.id, &self.rect, rq);

                                current_position = last.pt;
                                current_time = last.time;
                                last_element = ts.pop();
                            }
                    //     }
                    // }
                }
                self.drawing = match self.mode {
                    SketchMode::OneFinger if id == self.one_finger_id => false,
                    SketchMode::OneFinger => self.drawing,
                    _ => { self.fingers.remove(&id); self.fingers.is_empty() }
                };
                // if let Ok(json) = self.to_json() {
                //     println! ("JSON {}", &json);
                //     println! ("Auth {}", myscript::compute_hmac(&self.myscript.application_key, &self.myscript.hmac_key, json));
                // }

                true
            },
            Event::ToggleNear(ViewId::TitleMenu, rect) => {
                self.toggle_title_menu(rect, None, rq, context);
                true
            },
            Event::Select(EntryId::SetPenSize(size)) => {
                self.pen.size = size;
                true
            },
            Event::Select(EntryId::SetPenColor(color)) => {
                self.pen.color = color;
                true
            },
            Event::Select(EntryId::TogglePenDynamism) => {
                self.pen.dynamic = !self.pen.dynamic;
                true
            },
            Event::Select(EntryId::Load(ref name)) => {
                if let Err(e) = self.load(name) {
                    let msg = format!("Couldn't load sketch: {}).", e);
                    let notif = Notification::new(msg, hub, rq, context);
                    self.children.push(Box::new(notif) as Box<dyn View>);
                } else {
                    rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                }
                true
            },
            Event::Select(EntryId::LoadBackground(ref name)) => {
                if let Some(index) = locate_by_id(self, ViewId::SketchBackground) {
                    self.children.remove(index);
                }
                let mut boxed_bg = Box::new(Background::new(self.rect));
                if let Err(e) = boxed_bg.load(name, rq) {
                    let msg = format!("Couldn't background sketch: {}).", e);
                    let notif = Notification::new(msg, hub, rq, context);
                    self.children.push(Box::new(notif) as Box<dyn View>);
                } else {
                    rq.add(RenderData::new(boxed_bg.id(), *boxed_bg.rect(), UpdateMode::Gui));
                    self.children.push(boxed_bg);
                }
                true
            }
            ,
            Event::Select(EntryId::ClearBackground) => {
                if let Some(index) = locate_by_id(self, ViewId::SketchBackground) {
                    rq.add(RenderData::expose(*self.child(index).rect(), UpdateMode::Gui));
                    self.children.remove(index);
                }
                true
            },
            Event::Select(EntryId::Refresh) => {
                rq.add(RenderData::new(self.id, self.rect, UpdateMode::Full));
                true
            },
            Event::Select(EntryId::New) => {
                if let Some(index) = locate::<Image>(self) {
                    if let Some(image) = self.children[index].downcast_mut::<Image>() {
                        image.clear(WHITE);
                    }
                }
                self.filename = Local::now().format(FILENAME_PATTERN).to_string();
                rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                true
            },
            Event::Select(EntryId::Save) => {
                let mut msg = match self.save() {
                    Err(e) => Some(format!("Can't save sketch: {}.", e)),
                    Ok(..) => {
                        if context.settings.sketch.notify_success {
                            Some(format!("Saved {}.", self.filename))
                        } else {
                            None
                        }
                    },
                };
                if let Some(msg) = msg.take() {
                    let notif = Notification::new(msg, hub, rq, context);
                    self.children.push(Box::new(notif) as Box<dyn View>);
                }
                true
            },
            Event::Select(EntryId::Quit) => {
                self.quit(context);
                hub.send(Event::Back).ok();
                true
            },
            _ => false,
        }
    }

    fn render(&self, fb: &mut dyn Framebuffer, rect: Rectangle, _fonts: &mut Fonts) {
        
        // if (! self.drawing) || self.mode == SketchMode::Full {
        //     fb.draw_framed_pixmap_blended(&self.image.pixmap(), &rect, rect.min, BLACK);
        // } else {
        //     fb.draw_framed_pixmap_halftone(&self.image.pixmap(), &self.random, &rect, rect.min);
        // }
    }

    fn render_rect(&self, rect: &Rectangle) -> Rectangle {
        rect.intersection(&self.rect)
            .unwrap_or(self.rect)
    }

    fn might_rotate(&self) -> bool {
        false
    }

    fn is_background(&self) -> bool {
        false
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
