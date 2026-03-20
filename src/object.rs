use indexmap::IndexMap;
use napi::bindgen_prelude::*;
use napi::Env;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

use crate::convert::{js_to_persistent, lock_err, to_unknown, val_to_unknown};
use crate::types::OffHeapObject;

#[napi]
impl OffHeapObject {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(IndexMap::new())),
    }
  }

  #[napi]
  pub fn set<'a>(
    &self,
    this: This<'a>,
    env: Env,
    key: String,
    value: Unknown<'_>,
  ) -> napi::Result<Object<'a>> {
    let v = js_to_persistent(&env, value)?;
    self.inner.lock().map_err(lock_err)?.insert(key, v);
    Ok(this.object)
  }

  #[napi]
  pub fn get(&self, env: Env, key: String) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    match guard.get(&key) {
      None => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, v),
    }
  }

  #[napi]
  pub fn has(&self, key: String) -> napi::Result<bool> {
    Ok(self.inner.lock().map_err(lock_err)?.contains_key(&key))
  }

  #[napi]
  pub fn delete(&self, key: String) -> napi::Result<bool> {
    Ok(self.inner.lock().map_err(lock_err)?.shift_remove(&key).is_some())
  }

  #[napi]
  pub fn clear(&self) -> napi::Result<()> {
    self.inner.lock().map_err(lock_err)?.clear();
    Ok(())
  }

  #[napi(getter)]
  pub fn size(&self) -> napi::Result<u32> {
    Ok(self.inner.lock().map_err(lock_err)?.len() as u32)
  }

  #[napi]
  pub fn keys(&self) -> napi::Result<Vec<String>> {
    Ok(self.inner.lock().map_err(lock_err)?.keys().cloned().collect())
  }

  #[napi]
  pub fn values(&self, env: Env) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    guard.values().map(|v| val_to_unknown(raw_env, v)).collect()
  }

  #[napi]
  pub fn entries(&self, env: Env) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    guard
      .iter()
      .map(|(k, v)| {
        let raw_key = unsafe { String::to_napi_value(raw_env, k.clone())? };
        let js_key = unsafe { to_unknown(raw_env, raw_key) };
        let js_val = val_to_unknown(raw_env, v)?;
        let env_obj = Env::from_raw(raw_env);
        let arr = Array::from_vec(&env_obj, vec![js_key, js_val])?;
        Ok(unsafe { to_unknown(raw_env, arr.raw()) })
      })
      .collect()
  }

  // Lock is released before each callback so the callback can mutate the object without deadlock.
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
        guard.get_index(index).map(|(k, v)| (k.clone(), v.clone()))
      };
      match entry {
        None => break,
        Some((key, val)) => {
          let js_val = val_to_unknown(raw_env, &val)?;
          let js_key = unsafe { to_unknown(raw_env, String::to_napi_value(raw_env, key)?) };
          callback.call(FnArgs { data: (js_val, js_key) })?;
          index += 1;
        }
      }
    }
    Ok(())
  }
}
