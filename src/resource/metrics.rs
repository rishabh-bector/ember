use imgui::im_str;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use uuid::Uuid;

use super::ui::ImguiWindow;

pub struct EngineMetrics {
    pub systems: HashMap<Uuid, Arc<Mutex<SystemMetrics>>>,

    last_updated: Instant,

    // Written each frame
    // Should only store data for engine-level metrics
    frame_count: u32,

    // written each second, read each frame
    fps: u32,
    avg_execution_time: f64,
    percent_system_shares: HashMap<Uuid, (String, u32)>,
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self {
            fps: 60,
            avg_execution_time: 0.0,
            systems: HashMap::new(),
            percent_system_shares: HashMap::new(),
            last_updated: Instant::now(),
            frame_count: 0,
        }
    }

    pub fn register_system(&mut self, metrics: Arc<Mutex<SystemMetrics>>) -> Uuid {
        let id = Uuid::new_v4();
        self.systems.insert(id, metrics);
        self.percent_system_shares
            .insert(id, (metrics.lock().unwrap().system_name, 0));
        id
    }

    pub fn register_system_id(&mut self, id: Uuid, metrics: Arc<Mutex<SystemMetrics>>) {
        self.systems.insert(id, metrics);
        self.percent_system_shares
            .insert(id, (metrics.lock().unwrap().system_name, 0));
    }

    pub fn mut_system(&mut self, id: &Uuid) -> MutexGuard<SystemMetrics> {
        self.systems.get(id).unwrap().lock().unwrap()
    }

    // Should be called every frame
    pub fn update(&mut self) {
        self.frame_count += 1;
        if self.last_updated.elapsed() > Duration::from_secs(1) {
            // Metric: fps
            self.fps = (1.0
                / (self.last_updated.elapsed().as_secs_f64() / (self.frame_count as f64)))
                as u32
                + 1;
            self.last_updated = Instant::now();
            self.frame_count = 0;

            // Metric: average system run time
            self.avg_execution_time = 0.0;
            for (id, system) in &self.systems {
                self.avg_execution_time += system.lock().unwrap().avg_run_time;
            }
            self.avg_execution_time /= self.systems.len() as f64;

            // Metric: individual system frame share
            for (id, system) in &self.systems {
                let (name, _) = self.percent_system_shares.get(id).unwrap();
                self.percent_system_shares.insert(
                    *id,
                    (
                        name.to_owned(),
                        ((system.lock().unwrap().avg_run_time / self.avg_execution_time) * 100.0)
                            as u32,
                    ),
                );
            }
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
                    frame.text(format!("fps: {}", self.fps));
                }

                frame.spacing();
                if imgui::CollapsingHeader::new(im_str!("Render Graph")).build(&frame) {
                    frame.text("System Frame Usage");
                    for (_, (system_name, usage)) in &self.percent_system_shares {
                        frame.text(format!("{}: {}%", system_name, usage));
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

#[derive(Default)]
pub struct SystemMetrics {
    pub system_name: String,

    // Stats (updated once per second by reporter)
    avg_run_time: f64,
}

impl SystemMetrics {
    pub fn new(name: &str) -> Self {
        Self {
            system_name: name.to_owned(),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct SystemReporter {
    target: Arc<Mutex<SystemMetrics>>,
    last_reported: Instant,
    total_run_time: f64,
}

impl SystemReporter {
    pub fn new(target: Arc<Mutex<SystemMetrics>>) -> Self {
        Self {
            target,
            last_reported: Instant::now(),
            total_run_time: 0.0,
        }
    }

    // should be called every frame
    pub fn update(&mut self, run_time: f64) {
        self.total_run_time += run_time;

        if self.last_reported.elapsed() >= Duration::from_secs(1) {
            self.report();
            self.last_reported = Instant::now();
        }
    }

    // average run time of system (seconds)
    pub fn report(&self) {
        let avg = self.total_run_time / self.last_reported.elapsed().as_secs_f64();
        self.last_reported = Instant::now();
        self.total_run_time = 0.0;

        self.target.lock().unwrap().avg_run_time = avg;
    }
}
