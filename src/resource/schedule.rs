use std::{marker::PhantomData, sync::Arc};

use legion::systems::{Builder as ScheduleBuilder, ParallelRunnable};

pub enum Step {
    System(Arc<Box<dyn Schedulable>>),
    Flush,
}

pub struct SubSchedule {
    pub steps: Vec<Step>,
}

impl SubSchedule {
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    pub fn add_system<F: Fn() -> S + Send + Sync + 'static, S: ParallelRunnable + 'static>(
        &mut self,
        system: NodeSystem<F, S>,
    ) {
        self.steps.push(Step::System(Arc::new(Box::new(system))));
    }

    pub fn add_boxed(&mut self, system: Arc<Box<dyn Schedulable>>) {
        self.steps.push(Step::System(system));
    }

    pub fn flush(&mut self) {
        self.steps.push(Step::Flush);
    }
}

pub trait Schedulable: Send + Sync {
    fn schedule(&self, schedule: &mut ScheduleBuilder);
}

impl Schedulable for SubSchedule {
    fn schedule(&self, schedule: &mut ScheduleBuilder) {
        for step in &self.steps {
            match step {
                Step::Flush => {
                    schedule.flush();
                }
                Step::System(builder) => builder.schedule(schedule),
            }
        }
    }
}

pub struct NodeSystem<F, S>
where
    F: Fn() -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    builder: F,
    _marker: PhantomData<S>,
}

impl<F, S> NodeSystem<F, S>
where
    F: Fn() -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    pub fn new(system_builder: F) -> Self {
        Self {
            builder: system_builder,
            _marker: PhantomData,
        }
    }
}

impl<F, S> Schedulable for NodeSystem<F, S>
where
    F: Fn() -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    fn schedule(&self, schedule: &mut ScheduleBuilder) {
        schedule.add_system((self.builder)());
    }
}
