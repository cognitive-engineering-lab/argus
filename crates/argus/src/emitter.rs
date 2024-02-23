use std::io;

use rustc_driver::DEFAULT_LOCALE_RESOURCES;
use rustc_errors::{
  self,
  emitter::{DynEmitter, HumanEmitter},
  fallback_fluent_bundle,
};

pub struct SilentEmitter;

impl SilentEmitter {
  pub fn boxed() -> Box<DynEmitter> {
    // Create a new emitter writer which consumes *silently* all
    // errors. There most certainly is a *better* way to do this,
    // if you, the reader, know what that is, please open an issue :)
    let fallback_bundle =
      fallback_fluent_bundle(DEFAULT_LOCALE_RESOURCES.to_vec(), false);
    let emitter = HumanEmitter::new(Box::new(io::sink()), fallback_bundle);
    Box::new(emitter)
  }
}
