use indexmap::IndexSet;
use napi::bindgen_prelude::*;
use napi::Env;
use napi_derive::napi;
use parking_lot::Mutex;
use triomphe::Arc;

use crate::convert::{js_to_primitive, prim_to_unknown};
use crate::types::{OffHeapSet, PrimitiveValue};

#[napi]
impl OffHeapSet {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(IndexSet::new())),
    }
  }

  #[napi]
  pub fn add<'a>(&self, this: This<'a>, value: Unknown<'_>) -> napi::Result<Object<'a>> {
    let primitive = js_to_primitive(value)?;
    self.inner.lock().insert(primitive);
    Ok(this.object)
  }

  #[napi]
  pub fn has(&self, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(value)?;
    Ok(self.inner.lock().contains(&primitive))
  }

  #[napi]
  pub fn delete(&self, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(value)?;
    Ok(self.inner.lock().shift_remove(&primitive))
  }

  #[napi]
  pub fn clear(&self) -> napi::Result<()> {
    self.inner.lock().clear();
    Ok(())
  }

  #[napi(getter)]
  pub fn size(&self) -> napi::Result<u32> {
    Ok(self.inner.lock().len() as u32)
  }

  #[napi]
  pub fn values(&self, env: Env) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let values: Vec<PrimitiveValue> = self.inner.lock().iter().cloned().collect();
    values.iter().map(|p| prim_to_unknown(raw_env, p)).collect()
  }

  // Lock is released before each callback so the callback can mutate the set without deadlock.
  #[napi]
  pub fn for_each(
    &self,
    env: Env,
    callback: Function<'_, FnArgs<(Unknown<'static>, Unknown<'static>)>, Unknown<'static>>,
  ) -> napi::Result<()> {
    let raw_env = env.raw();
    let mut next_index = 0usize;
    loop {
      let entry = {
        let guard = self.inner.lock();
        guard.get_index(next_index).cloned()
      };
      match entry {
        None => break,
        Some(primitive) => {
          let js_val = prim_to_unknown(raw_env, &primitive)?;
          callback.call(FnArgs {
            data: (js_val, js_val),
          })?;
          next_index = self
            .inner
            .lock()
            .get_index_of(&primitive)
            .map_or(next_index, |pos| pos + 1);
        }
      }
    }
    Ok(())
  }
}
