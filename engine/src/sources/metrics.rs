// use imgui::im_str;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use uuid::Uuid;

// use super::ui::imgui::ImguiWindow;

pub struct EngineMetrics {
    pub systems: HashMap<Uuid, Arc<Mutex<SystemMetrics>>>,
    pub ui: Arc<Mutex<EngineMetricsUI>>,
    pub fps: Arc<Mutex<u32>>,
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self {
            ui: Default::default(),
            fps: Arc::new(Mutex::new(0)),
            systems: HashMap::new(),
        }
    }

    // pub fn register_system(&mut self, metrics: Arc<Mutex<SystemMetrics>>) -> Uuid {
    //     let id = Uuid::new_v4();
    //     self.percent_system_shares
    //         .insert(id, (metrics.lock().unwrap().system_name.to_owned(), 0));
    //     self.systems.insert(id, metrics);
    //     id
    // }

    pub fn register_system_id(&mut self, name: &str, id: Uuid) -> SystemReporter {
        let system_metrics = Arc::new(Mutex::new(SystemMetrics::new(name)));
        let reporter = SystemReporter::new(Arc::clone(&system_metrics));
        self.ui.lock().unwrap().percent_system_shares.insert(
            id,
            (system_metrics.lock().unwrap().system_name.to_owned(), 0),
        );
        self.systems.insert(id, system_metrics);
        reporter
    }

    pub fn mut_system(&mut self, id: &Uuid) -> MutexGuard<SystemMetrics> {
        self.systems.get(id).unwrap().lock().unwrap()
    }

    // Expensive, should not be called every frame
    pub fn calculate(&self) {
        let mut ui = self.ui.lock().unwrap();

        // Metric: average fps (from reporter)
        ui.avg_fps = *self.fps.lock().unwrap();
        info!("average fps: {}", ui.avg_fps);

        // Metric: average system run time
        ui.avg_execution_time = 0.0;
        for (_, system) in &self.systems {
            ui.avg_execution_time += system.lock().unwrap().avg_run_time;
        }
        let total_execution_time = ui.avg_execution_time;
        ui.avg_execution_time /= self.systems.len() as f64;

        // Metric: individual system frame share
        for (id, system) in &self.systems {
            let (name, _) = ui.percent_system_shares.get(id).unwrap();
            let name = name.to_owned();
            ui.percent_system_shares.insert(
                *id,
                (
                    name.to_owned(),
                    ((system.lock().unwrap().avg_run_time / total_execution_time) * 100.0) as u32,
                ),
            );
        }
    }
}

// impl ImguiWindow for EngineMetrics {
//     fn build(&self, frame: &imgui::Ui) {
//         self.ui.lock().unwrap().build(frame);
//     }

//     fn impl_imgui(self: Arc<Self>) -> Arc<dyn ImguiWindow> {
//         self
//     }
// }

#[derive(Default)]
pub struct EngineMetricsUI {
    pub avg_fps: u32,
    pub percent_system_shares: HashMap<Uuid, (String, u32)>,
    pub avg_execution_time: f64,
}

// impl ImguiWindow for EngineMetricsUI {
//     fn build(&self, frame: &imgui::Ui) {
//         imgui::Window::new(im_str!("Ember Engine Debugger"))
//             .size([225.0, 200.0], imgui::Condition::FirstUseEver)
//             .build(&frame, || {
//                 if imgui::CollapsingHeader::new(im_str!("General"))
//                     .default_open(true)
//                     .build(&frame)
//                 {
//                     frame.text(format!("fps: {}", self.avg_fps));
//                 }

//                 frame.spacing();
//                 if imgui::CollapsingHeader::new(im_str!("Render Graph")).build(&frame) {
//                     frame.text("Frame Time");
//                     frame.separator();
//                     for (_, (system_name, usage)) in &self.percent_system_shares {
//                         frame.text(format!("{}: {}%", system_name, usage));
//                     }
//                 }

//                 frame.separator();
//                 let mouse_pos = frame.io().mouse_pos;
//                 frame.text(format!(
//                     "Mouse Position: ({:.1},{:.1})",
//                     mouse_pos[0], mouse_pos[1]
//                 ));
//             });
//     }

//     fn impl_imgui(self: Arc<Self>) -> Arc<dyn ImguiWindow> {
//         self
//     }
// }

pub struct EngineReporter {
    target: Arc<Mutex<u32>>,
    last_reported: Instant,
    frame_count: u32,
}

impl EngineReporter {
    pub fn new(target: Arc<Mutex<u32>>) -> Self {
        Self {
            target,
            last_reported: Instant::now(),
            frame_count: 0,
        }
    }

    pub fn update(&mut self) {
        self.frame_count += 1;
        if self.last_reported.elapsed() >= Duration::from_secs(1) {
            self.report();
        }
    }

    fn report(&mut self) {
        *self.target.lock().unwrap() =
            (1.0 / (self.last_reported.elapsed().as_secs_f64() / (self.frame_count as f64))) as u32
                + 1;
        self.last_reported = Instant::now();
        self.frame_count = 0;
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
    frame_count: u32,
    total_run_time: f64,
}

impl SystemReporter {
    pub fn new(target: Arc<Mutex<SystemMetrics>>) -> Self {
        Self {
            target,
            last_reported: Instant::now(),
            total_run_time: 0.0,
            frame_count: 0,
        }
    }

    // should be called every frame
    pub fn update(&mut self, run_time: f64) {
        self.total_run_time += run_time;
        self.frame_count += 1;

        if self.last_reported.elapsed() >= Duration::from_secs(1) {
            self.report();
        }
    }

    // average run time of system (seconds)
    fn report(&mut self) {
        let avg = self.total_run_time / self.frame_count as f64;
        self.last_reported = Instant::now();
        self.total_run_time = 0.0;
        self.frame_count = 0;
        self.target.lock().unwrap().avg_run_time = avg;
    }
}
