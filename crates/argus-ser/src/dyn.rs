use rustc_infer::infer::InferCtxt;

// These types are safe for dependents to use.
use crate::interner::TyInterner;

pub trait DynCtxt<'tcx>
where
  Self: 'tcx,
{
  type Static: 'static;
  type Dynamic: 'tcx;

  fn tls() -> &'static fluid_let::DynamicVariable<&'static Self::Static>;

  fn invoke_in<'a, T>(v: &'a Self::Dynamic, f: impl FnOnce() -> T) -> T
  where
    'tcx: 'a,
  {
    log::trace!(
      "Setting dynamic ctx {:?}",
      std::any::TypeId::of::<Self::Static>()
    );

    let cell = Self::tls();
    let vstat: &'static Self::Static = unsafe { std::mem::transmute(v) };

    cell.set(vstat, f)
  }

  fn access<T>(f: impl for<'a> FnOnce(&'a Self::Dynamic) -> T) -> T {
    log::trace!(
      "Accessing dynamic ctx {:?}",
      std::any::TypeId::of::<Self::Static>()
    );

    let cell = Self::tls();
    cell.get(|v_opt| {
      let v: &'static Self::Static = v_opt.expect("no dynamic context set");
      let vdyn: &Self::Dynamic = unsafe { std::mem::transmute(v) };

      f(vdyn)
    })
  }
}

// NOTE: setting the dynamic TCX should *only* happen
// before calling the serialize function, it must guarantee
// that the 'tcx lifetime is the same as that of the serialized item.
fluid_let::fluid_let! {static INFCX: &'static InferCtxt<'static>}
fluid_let::fluid_let! {static TY_BUF: &'static TyInterner<'static>}

impl<'tcx> DynCtxt<'tcx> for InferCtxt<'tcx> {
  type Static = InferCtxt<'static>;
  type Dynamic = InferCtxt<'tcx>;

  fn tls() -> &'static fluid_let::DynamicVariable<&'static Self::Static> {
    &INFCX
  }
}

impl<'tcx> DynCtxt<'tcx> for TyInterner<'tcx> {
  type Static = TyInterner<'static>;
  type Dynamic = TyInterner<'tcx>;

  fn tls() -> &'static fluid_let::DynamicVariable<&'static Self::Static> {
    &TY_BUF
  }
}
