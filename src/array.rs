use napi::bindgen_prelude::*;
use napi::Env;
use napi_derive::napi;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::convert::{js_to_persistent, prim_to_unknown, undefined_to_unknown, val_to_unknown};
use crate::types::{OffHeapArray, OffHeapValue, PrimitiveValue};

#[napi]
impl OffHeapArray {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(Vec::new())),
    }
  }

  #[napi]
  pub fn push<'a>(&self, this: This<'a>, env: Env, value: Unknown<'_>) -> napi::Result<Object<'a>> {
    let v = js_to_persistent(&env, value)?;
    self.inner.lock().push(v);
    Ok(this.object)
  }

  #[napi]
  pub fn pop(&self, env: Env) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let val = self.inner.lock().pop();
    match val {
      None => undefined_to_unknown(raw_env),
      Some(v) => val_to_unknown(raw_env, &v),
    }
  }

  #[napi]
  pub fn get(&self, env: Env, index: u32) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let val = self.inner.lock().get(index as usize).cloned();
    match val {
      None => undefined_to_unknown(raw_env),
      Some(v) => val_to_unknown(raw_env, &v),
    }
  }

  #[napi]
  pub fn set(&self, env: Env, index: u32, value: Unknown<'_>) -> napi::Result<()> {
    let idx = index as usize;
    let mut guard = self.inner.lock();
    if idx >= guard.len() {
      return Err(napi::Error::new(
        napi::Status::GenericFailure,
        format!("index {} out of bounds (length {})", idx, guard.len()),
      ));
    }
    guard[idx] = js_to_persistent(&env, value)?;
    Ok(())
  }

  #[napi(getter)]
  pub fn length(&self) -> napi::Result<u32> {
    Ok(self.inner.lock().len() as u32)
  }

  // Items are converted before the lock is acquired to avoid holding the lock across JS calls.
  #[napi]
  pub fn splice(
    &self,
    env: Env,
    start: u32,
    delete_count: u32,
    items: Vec<Unknown<'_>>,
  ) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let new_items: Vec<OffHeapValue> = items
      .into_iter()
      .map(|v| js_to_persistent(&env, v))
      .collect::<napi::Result<_>>()?;

    let mut guard = self.inner.lock();
    let len = guard.len();
    let start = (start as usize).min(len);
    let end = (start + delete_count as usize).min(len);

    let removed: Vec<OffHeapValue> = guard.splice(start..end, new_items).collect();
    drop(guard);

    removed.iter().map(|v| val_to_unknown(raw_env, v)).collect()
  }

  // Lock is released before each callback so the callback can mutate the array without deadlock.
  #[napi]
  pub fn for_each(
    &self,
    env: Env,
    callback: Function<'_, FnArgs<(Unknown<'static>, Unknown<'static>)>, Unknown<'static>>,
  ) -> napi::Result<()> {
    let raw_env = env.raw();
    // Capture length upfront: matches JS Array.prototype.forEach which does not
    // visit elements pushed after the iteration starts.
    let initial_length = self.inner.lock().len();
    for index in 0..initial_length {
      let val = self.inner.lock().get(index).cloned();
      if let Some(val) = val {
        let js_val = val_to_unknown(raw_env, &val)?;
        let idx_u32 = u32::try_from(index).unwrap_or(u32::MAX);
        let js_idx = prim_to_unknown(raw_env, &PrimitiveValue::Int(idx_u32 as i64))?;
        callback.call(FnArgs {
          data: (js_val, js_idx),
        })?;
      }
    }
    Ok(())
  }
}
