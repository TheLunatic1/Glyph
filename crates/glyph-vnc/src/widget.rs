use crate::keysym::iced_key_to_keysym;
use crate::rfb::VncFrame;
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{Action, Event, Frame, Geometry, Program};
use iced::widget::{canvas, image, stack};
use iced::{ContentFit, Element, Length, Rectangle, Renderer, Theme};

#[derive(Debug, Clone)]
pub enum VncUiEvent {
    Pointer { x: u16, y: u16, mask: u8 },
    Key { keysym: u32, down: bool },
    Focused,
}

pub fn vnc_viewport<'a>(
    frame: &'a VncFrame,
    fit_to_window: bool,
    focused: bool,
) -> Element<'a, VncUiEvent> {
    let handle = image::Handle::from_rgba(frame.width as u32, frame.height as u32, frame.rgba.clone());

    let img_widget: Element<'a, VncUiEvent> = if fit_to_window {
        image(handle)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Contain)
            .into()
    } else {
        image(handle)
            .width(Length::Fixed(frame.width as f32))
            .height(Length::Fixed(frame.height as f32))
            .content_fit(ContentFit::Fill)
            .into()
    };

    let overlay: Element<'a, VncUiEvent> = if fit_to_window {
        canvas(VncInputOverlay {
            frame_w: frame.width,
            frame_h: frame.height,
            fit_to_window: true,
            focused,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        canvas(VncInputOverlay {
            frame_w: frame.width,
            frame_h: frame.height,
            fit_to_window: false,
            focused,
        })
        .width(Length::Fixed(frame.width as f32))
        .height(Length::Fixed(frame.height as f32))
        .into()
    };

    stack![img_widget, overlay]
        .width(if fit_to_window {
            Length::Fill
        } else {
            Length::Fixed(frame.width as f32)
        })
        .height(if fit_to_window {
            Length::Fill
        } else {
            Length::Fixed(frame.height as f32)
        })
        .into()
}

struct VncInputOverlay {
    frame_w: u16,
    frame_h: u16,
    fit_to_window: bool,
    focused: bool,
}

#[derive(Default)]
struct OverlayState {
    buttons_mask: u8,
    last_xy: (u16, u16),
}

impl VncInputOverlay {
    fn map_cursor(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<(u16, u16)> {
        let pos = cursor.position_in(bounds)?;
        let fw = self.frame_w.max(1) as f32;
        let fh = self.frame_h.max(1) as f32;

        let (rx, ry) = if self.fit_to_window && bounds.width > 0.0 && bounds.height > 0.0 {
            let scale = (bounds.width / fw).min(bounds.height / fh);
            let draw_w = fw * scale;
            let draw_h = fh * scale;
            let off_x = (bounds.width - draw_w) * 0.5;
            let off_y = (bounds.height - draw_h) * 0.5;
            let local_x = (pos.x - off_x) / scale;
            let local_y = (pos.y - off_y) / scale;
            if local_x < 0.0 || local_y < 0.0 || local_x > fw || local_y > fh {
                return None;
            }
            (local_x, local_y)
        } else {
            (pos.x, pos.y)
        };

        let x = rx.round().clamp(0.0, (self.frame_w.saturating_sub(1)) as f32) as u16;
        let y = ry.round().clamp(0.0, (self.frame_h.saturating_sub(1)) as f32) as u16;
        Some((x, y))
    }
}

impl Program<VncUiEvent> for VncInputOverlay {
    type State = OverlayState;

    fn draw(
        &self,
        _state: &Self::State,
        _renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let frame = Frame::new(_renderer, bounds.size());
        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<VncUiEvent>> {
        match event {
            Event::Mouse(me) => match me {
                mouse::Event::CursorMoved { .. } => {
                    let (x, y) = self.map_cursor(bounds, cursor)?;
                    state.last_xy = (x, y);
                    Some(
                        Action::publish(VncUiEvent::Pointer {
                            x,
                            y,
                            mask: state.buttons_mask,
                        })
                        .and_capture(),
                    )
                }
                mouse::Event::ButtonPressed(btn) => {
                    let bit = button_bit(*btn);
                    if let Some((x, y)) = self.map_cursor(bounds, cursor) {
                        state.last_xy = (x, y);
                        state.buttons_mask |= bit;
                        return Some(
                            Action::publish(VncUiEvent::Pointer {
                                x,
                                y,
                                mask: state.buttons_mask,
                            })
                            .and_capture(),
                        );
                    }
                    if cursor.is_over(bounds) {
                        return Some(Action::publish(VncUiEvent::Focused).and_capture());
                    }
                    None
                }
                mouse::Event::ButtonReleased(btn) => {
                    let bit = button_bit(*btn);
                    state.buttons_mask &= !bit;
                    let (x, y) = self.map_cursor(bounds, cursor).unwrap_or(state.last_xy);
                    Some(
                        Action::publish(VncUiEvent::Pointer {
                            x,
                            y,
                            mask: state.buttons_mask,
                        })
                        .and_capture(),
                    )
                }
                mouse::Event::WheelScrolled { delta } => {
                    let (x, y) = self.map_cursor(bounds, cursor)?;
                    let wheel_bit = match delta {
                        mouse::ScrollDelta::Lines { y: dy, .. }
                        | mouse::ScrollDelta::Pixels { y: dy, .. } => {
                            if *dy > 0.0 {
                                1 << 3 // RFB button 4: scroll up
                            } else if *dy < 0.0 {
                                1 << 4 // RFB button 5: scroll down
                            } else {
                                0
                            }
                        }
                    };
                    if wheel_bit != 0 {
                        Some(
                            Action::publish(VncUiEvent::Pointer {
                                x,
                                y,
                                mask: state.buttons_mask | wheel_bit,
                            })
                            .and_capture(),
                        )
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Event::Keyboard(ke) if self.focused || cursor.is_over(bounds) => match ke {
                keyboard::Event::KeyPressed { key, text, .. } => {
                    let txt = text.as_ref().map(|s| s.as_str());
                    let ks = iced_key_to_keysym(key, txt)?;
                    Some(Action::publish(VncUiEvent::Key { keysym: ks, down: true }).and_capture())
                }
                keyboard::Event::KeyReleased { key, .. } => {
                    let ks = iced_key_to_keysym(key, None)?;
                    Some(Action::publish(VncUiEvent::Key { keysym: ks, down: false }).and_capture())
                }
                _ => None,
            },
            _ => None,
        }
    }
}

fn button_bit(btn: mouse::Button) -> u8 {
    match btn {
        mouse::Button::Left => 1 << 0,
        mouse::Button::Middle => 1 << 1,
        mouse::Button::Right => 1 << 2,
        _ => 0,
    }
}
