use imgui::im_str;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

use super::ui::ImguiWindow;

pub struct EngineMetrics {
    pub frame_start: Mutex<Instant>,
    pub fps: Mutex<u32>,
    pub execution_time: Mutex<f64>,
    pub systems: HashMap<Uuid, Mutex<SystemMetrics>>,
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self {
            frame_start: Mutex::new(Instant::now()),
            fps: Mutex::new(60),
            execution_time: Mutex::new(0.0),
            systems: HashMap::new(),
        }
    }
}

impl EngineMetrics {
    pub fn register_system(&mut self, metrics: Mutex<SystemMetrics>) -> Uuid {
        let id = Uuid::new_v4();
        self.systems.insert(id, metrics);
        id
    }

    pub fn register_system_id(&mut self, id: Uuid, metrics: Mutex<SystemMetrics>) {
        self.systems.insert(id, metrics);
    }

    pub fn submit_system_run_time(&self, id: &Uuid, run_time_secs: f64) {
        self.systems.get(id).unwrap().lock().unwrap().run_time_secs += run_time_secs;
    }
}

pub struct SystemMetrics {
    pub system_name: String,
    pub run_time_secs: f64,
}

impl SystemMetrics {
    pub fn new(name: &str) -> Self {
        Self {
            system_name: name.to_owned(),
            run_time_secs: 0.0,
        }
    }
}

impl ImguiWindow for EngineMetrics {
    fn build(&self, frame: &imgui::Ui) {
        imgui::Window::new(im_str!("Ember Engine Debugger"))
            .size([225.0, 200.0], imgui::Condition::FirstUseEver)
            .build(&frame, || {
                if imgui::CollapsingHeader::new(im_str!("General"))
                    .default_open(true)
                    .build(&frame)
                {
                    frame.text(format!("fps: {}", *self.fps.lock().unwrap()));
                }

                frame.spacing();
                if imgui::CollapsingHeader::new(im_str!("Render Graph")).build(&frame) {
                    let execution_time = *self.execution_time.lock().unwrap();
                    frame.text("System Frame Usage");
                    for (_, system) in &self.systems {
                        let system = system.lock().unwrap();
                        let usage = ((system.run_time_secs / execution_time) * 100.0) as u32;
                        frame.text(format!("{}: {}%", system.system_name, usage));
                    }
                    frame.separator();
                }

                frame.separator();
                let mouse_pos = frame.io().mouse_pos;
                frame.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });
    }

    fn impl_imgui(self: Arc<Self>) -> Arc<dyn ImguiWindow> {
        self
    }
}
