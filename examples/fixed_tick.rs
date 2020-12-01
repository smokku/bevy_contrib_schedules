use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use bevy_contrib_schedules::*;

fn main() {
    if let Err(e) = simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Error)
        .init()
    {
        println!("Failed to setup logger!\n{}", e);
    }

    App::build()
        .add_resource(ScheduleRunner::from_rate(2.0).add_system(fixed_sys))
        .add_plugins(MinimalPlugins)
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_system(schedule_runner_system)
        .run();
}

fn fixed_sys(runner: Res<ScheduleRunner>) {
    println!("game tick! {}", runner.frame_percent());
}
