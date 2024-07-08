use bevy::prelude::*;

#[derive(Resource)]
struct Timer(usize);

fn run_timer(_: Timer) {}

fn main() {
  App::new().add_systems(Update, run_timer).run();
}
