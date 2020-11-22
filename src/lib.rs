use bevy::{
    app::stage,
    core::Time,
    ecs::{Entity, IntoSystem, ParallelExecutor, Resources, Schedule, System, World},
    utils::HashMap,
};
use std::ops::{Deref, DerefMut};

/// Determines how the schedule should run
#[derive(Debug, Copy, Clone)]
pub enum ScheduleType {
    // The Schedule runs with...
    // ... Every frame
    Always,
    // ... A fixed tick cycle
    Fixed(f64, f64), // (rate, accumulator)

                     // TODO: Figure out how to make this more useful?
                     // ... A user-provided fn
                     // With(Box<dyn FnMut(&mut PackedSchedule, &mut World, &mut Resources) + Send + Sync>),
}

/// The PackedSchedule is responsible for actual execution
/// You probably won't need to touch this directly
#[derive(Debug)]
pub struct PackedSchedule(pub ScheduleType, pub Schedule, ParallelExecutor);

impl Default for PackedSchedule {
    fn default() -> Self {
        PackedSchedule(
            ScheduleType::Always,
            Default::default(),
            ParallelExecutor::without_tracker_clears(),
        )
    }
}

impl PackedSchedule {
    fn run(&mut self, mut world: &mut World, mut resources: &mut Resources) {
        self.1.initialize(world, resources);

        match &mut self.0 {
            ScheduleType::Always => {
                self.2.run(&mut self.1, &mut world, &mut resources);
            }
            ScheduleType::Fixed(rate, accumulator) => {
                // Accumulate time
                match resources.get::<Time>() {
                    Some(time) => {
                        *accumulator += time.delta_seconds_f64;
                    }
                    None => panic!("Time does not exist, Fixed Schedule cannot run!"),
                };

                // Run fixed-interval ticks
                while accumulator >= rate {
                    self.2.run(&mut self.1, &mut world, &mut resources);
                    *accumulator -= *rate;
                }
            }
        };
    }

    fn get_dummy(&self) -> Self {
        PackedSchedule {
            0: self.0,
            ..Default::default()
        }
    }

    fn frame_percent(&self) -> f64 {
        match self.0 {
            ScheduleType::Always => 1.0,
            ScheduleType::Fixed(rate, accumulator) => {
                f64::min(1.0, f64::max(0.0, accumulator / rate))
            }
        }
    }
}

/// Responsible for holding the data in Bevy
/// Use as a Resource or Component
#[derive(Debug)]
pub struct ScheduleRunner(pub PackedSchedule);

impl Default for ScheduleRunner {
    fn default() -> Self {
        ScheduleRunner(PackedSchedule {
            0: ScheduleType::Always,
            ..Default::default()
        })
        .add_default_stages()
    }
}

/// Portion taken from bevy::AppBuilder for convenience
impl ScheduleRunner {
    /// A fixed-rate runner that runs every `rate` seconds
    pub fn from_rate(rate: f64) -> Self {
        ScheduleRunner(PackedSchedule {
            0: ScheduleType::Fixed(rate, 0.0),
            ..Default::default()
        })
        .add_default_stages()
    }

    /// A fixed-rate runner that runs `rate` per second
    pub fn from_rate_inv(rate: f64) -> Self {
        Self::from_rate(1.0 / rate)
    }

    // TODO: Figure out how we should support this stuff
    // A runner executed by a user-provided fn
    // pub fn from_fn<F>(f: F) -> Self
    // where F: FnMut(&mut PackedSchedule, &mut World, &mut Resources) + Send + Sync + 'static {
    //     ScheduleRunner(PackedSchedule { 0: ScheduleType::With(Box::new(f)) , .. Default::default() })
    // }

    pub fn add_default_stages(self) -> Self {
        self.add_stage(stage::FIRST)
            .add_stage(stage::PRE_EVENT)
            .add_stage(stage::EVENT)
            .add_stage(stage::PRE_UPDATE)
            .add_stage(stage::UPDATE)
            .add_stage(stage::POST_UPDATE)
            .add_stage(stage::LAST)
    }

    pub fn add_stage(mut self, stage_name: &'static str) -> Self {
        self.0 .1.add_stage(stage_name);
        self
    }

    pub fn add_system<S, Params, IntoS>(mut self, system: IntoS) -> Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.0 .1.add_system_to_stage(stage::UPDATE, system);
        self
    }

    pub fn frame_percent(&self) -> f64 {
        self.0.frame_percent()
    }
}

/// Deref implementation to pass systems handling to internal schedule
impl Deref for ScheduleRunner {
    type Target = Schedule;
    fn deref(&self) -> &Self::Target {
        &self.0 .1
    }
}

impl DerefMut for ScheduleRunner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0 .1
    }
}

/// System responsible for executing all schedules
/// You should add it to your AppBuilder or parent schedule manually
pub fn schedule_runner_system(mut world: &mut World, mut resources: &mut Resources) {
    // Run it as a resource
    if resources.contains::<ScheduleRunner>() {
        // rip and tear
        let mut schedule = {
            let schedule = &mut resources.get_mut::<ScheduleRunner>().unwrap().0;
            std::mem::replace(schedule, schedule.get_dummy())
        };
        schedule.run(&mut world, &mut resources);
        resources.get_mut::<ScheduleRunner>().unwrap().0 = schedule;
    }

    // Run it as a component
    // We take all components, run them, put them back
    let mut entity_map: HashMap<Entity, PackedSchedule> = world
        .query_mut::<(Entity, &mut ScheduleRunner)>()
        .map(|(entity, mut runner)| {
            let replacement = runner.0.get_dummy();
            (entity, std::mem::replace(&mut runner.0, replacement))
        })
        .collect();
    for (_, schedule) in entity_map.iter_mut() {
        schedule.run(&mut world, &mut resources);
    }
    for (entity, mut runner) in &mut world.query_mut::<(Entity, &mut ScheduleRunner)>() {
        runner.0 = entity_map.remove(&entity).unwrap();
    }
}
