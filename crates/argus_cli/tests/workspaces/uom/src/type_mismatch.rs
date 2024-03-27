use uom::si::{f32::*, length::meter, time::second};

fn test() {
  // Setup length and time quantities using different units.
  let l1 = Length::new::<meter>(15.0);
  let t1 = Time::new::<second>(50.0);

  let error = l1 + t1;
}
