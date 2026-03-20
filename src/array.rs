use napi::bindgen_prelude::*;
use napi::Env;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

use crate::convert::{js_to_persistent, lock_err, to_unknown, val_to_unknown};
use crate::types::{OffHeapArray, OffHeapValue};

#[napi]
impl OffHeapArray {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(Vec::new())),
    }
  }

  #[napi]
  pub fn push<'a>(
    &self,
    this: This<'a>,
    env: Env,
    value: Unknown<'_>,
  ) -> napi::Result<Object<'a>> {
    let v = js_to_persistent(&env, value)?;
    self.inner.lock().map_err(lock_err)?.push(v);
    Ok(this.object)
  }

  #[napi]
  pub fn pop(&self, env: Env) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let mut guard = self.inner.lock().map_err(lock_err)?;
    match guard.pop() {
      None => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, &v),
    }
  }

  #[napi]
  pub fn get(&self, env: Env, index: u32) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    match guard.get(index as usize) {
      None => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, v),
    }
  }

  #[napi]
  pub fn set(&self, env: Env, index: u32, value: Unknown<'_>) -> napi::Result<()> {
    let v = js_to_persistent(&env, value)?;
    let mut guard = self.inner.lock().map_err(lock_err)?;
    let idx = index as usize;
    if idx >= guard.len() {
      return Err(napi::Error::new(
        napi::Status::GenericFailure,
        format!("index {} out of bounds (length {})", idx, guard.len()),
      ));
    }
    guard[idx] = v;
    Ok(())
  }

  #[napi(getter)]
  pub fn length(&self) -> napi::Result<u32> {
    Ok(self.inner.lock().map_err(lock_err)?.len() as u32)
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

    let mut guard = self.inner.lock().map_err(lock_err)?;
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
    let mut index = 0usize;
    loop {
      let entry = {
        let guard = self.inner.lock().map_err(lock_err)?;
        guard.get(index).cloned()
      };
      match entry {
        None => break,
        Some(val) => {
          let js_val = val_to_unknown(raw_env, &val)?;
          let js_idx =
            unsafe { to_unknown(raw_env, u32::to_napi_value(raw_env, index as u32)?) };
          callback.call(FnArgs { data: (js_val, js_idx) })?;
          index += 1;
        }
      }
    }
    Ok(())
  }
}
