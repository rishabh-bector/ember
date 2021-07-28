use std::{marker::PhantomData, sync::Arc};

use legion::systems::{Builder as ScheduleBuilder, ParallelRunnable};

use crate::render::graph::NodeState;

pub enum Step {
    Stateless {
        builder: Arc<Box<dyn Schedulable>>,
    },
    System {
        builder: Arc<Box<dyn SubSchedulable>>,
        state: NodeState,
    },
    Flush,
}

pub struct SubSchedule {
    pub steps: Vec<Step>,
}

impl SubSchedule {
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    pub fn add_system<
        F: Fn(NodeState) -> S + Send + Sync + 'static,
        S: ParallelRunnable + 'static,
    >(
        &mut self,
        system: NodeSystem<F, S>,
        state: NodeState,
    ) {
        self.steps.push(Step::System {
            builder: Arc::new(Box::new(system)),
            state,
        });
    }

    pub fn add_boxed(&mut self, system: Arc<Box<dyn SubSchedulable>>, state: NodeState) {
        self.steps.push(Step::System {
            builder: system,
            state,
        });
    }

    pub fn add_boxed_stateless(&mut self, system: Arc<Box<dyn Schedulable>>) {
        self.steps.push(Step::Stateless { builder: system });
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
                Step::System { builder, state } => builder.schedule(schedule, state.clone()),
                Step::Stateless { builder } => builder.schedule(schedule),
            }
        }
    }
}

pub struct NodeSystem<F, S>
where
    F: Fn(NodeState) -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    builder: F,
    _marker: PhantomData<S>,
}

impl<F, S> NodeSystem<F, S>
where
    F: Fn(NodeState) -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    pub fn new(system_builder: F) -> Self {
        Self {
            builder: system_builder,
            _marker: PhantomData,
        }
    }
}

pub trait SubSchedulable: Send + Sync {
    fn schedule(&self, schedule: &mut ScheduleBuilder, state: NodeState);
}

impl<F, S> SubSchedulable for NodeSystem<F, S>
where
    F: Fn(NodeState) -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    fn schedule(&self, schedule: &mut ScheduleBuilder, state: NodeState) {
        schedule.add_system((self.builder)(state));
    }
}

// For systems with no state

pub struct PlainSystem<F, S>
where
    F: Fn() -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    builder: F,
    _marker: PhantomData<S>,
}

impl<F, S> PlainSystem<F, S>
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

impl<F, S> Schedulable for PlainSystem<F, S>
where
    F: Fn() -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    fn schedule(&self, schedule: &mut ScheduleBuilder) {
        schedule.add_system((self.builder)());
    }
}
