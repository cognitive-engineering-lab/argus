use bevy::prelude::*;

#[derive(Resource)]
struct Timer(usize);

fn startup_system(_: Timer) {}

fn main() {
  App::new().add_startup_system(startup_system).run();
}
