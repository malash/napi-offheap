#![deny(clippy::all)]

use indexmap::{IndexMap, IndexSet};
use napi::bindgen_prelude::*;
use napi::{sys, Env, ValueType};
use napi_derive::napi;
use ordered_float::OrderedFloat;
use std::sync::{Arc, Mutex};

// ─── Primitive value ──────────────────────────────────────────────────────────

/// Can be stored directly in Set / used as Map value; must be Hash + Eq.
/// f64 is wrapped in OrderedFloat so it satisfies those bounds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveValue {
  Null,
  Undefined,
  Bool(bool),
  Int(i64),
  Float(OrderedFloat<f64>),
  Str(Arc<str>),
}

// ─── General storable value ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PersistentValue {
  Primitive(PrimitiveValue),
  Map(Arc<Mutex<SharedMap>>),
  Array(Arc<Mutex<SharedArray>>),
  Set(Arc<Mutex<SharedSet>>),
}

/// IndexMap preserves insertion order, matching JS Map semantics.
pub type SharedMap = IndexMap<String, PersistentValue>;
/// Plain Vec.
pub type SharedArray = Vec<PersistentValue>;
/// IndexSet preserves insertion order, matching JS Set semantics.
/// Elements are limited to PrimitiveValue because object identity hash is unstable.
pub type SharedSet = IndexSet<PrimitiveValue>;

// ─── napi class shells ────────────────────────────────────────────────────────

#[napi]
pub struct PersistentMap {
  inner: Arc<Mutex<SharedMap>>,
}

#[napi]
pub struct PersistentArray {
  inner: Arc<Mutex<SharedArray>>,
}

#[napi]
pub struct PersistentSet {
  inner: Arc<Mutex<SharedSet>>,
}

// ─── Helper: uniform lock error ───────────────────────────────────────────────

fn lock_err(e: impl std::fmt::Display) -> napi::Error {
  napi::Error::new(
    napi::Status::GenericFailure,
    format!("lock poisoned: {e}"),
  )
}

// ─── JS → Rust type conversion ────────────────────────────────────────────────

/// Convert a JS value to PersistentValue.
/// Accepts PersistentMap | PersistentArray | PersistentSet | primitives.
/// Rejects plain JS objects, functions, Symbols, etc.
fn js_to_persistent(env: &Env, val: Unknown<'_>) -> napi::Result<PersistentValue> {
  if val.get_type()? == ValueType::Object {
    let raw_env = env.raw();
    let raw_val = val.value().value;

    if PersistentMap::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, PersistentMap>::from_napi_value(raw_env, raw_val)? };
      return Ok(PersistentValue::Map(Arc::clone(&instance.inner)));
    }
    if PersistentArray::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, PersistentArray>::from_napi_value(raw_env, raw_val)? };
      return Ok(PersistentValue::Array(Arc::clone(&instance.inner)));
    }
    if PersistentSet::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, PersistentSet>::from_napi_value(raw_env, raw_val)? };
      return Ok(PersistentValue::Set(Arc::clone(&instance.inner)));
    }

    return Err(napi::Error::new(
      napi::Status::InvalidArg,
      "plain JS objects are not accepted; wrap with PersistentMap/Array/Set",
    ));
  }
  Ok(PersistentValue::Primitive(js_to_primitive(val)?))
}

/// Convert a JS primitive to PrimitiveValue.
fn js_to_primitive(val: Unknown<'_>) -> napi::Result<PrimitiveValue> {
  let v = val.value();
  match val.get_type()? {
    ValueType::Null => Ok(PrimitiveValue::Null),
    ValueType::Undefined => Ok(PrimitiveValue::Undefined),
    ValueType::Boolean => {
      let b = unsafe { bool::from_napi_value(v.env, v.value)? };
      Ok(PrimitiveValue::Bool(b))
    }
    ValueType::Number => {
      let n = unsafe { f64::from_napi_value(v.env, v.value)? };
      // Prefer integer representation when the number is a whole number.
      if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        Ok(PrimitiveValue::Int(n as i64))
      } else {
        Ok(PrimitiveValue::Float(OrderedFloat(n)))
      }
    }
    ValueType::String => {
      let s = unsafe { String::from_napi_value(v.env, v.value)? };
      Ok(PrimitiveValue::Str(Arc::from(s.as_str())))
    }
    _ => Err(napi::Error::new(
      napi::Status::InvalidArg,
      "value must be a primitive or a Persistent type",
    )),
  }
}

// ─── Rust → JS type conversion (raw-pointer level) ───────────────────────────
//
// Helper functions work with the raw `sys::napi_env` to avoid tying the
// returned JS values to any local `Env` borrow, which would prevent them from
// being returned from `#[napi]` methods where `Env` is owned.

/// Convert PersistentValue to a raw napi_value.
/// For container types, a new thin JS shell is created that shares the same Arc.
fn to_napi_value_inner(
  raw_env: sys::napi_env,
  val: &PersistentValue,
) -> napi::Result<sys::napi_value> {
  match val {
    PersistentValue::Primitive(p) => primitive_to_napi(raw_env, p),
    PersistentValue::Map(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = PersistentMap {
        inner: Arc::clone(arc),
      }
      .into_instance(&env)?;
      Ok(instance.value)
    }
    PersistentValue::Array(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = PersistentArray {
        inner: Arc::clone(arc),
      }
      .into_instance(&env)?;
      Ok(instance.value)
    }
    PersistentValue::Set(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = PersistentSet {
        inner: Arc::clone(arc),
      }
      .into_instance(&env)?;
      Ok(instance.value)
    }
  }
}

/// Convert PrimitiveValue to a raw napi_value.
fn primitive_to_napi(raw_env: sys::napi_env, val: &PrimitiveValue) -> napi::Result<sys::napi_value> {
  unsafe {
    match val {
      PrimitiveValue::Null => Null::to_napi_value(raw_env, Null),
      PrimitiveValue::Undefined => <()>::to_napi_value(raw_env, ()),
      PrimitiveValue::Bool(b) => bool::to_napi_value(raw_env, *b),
      PrimitiveValue::Int(i) => i64::to_napi_value(raw_env, *i),
      PrimitiveValue::Float(f) => f64::to_napi_value(raw_env, f.0),
      PrimitiveValue::Str(s) => String::to_napi_value(raw_env, s.to_string()),
    }
  }
}

/// Convenience: returns an `Unknown<'_>` from raw parts.
/// # Safety
/// The caller must ensure the napi_value is valid for the intended lifetime.
#[inline]
unsafe fn to_unknown(raw_env: sys::napi_env, raw_val: sys::napi_value) -> Unknown<'static> {
  Unknown::from_raw_unchecked(raw_env, raw_val)
}

/// PersistentValue → Unknown<'static>  (lifetime erased; valid within the napi scope)
fn val_to_unknown(raw_env: sys::napi_env, val: &PersistentValue) -> napi::Result<Unknown<'static>> {
  let raw = to_napi_value_inner(raw_env, val)?;
  Ok(unsafe { to_unknown(raw_env, raw) })
}

/// PrimitiveValue → Unknown<'static>
fn prim_to_unknown(raw_env: sys::napi_env, val: &PrimitiveValue) -> napi::Result<Unknown<'static>> {
  let raw = primitive_to_napi(raw_env, val)?;
  Ok(unsafe { to_unknown(raw_env, raw) })
}

// ─── PersistentMap ────────────────────────────────────────────────────────────

#[napi]
impl PersistentMap {
  /// new PersistentMap()
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(IndexMap::new())),
    }
  }

  /// map.set(key, value) → returns this for chaining
  #[napi(ts_return_type = "this")]
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

  /// map.get(key) → PersistentXxx | primitive | undefined
  #[napi]
  pub fn get(&self, env: Env, key: String) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    match guard.get(&key) {
      None => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, v),
    }
  }

  /// map.has(key) → boolean
  #[napi]
  pub fn has(&self, key: String) -> napi::Result<bool> {
    Ok(
      self
        .inner
        .lock()
        .map_err(lock_err)?
        .contains_key(&key),
    )
  }

  /// map.delete(key) → boolean
  #[napi]
  pub fn delete(&self, key: String) -> napi::Result<bool> {
    Ok(
      self
        .inner
        .lock()
        .map_err(lock_err)?
        .shift_remove(&key)
        .is_some(),
    )
  }

  /// map.clear()
  #[napi]
  pub fn clear(&self) -> napi::Result<()> {
    self.inner.lock().map_err(lock_err)?.clear();
    Ok(())
  }

  /// map.size (getter)
  #[napi(getter)]
  pub fn size(&self) -> napi::Result<u32> {
    Ok(self.inner.lock().map_err(lock_err)?.len() as u32)
  }

  /// map.keys() → string[]
  #[napi]
  pub fn keys(&self) -> napi::Result<Vec<String>> {
    Ok(
      self
        .inner
        .lock()
        .map_err(lock_err)?
        .keys()
        .cloned()
        .collect(),
    )
  }

  /// map.values() → unknown[]
  #[napi]
  pub fn values(&self, env: Env) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    guard
      .values()
      .map(|v| val_to_unknown(raw_env, v))
      .collect()
  }

  /// map.entries() → [string, unknown][]
  #[napi(ts_return_type = "[string, unknown][]")]
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

  /// map.forEach(callback)
  ///
  /// Live iteration: releases the lock before each callback invocation so that
  /// the callback can safely mutate the same map. Uses positional index (IndexMap)
  /// to advance the cursor, consistent with the JS Map.forEach spec for additions
  /// and deletions that happen after the current key.
  #[napi]
  pub fn for_each(
    &self,
    env: Env,
    callback: Function<'_, FnArgs<(Unknown<'static>, Unknown<'static>)>, Unknown<'static>>,
  ) -> napi::Result<()> {
    let raw_env = env.raw();
    let mut index = 0usize;
    loop {
      // Hold the lock only long enough to snapshot the current entry.
      let entry = {
        let guard = self.inner.lock().map_err(lock_err)?;
        guard
          .get_index(index)
          .map(|(k, v)| (k.clone(), v.clone()))
      }; // lock released here

      match entry {
        None => break,
        Some((key, val)) => {
          let js_val = val_to_unknown(raw_env, &val)?;
          let js_key =
            unsafe { to_unknown(raw_env, String::to_napi_value(raw_env, key)?) };
          // Lock is free; callback can mutate the map without deadlock.
          callback.call(FnArgs { data: (js_val, js_key) })?;
          index += 1;
        }
      }
    }
    Ok(())
  }
}

// ─── PersistentArray ──────────────────────────────────────────────────────────

#[napi]
impl PersistentArray {
  /// new PersistentArray()
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(Vec::new())),
    }
  }

  /// arr.push(value) → returns this for chaining
  #[napi(ts_return_type = "this")]
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

  /// arr.pop() → last element | undefined
  #[napi]
  pub fn pop(&self, env: Env) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let mut guard = self.inner.lock().map_err(lock_err)?;
    match guard.pop() {
      None => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, &v),
    }
  }

  /// arr.get(index) → element | undefined
  #[napi]
  pub fn get(&self, env: Env, index: u32) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    match guard.get(index as usize) {
      None => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, v),
    }
  }

  /// arr.set(index, value)  — throws if index is out of bounds
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

  /// arr.length (getter)
  #[napi(getter)]
  pub fn length(&self) -> napi::Result<u32> {
    Ok(self.inner.lock().map_err(lock_err)?.len() as u32)
  }

  /// arr.splice(start, deleteCount, ...items) → removed elements
  ///
  /// Items are converted before the lock is acquired to avoid holding the lock
  /// across JS calls.
  #[napi]
  pub fn splice(
    &self,
    env: Env,
    start: u32,
    delete_count: u32,
    items: Vec<Unknown<'_>>,
  ) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    // Convert all insertion items before touching the lock.
    let new_items: Vec<PersistentValue> = items
      .into_iter()
      .map(|v| js_to_persistent(&env, v))
      .collect::<napi::Result<_>>()?;

    let mut guard = self.inner.lock().map_err(lock_err)?;
    let len = guard.len();
    let start = (start as usize).min(len);
    let end = (start + delete_count as usize).min(len);

    let removed: Vec<PersistentValue> = guard.drain(start..end).collect();
    for (offset, item) in new_items.into_iter().enumerate() {
      guard.insert(start + offset, item);
    }
    drop(guard); // release lock before JS conversions

    removed
      .iter()
      .map(|v| val_to_unknown(raw_env, v))
      .collect()
  }

  /// arr.forEach(callback)
  ///
  /// Live iteration with the same lock-release-per-step semantics as PersistentMap.
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
      }; // lock released

      match entry {
        None => break,
        Some(val) => {
          let js_val = val_to_unknown(raw_env, &val)?;
          let js_idx = unsafe {
            to_unknown(raw_env, u32::to_napi_value(raw_env, index as u32)?)
          };
          callback.call(FnArgs { data: (js_val, js_idx) })?;
          index += 1;
        }
      }
    }
    Ok(())
  }
}

// ─── PersistentSet ────────────────────────────────────────────────────────────

#[napi]
impl PersistentSet {
  /// new PersistentSet()
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
    value: Unknown<'_>,
  ) -> napi::Result<Object<'a>> {
    let primitive = js_to_primitive(value)?;
    self.inner.lock().map_err(lock_err)?.insert(primitive);
    Ok(this.object)
  }

  /// set.has(value) → boolean
  #[napi]
  pub fn has(&self, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(value)?;
    Ok(self.inner.lock().map_err(lock_err)?.contains(&primitive))
  }

  /// set.delete(value) → boolean
  #[napi]
  pub fn delete(&self, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(value)?;
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
