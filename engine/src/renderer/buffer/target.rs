use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::renderer::graph::target::RenderTarget;

pub struct TargetBuffer {
    pub targets: HashMap<Uuid, Arc<Mutex<RenderTarget>>>,
    pub master: Uuid,
}

impl TargetBuffer {
    pub fn new(targets: HashMap<Uuid, Arc<Mutex<RenderTarget>>>, master: Uuid) -> TargetBuffer {
        TargetBuffer { targets, master }
    }

    pub fn get(&self, uuid: &Uuid) -> Arc<Mutex<RenderTarget>> {
        Arc::clone(&self.targets[uuid])
    }

    pub fn master(&self) -> Arc<Mutex<RenderTarget>> {
        Arc::clone(&self.targets[&self.master])
    }
}
