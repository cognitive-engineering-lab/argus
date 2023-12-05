#[macro_export]
macro_rules! define_usize_idx {
  ($($ty:tt),*) => {
      crate::define_tsrs_alias!($($ty,)* => "number");
      $(
        index_vec::define_index_type! {
          pub struct $ty = usize;
        }
      )*
  }
}

#[macro_export]
macro_rules! define_tsrs_alias {
    ($($($ty:ty,)* => $l:literal),*) => {$($(
        impl ts_rs::TS for $ty {
            fn name() -> String {
                $l.to_owned()
            }
            fn name_with_type_args(args: Vec<String>) -> String {
                assert!(
                    args.is_empty(),
                    "called name_with_type_args on {}",
                    stringify!($ty)
                );
                $l.to_owned()
            }
            fn inline() -> String {
                $l.to_owned()
            }
            fn dependencies() -> Vec<ts_rs::Dependency> {
                vec![]
            }
            fn transparent() -> bool {
                false
            }
        }
    )*)*};
}

#[macro_export]
macro_rules! serialize_as_number {
    (PATH ( $field_path:tt ){ $($name:ident,)* }) => {
        $(
            impl serde::Serialize for $name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    let s = format!("{}", self.$field_path.as_usize());
                    serializer.serialize_str(&s)
                }
            }
        )*
    }
}
