use std::{marker::PhantomData, sync::Arc};

use legion::systems::{Builder as ScheduleBuilder, ParallelRunnable, Runnable};

use crate::render::graph::NodeState;

use super::metrics::SystemReporter;

pub enum Step {
    Stateless {
        builder: Arc<Box<dyn Schedulable>>,
    },
    System {
        builder: Arc<Box<dyn SubSchedulable>>,
        state: NodeState,
    },
    Local {
        builder: Arc<Box<dyn LocalSchedulable>>,
    },
    LocalReporter {
        builder: Arc<Box<dyn LocalReporterSchedulable>>,
        state: SystemReporter,
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

    pub fn add_single_threaded(&mut self, system: Arc<Box<dyn LocalSchedulable>>) {
        self.steps.push(Step::Local { builder: system });
    }

    pub fn add_single_threaded_reporter(
        &mut self,
        system: Arc<Box<dyn LocalReporterSchedulable>>,
        state: SystemReporter,
    ) {
        self.steps.push(Step::LocalReporter {
            builder: system,
            state,
        });
    }

    pub fn flush(&mut self) {
        self.steps.push(Step::Flush);
    }
}

pub trait Schedulable {
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
                Step::Local { builder } => builder.schedule(schedule),
                Step::LocalReporter { builder, state } => builder.schedule(schedule, state.clone()),
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

// For async systems with no state

pub struct StatelessSystem<F, S>
where
    F: Fn() -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    builder: F,
    _marker: PhantomData<S>,
}

impl<F, S> StatelessSystem<F, S>
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

impl<F, S> Schedulable for StatelessSystem<F, S>
where
    F: Fn() -> S + Send + Sync,
    S: ParallelRunnable + 'static,
{
    fn schedule(&self, schedule: &mut ScheduleBuilder) {
        schedule.add_system((self.builder)());
    }
}

pub trait LocalSchedulable {
    fn schedule(&self, schedule: &mut ScheduleBuilder);
}

pub struct LocalSystem<F, S>
where
    F: Fn() -> S,
    S: Runnable + 'static,
{
    builder: F,
    _marker: PhantomData<S>,
}

impl<F, S> LocalSystem<F, S>
where
    F: Fn() -> S,
    S: Runnable + 'static,
{
    pub fn new(system_builder: F) -> Self {
        Self {
            builder: system_builder,
            _marker: PhantomData,
        }
    }
}

impl<F, S> LocalSchedulable for LocalSystem<F, S>
where
    F: Fn() -> S,
    S: Runnable + 'static,
{
    fn schedule(&self, schedule: &mut ScheduleBuilder) {
        schedule.add_thread_local((self.builder)());
    }
}

pub struct LocalReporterSystem<F, S>
where
    F: Fn(SystemReporter) -> S,
    S: Runnable + 'static,
{
    builder: F,
    _marker: PhantomData<S>,
}

impl<F, S> LocalReporterSystem<F, S>
where
    F: Fn(SystemReporter) -> S,
    S: Runnable + 'static,
{
    pub fn new(system_builder: F) -> Self {
        Self {
            builder: system_builder,
            _marker: PhantomData,
        }
    }
}

pub trait LocalReporterSchedulable {
    fn schedule(&self, schedule: &mut ScheduleBuilder, state: SystemReporter);
}

impl<F, S> LocalReporterSchedulable for LocalReporterSystem<F, S>
where
    F: Fn(SystemReporter) -> S,
    S: Runnable + 'static,
{
    fn schedule(&self, schedule: &mut ScheduleBuilder, state: SystemReporter) {
        schedule.add_thread_local((self.builder)(state));
    }
}