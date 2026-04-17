use std::borrow::Cow;

use cosmic::{
    iced::{
        clipboard::{
            dnd::{self, DndAction, DndDestinationRectangle, DndEvent, OfferEvent, SourceEvent},
            mime::{AllowedMimeTypes, AsMimeTypes},
        },
        mouse,
    },
    iced_core::{
        self, Border, Color, Length, Point, Rectangle, Renderer, Shadow, Size,
        clipboard::DndSource, layout::Node, renderer::Quad,
    },
    widget::{self, Widget},
};

use crate::screenshot::{AnnotationTool, Rect, ScreenshotImage};

pub const MIME: &str = "X-COSMIC-PORTAL-MyData";
pub struct MyData;

impl From<(Vec<u8>, String)> for MyData {
    fn from(_: (Vec<u8>, String)) -> Self {
        MyData
    }
}

impl AllowedMimeTypes for MyData {
    fn allowed() -> std::borrow::Cow<'static, [String]> {
        std::borrow::Cow::Owned(vec![MIME.to_string()])
    }
}

impl AsMimeTypes for MyData {
    fn available(&self) -> std::borrow::Cow<'static, [String]> {
        std::borrow::Cow::Owned(vec![MIME.to_string()])
    }

    fn as_bytes(&self, _: &str) -> Option<std::borrow::Cow<'static, [u8]>> {
        Some(std::borrow::Cow::Borrowed("rectangle".as_bytes()))
    }
}

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragState {
    #[default]
    None,
    NW,
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
}

impl From<u8> for DragState {
    fn from(state: u8) -> Self {
        match state {
            0 => DragState::None,
            1 => DragState::NW,
            2 => DragState::N,
            3 => DragState::NE,
            4 => DragState::E,
            5 => DragState::SE,
            6 => DragState::S,
            7 => DragState::SW,
            8 => DragState::W,
            _ => unreachable!(),
        }
    }
}

const EDGE_GRAB_THICKNESS: f32 = 8.0;
const CORNER_DIAMETER: f32 = 16.0;

pub struct RectangleSelection<Msg> {
    output_rect: Rect,
    rectangle_selection: Rect,
    window_id: iced_core::window::Id,
    on_rectangle: Box<dyn Fn(DragState, Rect) -> Msg>,
    drag_state: DragState,
    widget_id: widget::Id,
    drag_id: u128,
    output_image: ScreenshotImage,
    annotation_tool: AnnotationTool,
}

#[derive(Default)]
struct RectangleSelectionState {
    cursor_position: Option<Point>,
}

impl<Msg> RectangleSelection<Msg> {
    pub fn new(
        output_rect: Rect,
        rectangle_selection: Rect,
        drag_direction: DragState,
        window_id: iced_core::window::Id,
        drag_id: u128,
        on_rectangle: impl Fn(DragState, Rect) -> Msg + 'static,
        output_image: &ScreenshotImage,
        annotation_tool: &AnnotationTool,
    ) -> Self {
        Self {
            on_rectangle: Box::new(on_rectangle),
            drag_state: drag_direction,
            rectangle_selection,
            output_rect,
            window_id,
            drag_id,
            widget_id: widget::Id::new(format!("rectangle-selection-{window_id:?}")),
            output_image: output_image.clone(),
            annotation_tool: annotation_tool.clone(),
        }
    }

    pub fn translated_inner_rect(&self) -> Rectangle {
        let inner_rect = self.rectangle_selection;
        let inner_rect = Rectangle::new(
            Point::new(inner_rect.left as f32, inner_rect.top as f32),
            Size::new(
                (inner_rect.right - inner_rect.left).abs() as f32,
                (inner_rect.bottom - inner_rect.top).abs() as f32,
            ),
        );
        Rectangle::new(
            Point::new(
                inner_rect.x - self.output_rect.left as f32,
                inner_rect.y - self.output_rect.top as f32,
            ),
            inner_rect.size(),
        )
    }

    fn drag_state(&self, cursor: mouse::Cursor) -> DragState {
        let inner_rect = self.translated_inner_rect();

        let nw_corner_rect = Rectangle::new(
            Point::new(
                inner_rect.x - CORNER_DIAMETER / 2.0,
                inner_rect.y - CORNER_DIAMETER / 2.0,
            ),
            Size::new(CORNER_DIAMETER, CORNER_DIAMETER),
        );
        // TODO need NW, NE, SW, SE resize cursors
        if cursor.is_over(nw_corner_rect) {
            return DragState::NW;
        };

        let ne_corner_rect = Rectangle::new(
            Point::new(
                inner_rect.x + inner_rect.width - CORNER_DIAMETER / 2.0,
                inner_rect.y - CORNER_DIAMETER / 2.0,
            ),
            Size::new(CORNER_DIAMETER, CORNER_DIAMETER),
        );
        if cursor.is_over(ne_corner_rect) {
            return DragState::NE;
        };

        let sw_corner_rect = Rectangle::new(
            Point::new(
                inner_rect.x - CORNER_DIAMETER / 2.0,
                inner_rect.y + inner_rect.height - CORNER_DIAMETER / 2.0,
            ),
            Size::new(CORNER_DIAMETER, CORNER_DIAMETER),
        );
        if cursor.is_over(sw_corner_rect) {
            return DragState::SW;
        };

        let se_corner_rect = Rectangle::new(
            Point::new(
                inner_rect.x + inner_rect.width - CORNER_DIAMETER / 2.,
                inner_rect.y + inner_rect.height - CORNER_DIAMETER / 2.,
            ),
            Size::new(CORNER_DIAMETER, CORNER_DIAMETER),
        );
        if cursor.is_over(se_corner_rect) {
            return DragState::SE;
        };

        let n_edge_rect = Rectangle::new(
            Point::new(inner_rect.x, inner_rect.y - EDGE_GRAB_THICKNESS / 2.0),
            Size::new(inner_rect.width, EDGE_GRAB_THICKNESS),
        );
        if cursor.is_over(n_edge_rect) {
            return DragState::N;
        };

        let s_edge_rect = Rectangle::new(
            Point::new(
                inner_rect.x,
                inner_rect.y + inner_rect.height - EDGE_GRAB_THICKNESS / 2.0,
            ),
            Size::new(inner_rect.width, EDGE_GRAB_THICKNESS),
        );
        if cursor.is_over(s_edge_rect) {
            return DragState::S;
        };

        let w_edge_rect = Rectangle::new(
            Point::new(inner_rect.x - EDGE_GRAB_THICKNESS / 2.0, inner_rect.y),
            Size::new(EDGE_GRAB_THICKNESS, inner_rect.height),
        );
        if cursor.is_over(w_edge_rect) {
            return DragState::W;
        };

        let e_edge_rect = Rectangle::new(
            Point::new(
                inner_rect.x + inner_rect.width - EDGE_GRAB_THICKNESS / 2.0,
                inner_rect.y,
            ),
            Size::new(EDGE_GRAB_THICKNESS, inner_rect.height),
        );
        if cursor.is_over(e_edge_rect) {
            return DragState::E;
        };
        DragState::None
    }

    fn handle_drag_pos(&mut self, x: i32, y: i32, shell: &mut iced_core::Shell<'_, Msg>) {
        let prev = self.rectangle_selection;

        let d_x = self.output_rect.left + x;
        let d_y = self.output_rect.top + y;

        let prev_state = self.drag_state;
        // the point of reflection is where, when crossed, the drag state changes to the opposit direction
        // for edge drags, only one of the x or y coordinate is used, for corner drags, both are used
        // the new dimensions are calculated by subtracting the reflection point from the drag point
        let reflection_point = match prev_state {
            DragState::None => return,
            DragState::NW => (prev.right, prev.bottom),
            DragState::N => (0, prev.bottom),
            DragState::NE => (prev.left, prev.bottom),
            DragState::E => (prev.left, 0),
            DragState::SE => (prev.left, prev.top),
            DragState::S => (0, prev.top),
            DragState::SW => (prev.right, prev.top),
            DragState::W => (prev.right, 0),
        };

        let new_drag_state = match prev_state {
            DragState::SE | DragState::NW | DragState::NE | DragState::SW => {
                if d_x < reflection_point.0 && d_y < reflection_point.1 {
                    DragState::NW
                } else if d_x > reflection_point.0 && d_y > reflection_point.1 {
                    DragState::SE
                } else if d_x > reflection_point.0 && d_y < reflection_point.1 {
                    DragState::NE
                } else if d_x < reflection_point.0 && d_y > reflection_point.1 {
                    DragState::SW
                } else {
                    prev_state
                }
            }
            DragState::N | DragState::S => {
                if d_y < reflection_point.1 {
                    DragState::N
                } else {
                    DragState::S
                }
            }
            DragState::E | DragState::W => {
                if d_x > reflection_point.0 {
                    DragState::E
                } else {
                    DragState::W
                }
            }

            DragState::None => DragState::None,
        };
        let top_left = match new_drag_state {
            DragState::NW => (d_x, d_y),
            DragState::NE => (reflection_point.0, d_y),
            DragState::SE => (reflection_point.0, reflection_point.1),
            DragState::SW => (d_x, reflection_point.1),
            DragState::N => (prev.left, d_y),
            DragState::E => (reflection_point.0, prev.top),
            DragState::S => (prev.left, reflection_point.1),
            DragState::W => (d_x, prev.top),
            DragState::None => (prev.left, prev.top),
        };

        let bottom_right = match new_drag_state {
            DragState::NW => (reflection_point.0, reflection_point.1),
            DragState::NE => (d_x, reflection_point.1),
            DragState::SE => (d_x, d_y),
            DragState::SW => (reflection_point.0, d_y),
            DragState::N => (prev.right, reflection_point.1),
            DragState::E => (d_x, prev.bottom),
            DragState::S => (prev.right, d_y),
            DragState::W => (reflection_point.0, prev.bottom),
            DragState::None => (prev.right, prev.bottom),
        };
        let new_rect = Rect {
            left: top_left.0,
            top: top_left.1,
            right: bottom_right.0,
            bottom: bottom_right.1,
        };
        self.rectangle_selection = new_rect;
        self.drag_state = new_drag_state;

        shell.publish((self.on_rectangle)(new_drag_state, new_rect));
    }

    fn draw_magnifier(
        &self,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        cursor_position: Option<Point>,
    ) {
        const SAMPLE_SIZE: i32 = 15;
        const ZOOM: f32 = 10.0;
        const GAP: f32 = 20.0;
        const BORDER_WIDTH: f32 = 2.0;

        let Some(cursor_pos) = cursor_position else {
            return;
        };

        let cosmic = theme.cosmic();

        let image = &self.output_image.rgba;
        if image.width() == 0 || image.height() == 0 {
            return;
        }

        let logical_width = (self.output_rect.right - self.output_rect.left).unsigned_abs() as f32;
        let logical_height = (self.output_rect.bottom - self.output_rect.top).unsigned_abs() as f32;
        if logical_width <= 0.0 || logical_height <= 0.0 {
            return;
        }

        let local_x = cursor_pos.x.clamp(0.0, logical_width.max(1.0) - 1.0);
        let local_y = cursor_pos.y.clamp(0.0, logical_height.max(1.0) - 1.0);

        let scale_x = image.width() as f32 / logical_width;
        let scale_y = image.height() as f32 / logical_height;
        let center_px = (local_x * scale_x).floor() as i32;
        let center_py = (local_y * scale_y).floor() as i32;

        let magnifier_size = SAMPLE_SIZE as f32 * ZOOM;
        let mut magnifier_x = cursor_pos.x + GAP;
        let mut magnifier_y = cursor_pos.y + GAP;
        if magnifier_x + magnifier_size > logical_width {
            magnifier_x = cursor_pos.x - GAP - magnifier_size;
        }
        if magnifier_y + magnifier_size > logical_height {
            magnifier_y = cursor_pos.y - GAP - magnifier_size;
        }
        magnifier_x = magnifier_x.clamp(0.0, logical_width - magnifier_size);
        magnifier_y = magnifier_y.clamp(0.0, logical_height - magnifier_size);

        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(
                    Point::new(magnifier_x, magnifier_y),
                    Size::new(magnifier_size, magnifier_size),
                ),
                border: Border {
                    radius: cosmic.radius_xl().into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                shadow: Shadow::default(),
                snap: true,
            },
            Color::from_rgba(0.0, 0.0, 0.0, 0.7),
        );

        let half = SAMPLE_SIZE / 2;
        for gy in 0..SAMPLE_SIZE {
            for gx in 0..SAMPLE_SIZE {
                let src_x = (center_px + gx - half).clamp(0, image.width() as i32 - 1) as u32;
                let src_y = (center_py + gy - half).clamp(0, image.height() as i32 - 1) as u32;
                let p = image.get_pixel(src_x, src_y).0;
                let color = Color::from_rgba(
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                    p[3] as f32 / 255.0,
                );

                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(
                                magnifier_x + gx as f32 * ZOOM,
                                magnifier_y + gy as f32 * ZOOM,
                            ),
                            Size::new(ZOOM, ZOOM),
                        ),
                        border: Border::default(),
                        shadow: Shadow::default(),
                        snap: true,
                    },
                    color,
                );
            }
        }

        
        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(
                    Point::new(magnifier_x - BORDER_WIDTH, magnifier_y - BORDER_WIDTH),
                    Size::new(
                        magnifier_size + BORDER_WIDTH * 2.0,
                        magnifier_size + BORDER_WIDTH * 2.0,
                    ),
                ),
                border: Border {
                    radius: cosmic.radius_s().into(),
                    width: BORDER_WIDTH,
                    color: Color::from(cosmic.accent_color()),
                },
                shadow: Shadow::default(),
                snap: true,
            },
            Color::TRANSPARENT,
        );

        let center_cell_x = magnifier_x + (half as f32) * ZOOM;
        let center_cell_y = magnifier_y + (half as f32) * ZOOM;
        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(
                    Point::new(center_cell_x, center_cell_y),
                    Size::new(ZOOM, ZOOM),
                ),
                border: Border {
                    radius: 0.0.into(),
                    width: 2.0,
                    color: Color::WHITE,
                },
                shadow: Shadow::default(),
                snap: true,
            },
            Color::TRANSPARENT,
        );
    }
}

impl<Msg: 'static + Clone> Widget<Msg, cosmic::Theme, cosmic::Renderer>
    for RectangleSelection<Msg>
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut cosmic::iced_core::widget::Tree,
        _renderer: &cosmic::Renderer,
        limits: &cosmic::iced_core::layout::Limits,
    ) -> cosmic::iced_core::layout::Node {
        Node::new(limits.width(Length::Fill).height(Length::Fill).resolve(
            Length::Fill,
            Length::Fill,
            cosmic::iced_core::Size::ZERO,
        ))
    }

    fn tag(&self) -> iced_core::widget::tree::Tag {
        iced_core::widget::tree::Tag::of::<RectangleSelectionState>()
    }

    fn state(&self) -> iced_core::widget::tree::State {
        iced_core::widget::tree::State::new(RectangleSelectionState::default())
    }

    fn mouse_interaction(
        &self,
        _state: &iced_core::widget::Tree,
        _layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &cosmic::Renderer,
    ) -> iced_core::mouse::Interaction {
        match self.drag_state(cursor) {
            DragState::None => {
                if self.drag_state == DragState::None {
                    iced_core::mouse::Interaction::Crosshair
                } else {
                    iced_core::mouse::Interaction::Grabbing
                }
            }
            DragState::NW | DragState::NE | DragState::SE | DragState::SW => {
                if self.drag_state == DragState::None {
                    iced_core::mouse::Interaction::Grab
                } else {
                    iced_core::mouse::Interaction::Grabbing
                }
            }
            DragState::N | DragState::S => iced_core::mouse::Interaction::ResizingVertically,
            DragState::E | DragState::W => iced_core::mouse::Interaction::ResizingHorizontally,
        }
    }

    fn update(
        &mut self,
        state: &mut iced_core::widget::Tree,
        event: &iced_core::Event,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        _renderer: &cosmic::Renderer,
        clipboard: &mut dyn iced_core::Clipboard,
        shell: &mut iced_core::Shell<'_, Msg>,
        _viewport: &Rectangle,
    ) {
        let state = state.state.downcast_mut::<RectangleSelectionState>();
        match event {
            cosmic::iced_core::Event::Dnd(DndEvent::Offer(id, e)) if *id == Some(self.drag_id) => {
                if self.drag_state == DragState::None {
                    return;
                }
                // Don't need to accept mime types or actions
                match e {
                    OfferEvent::Enter { x, y, .. } => {
                        state.cursor_position = Some(Point::new(*x as f32, *y as f32));
                        let p = Point::new(*x as f32, *y as f32);
                        let cursor = mouse::Cursor::Available(p);
                        if !cursor.is_over(layout.bounds()) {
                            return;
                        }

                        self.handle_drag_pos(x.round() as i32, y.round() as i32, shell);
                        shell.capture_event();
                    }
                    OfferEvent::Motion { x, y } => {
                        state.cursor_position = Some(Point::new(*x as f32, *y as f32));
                        let p = Point::new(*x as f32, *y as f32);
                        let cursor = mouse::Cursor::Available(p);
                        if !cursor.is_over(layout.bounds()) {
                            return;
                        }
                        self.handle_drag_pos(x.round() as i32, y.round() as i32, shell);
                        shell.capture_event();
                    }
                    OfferEvent::Drop => {
                        state.cursor_position = None;
                        self.drag_state = DragState::None;
                        shell.publish((self.on_rectangle)(
                            DragState::None,
                            self.rectangle_selection,
                        ));
                        shell.capture_event();
                    }
                    _ => {}
                }
            }
            cosmic::iced_core::Event::Dnd(DndEvent::Source(e)) => {
                if matches!(
                    e,
                    SourceEvent::Finished | SourceEvent::Cancelled | SourceEvent::Dropped
                ) {
                    state.cursor_position = None;
                    self.drag_state = DragState::None;
                    shell.publish((self.on_rectangle)(
                        DragState::None,
                        self.rectangle_selection,
                    ));
                }
            }
            cosmic::iced_core::Event::Mouse(e) => {
                match e {
                    iced_core::mouse::Event::CursorMoved { position } => {
                        state.cursor_position = Some(*position);
                    }
                    iced_core::mouse::Event::CursorLeft => {
                        state.cursor_position = None;
                    }
                    _ => {}
                }

                if !cursor.is_over(layout.bounds()) {
                    return;
                }

                // on press start internal DnD and set drag state
                if let iced_core::mouse::Event::ButtonPressed(iced_core::mouse::Button::Left) = e {
                    let window_id = self.window_id;

                    clipboard.start_dnd(
                        false,
                        Some(DndSource::Surface(window_id)),
                        None,
                        Box::new(MyData),
                        DndAction::Copy,
                    );

                    let s = self.drag_state(cursor);
                    if let DragState::None = s {
                        let mut pos = cursor.position().unwrap_or_default();
                        pos.x += self.output_rect.left as f32;
                        pos.y += self.output_rect.top as f32;
                        self.drag_state = DragState::SE;
                        shell.publish((self.on_rectangle)(
                            DragState::SE,
                            Rect {
                                left: pos.x as i32,
                                top: pos.y as i32,
                                right: pos.x as i32 + 1,
                                bottom: pos.y as i32 + 1,
                            },
                        ));
                    } else {
                        self.drag_state = s;
                        shell.publish((self.on_rectangle)(s, self.rectangle_selection));
                    }
                    return shell.capture_event();
                }
                shell.capture_event();
            }
            _ => (),
        };
    }

    fn draw(
        &self,
        tree: &cosmic::iced_core::widget::Tree,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        _style: &cosmic::iced_core::renderer::Style,
        _layout: cosmic::iced_core::Layout<'_>,
        _cursor: cosmic::iced_core::mouse::Cursor,
        _viewport: &cosmic::iced_core::Rectangle,
    ) {
        // first draw background overlay for non-selected bg
        // then draw quad for selection clipped to output rect
        // then optionally draw handles if they are in the output rect

        let cosmic = theme.cosmic();
        let accent = Color::from(cosmic.accent_color());
        let inner_rect = self.rectangle_selection;
        let inner_rect = Rectangle::new(
            Point::new(inner_rect.left as f32, inner_rect.top as f32),
            Size::new(
                (inner_rect.right - inner_rect.left).abs() as f32,
                (inner_rect.bottom - inner_rect.top).abs() as f32,
            ),
        );
        let outer_size = Size::new(
            (self.output_rect.right - self.output_rect.left).abs() as f32,
            (self.output_rect.bottom - self.output_rect.top).abs() as f32,
        );
        let outer_top_left = Point::new(self.output_rect.left as f32, self.output_rect.top as f32);
        let outer_rect = Rectangle::new(outer_top_left, outer_size);
        if let Some(clipped_inner_rect) = inner_rect.intersection(&outer_rect) {
            let translated_clipped_inner_rect = Rectangle::new(
                Point::new(
                    clipped_inner_rect.x - outer_rect.x,
                    clipped_inner_rect.y - outer_rect.y,
                ),
                clipped_inner_rect.size(),
            );
            let mut overlay = Color::BLACK;
            overlay.a = 0.45;

            // Here we darken everything outside the selected area
            let top_overlay = Rectangle::new(
                Point::new(0.0, 0.0),
                Size::new(outer_size.width, translated_clipped_inner_rect.y.max(0.0)),
            );
            let bottom_overlay = Rectangle::new(
                Point::new(
                    0.0,
                    translated_clipped_inner_rect.y + translated_clipped_inner_rect.height,
                ),
                Size::new(
                    outer_size.width,
                    (outer_size.height
                        - (translated_clipped_inner_rect.y + translated_clipped_inner_rect.height))
                    .max(0.0),
                ),
            );
            let left_overlay = Rectangle::new(
                Point::new(0.0, translated_clipped_inner_rect.y),
                Size::new(
                    translated_clipped_inner_rect.x.max(0.0),
                    translated_clipped_inner_rect.height,
                ),
            );
            let right_overlay = Rectangle::new(
                Point::new(
                    translated_clipped_inner_rect.x + translated_clipped_inner_rect.width,
                    translated_clipped_inner_rect.y,
                ),
                Size::new(
                    (outer_size.width
                        - (translated_clipped_inner_rect.x + translated_clipped_inner_rect.width))
                    .max(0.0),
                    translated_clipped_inner_rect.height,
                ),
            );
            for bounds in [
                top_overlay,
                bottom_overlay,
                left_overlay,
                right_overlay,
            ] {
                if bounds.width <= 0.0 || bounds.height <= 0.0 {
                    continue;
                }
                let quad = Quad {
                    bounds,
                    border: Border {
                        radius: 0.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    shadow: Shadow::default(),
                    snap: true,
                };
                renderer.fill_quad(quad, overlay);
            }

            let quad = Quad {
                bounds: translated_clipped_inner_rect,
                border: Border {
                    radius: 0.0.into(),
                    width: 4.0,
                    color: accent,
                },
                shadow: Shadow::default(),
                snap: true,
            };
            renderer.fill_quad(quad, Color::TRANSPARENT);

            // draw handles as quads with radius_s
            let radius_s = cosmic.radius_s();
            for (x, y) in &[
                (inner_rect.x, inner_rect.y),
                (inner_rect.x + inner_rect.width, inner_rect.y),
                (inner_rect.x, inner_rect.y + inner_rect.height),
                (
                    inner_rect.x + inner_rect.width,
                    inner_rect.y + inner_rect.height,
                ),
            ] {
                if !outer_rect.contains(Point::new(*x, *y)) {
                    continue;
                }
                let translated_x = x - outer_rect.x;
                let translated_y = y - outer_rect.y;
                let bounds = Rectangle::new(
                    Point::new(
                        translated_x - CORNER_DIAMETER / 2.0,
                        translated_y - CORNER_DIAMETER / 2.0,
                    ),
                    Size::new(CORNER_DIAMETER, CORNER_DIAMETER),
                );
                let quad = Quad {
                    bounds,
                    border: Border {
                        radius: radius_s.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    shadow: Shadow::default(),
                    snap: true,
                };
                renderer.fill_quad(quad, accent);
            }
        }

        if self.annotation_tool == AnnotationTool::Magnifier {
            let state = tree.state.downcast_ref::<RectangleSelectionState>();
            self.draw_magnifier(renderer, theme, state.cursor_position);
        }

    }

    fn drag_destinations(
        &self,
        _state: &iced_core::widget::Tree,
        layout: iced_core::Layout<'_>,
        _renderer: &cosmic::Renderer,
        dnd_rectangles: &mut iced_core::clipboard::DndDestinationRectangles,
    ) {
        let bounds = layout.bounds();
        dnd_rectangles.push(DndDestinationRectangle {
            id: self.drag_id,
            rectangle: dnd::Rectangle {
                x: bounds.x as f64,
                y: bounds.y as f64,
                width: bounds.width as f64,
                height: bounds.height as f64,
            },
            mime_types: vec![Cow::Borrowed(MIME)],
            actions: DndAction::Copy,
            preferred: DndAction::Copy,
        });
    }

    fn set_id(&mut self, id: widget::Id) {
        self.widget_id = id;
    }
}

impl<'a, Message> From<RectangleSelection<Message>> for cosmic::Element<'a, Message>
where
    Message: 'static + Clone,
{
    fn from(w: RectangleSelection<Message>) -> cosmic::Element<'a, Message> {
        cosmic::Element::new(w)
    }
}
