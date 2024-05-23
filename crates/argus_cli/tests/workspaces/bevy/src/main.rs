use bevy::prelude::*;

#[derive(Resource)]
struct Timer(usize);

fn startup_system(_: Timer) {}

fn main() {
  App::new().add_systems(Startup, startup_system).run();
}
