use indexmap::IndexSet;
use napi::bindgen_prelude::*;
use napi::Env;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

use crate::convert::{js_to_primitive, lock_err, prim_to_unknown};
use crate::types::OffHeapSet;

#[napi]
impl OffHeapSet {
  /// new OffHeapSet()
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(IndexSet::new())),
    }
  }

  /// set.add(value) → returns this for chaining  (primitives only)
  #[napi(ts_return_type = "this")]
  pub fn add<'a>(
    &self,
    this: This<'a>,
    env: Env,
    value: Unknown<'_>,
  ) -> napi::Result<Object<'a>> {
    let primitive = js_to_primitive(&env, value)?;
    self.inner.lock().map_err(lock_err)?.insert(primitive);
    Ok(this.object)
  }

  /// set.has(value) → boolean
  #[napi]
  pub fn has(&self, env: Env, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(&env, value)?;
    Ok(self.inner.lock().map_err(lock_err)?.contains(&primitive))
  }

  /// set.delete(value) → boolean
  #[napi]
  pub fn delete(&self, env: Env, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(&env, value)?;
    Ok(
      self
        .inner
        .lock()
        .map_err(lock_err)?
        .shift_remove(&primitive),
    )
  }

  /// set.clear()
  #[napi]
  pub fn clear(&self) -> napi::Result<()> {
    self.inner.lock().map_err(lock_err)?.clear();
    Ok(())
  }

  /// set.size (getter)
  #[napi(getter)]
  pub fn size(&self) -> napi::Result<u32> {
    Ok(self.inner.lock().map_err(lock_err)?.len() as u32)
  }

  /// set.values() → primitive[]
  #[napi]
  pub fn values(&self, env: Env) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    guard
      .iter()
      .map(|p| prim_to_unknown(raw_env, p))
      .collect()
  }

  /// set.forEach(callback)
  ///
  /// callback receives (value, value) per the JS Set.forEach spec.
  #[napi]
  pub fn for_each(
    &self,
    env: Env,
    callback: Function<'_, FnArgs<(Unknown<'static>, Unknown<'static>)>, Unknown<'static>>,
  ) -> napi::Result<()> {
    let raw_env = env.raw();
    let mut index = 0usize;
    loop {
      let entry = {
        let guard = self.inner.lock().map_err(lock_err)?;
        guard.get_index(index).cloned()
      }; // lock released

      match entry {
        None => break,
        Some(primitive) => {
          let js_val1 = prim_to_unknown(raw_env, &primitive)?;
          let js_val2 = prim_to_unknown(raw_env, &primitive)?;
          callback.call(FnArgs { data: (js_val1, js_val2) })?;
          index += 1;
        }
      }
    }
    Ok(())
  }
}
