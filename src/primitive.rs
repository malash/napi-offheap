use napi::bindgen_prelude::*;
use napi::Env;
use napi_derive::napi;

use crate::convert::{js_to_primitive, prim_to_unknown};
use crate::types::OffHeapPrimitive;

#[napi]
impl OffHeapPrimitive {
  /// new OffHeapPrimitive(value)
  #[napi(constructor)]
  pub fn new(env: Env, value: Unknown<'_>) -> napi::Result<Self> {
    Ok(Self { inner: js_to_primitive(&env, value)? })
  }

  /// prim.value (getter) → JS primitive
  #[napi(getter)]
  pub fn value(&self, env: Env) -> napi::Result<Unknown<'static>> {
    prim_to_unknown(env.raw(), &self.inner)
  }
}
