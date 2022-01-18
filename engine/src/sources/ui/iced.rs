use anyhow::Result;
use futures::executor::LocalPool;
use iced::{Point, Size};
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Clipboard, Debug};
use legion::Resources;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;
use wgpu::util::StagingBelt;
use winit::{dpi::PhysicalPosition, event::ModifiersState, window::Window};

use crate::renderer::{graph::target::RenderTarget, WindowWrapper};

pub struct IcedWinitHelper {
    pub cursor_position: PhysicalPosition<f64>,
    pub modifiers: ModifiersState,
    pub viewport: Viewport,
}

impl IcedWinitHelper {
    pub fn new(window: &winit::window::Window) -> Self {
        let physical_size = window.inner_size();
        let viewport = Viewport::with_physical_size(
            Size::new(physical_size.width, physical_size.height),
            window.scale_factor(),
        );

        let cursor_position = PhysicalPosition::new(-1.0, -1.0);
        let modifiers = ModifiersState::default();

        Self {
            cursor_position,
            modifiers,
            viewport,
        }
    }
}

pub struct IcedUI {
    pub target: Arc<Mutex<RenderTarget>>,
    pub renderer: Arc<Mutex<Renderer>>,
    pub local_pool: LocalPool,
    pub state: program::State<Controls>,
}

impl IcedUI {
    pub fn new(
        target: Arc<Mutex<RenderTarget>>,
        device: &Arc<wgpu::Device>,
        window: &winit::window::Window,
        format: wgpu::TextureFormat,
        helper: &IcedWinitHelper,
        debug: &mut Debug,
    ) -> (Self, StagingBelt) {
        let mut renderer = Renderer::new(Backend::new(&device, Settings::default(), format));

        let staging_belt = StagingBelt::new(5 * 1024);
        let local_pool = LocalPool::new();

        let controls = Controls::new();
        let state = program::State::new(
            controls,
            helper.viewport.logical_size(),
            &mut renderer,
            debug,
        );

        (
            Self {
                target,
                local_pool,
                state,
                renderer: Arc::new(Mutex::new(renderer)),
            },
            staging_belt,
        )
    }

    pub fn update(
        &mut self,
        clipboard: &mut Clipboard,
        helper: &IcedWinitHelper,
        ui_debug: &mut Debug,
    ) {
        let mut renderer = self.renderer.lock().unwrap();
        self.state.update(
            helper.viewport.logical_size(),
            conversion::cursor_position(helper.cursor_position, helper.viewport.scale_factor()),
            &mut renderer,
            clipboard,
            ui_debug,
        );
    }
}

use iced_winit::widget::slider::{self, Slider};
use iced_winit::widget::{Column, Row, Text};
use iced_winit::{Alignment, Color, Command, Element, Length, Program};

#[derive(Debug, Clone)]
pub enum Message {
    BackgroundColorChanged(Color),
}

pub struct Controls {
    background_color: Color,
    sliders: [slider::State; 3],
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            background_color: Color::BLACK,
            sliders: Default::default(),
        }
    }

    pub fn background_color(&self) -> Color {
        self.background_color
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::BackgroundColorChanged(color) => {
                self.background_color = color;
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let [r, g, b] = &mut self.sliders;
        let background_color = self.background_color;

        let sliders = Row::new()
            .width(Length::Units(500))
            .spacing(20)
            .push(
                Slider::new(r, 0.0..=1.0, background_color.r, move |r| {
                    Message::BackgroundColorChanged(Color {
                        r,
                        ..background_color
                    })
                })
                .step(0.01),
            )
            .push(
                Slider::new(g, 0.0..=1.0, background_color.g, move |g| {
                    Message::BackgroundColorChanged(Color {
                        g,
                        ..background_color
                    })
                })
                .step(0.01),
            )
            .push(
                Slider::new(b, 0.0..=1.0, background_color.b, move |b| {
                    Message::BackgroundColorChanged(Color {
                        b,
                        ..background_color
                    })
                })
                .step(0.01),
            );

        Row::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::End)
            .push(
                Column::new()
                    .width(Length::Fill)
                    .align_items(Alignment::End)
                    .push(
                        Column::new()
                            .padding(10)
                            .spacing(10)
                            .push(Text::new("Background color").color(Color::WHITE))
                            .push(sliders)
                            .push(
                                Text::new(format!("{:?}", background_color))
                                    .size(14)
                                    .color(Color::WHITE),
                            ),
                    ),
            )
            .into()
    }
}
