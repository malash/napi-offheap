use napi::bindgen_prelude::*;
use napi::{sys, Env, ValueType};
use ordered_float::OrderedFloat;
use std::sync::Arc;

use crate::types::{
  OffHeapArray, OffHeapMap, OffHeapPrimitive, OffHeapSet, OffHeapValue, PrimitiveValue,
};

// ─── Helper: uniform lock error ───────────────────────────────────────────────

pub(crate) fn lock_err(e: impl std::fmt::Display) -> napi::Error {
  napi::Error::new(
    napi::Status::GenericFailure,
    format!("lock poisoned: {e}"),
  )
}

// ─── JS → Rust type conversion ────────────────────────────────────────────────

/// Convert a JS value to OffHeapValue.
/// Accepts OffHeapMap | OffHeapArray | OffHeapSet | OffHeapPrimitive | primitives.
/// Rejects plain JS objects, functions, Symbols, etc.
pub(crate) fn js_to_persistent(env: &Env, val: Unknown<'_>) -> napi::Result<OffHeapValue> {
  if val.get_type()? == ValueType::Object {
    let raw_env = env.raw();
    let raw_val = val.value().value;

    if OffHeapMap::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, OffHeapMap>::from_napi_value(raw_env, raw_val)? };
      return Ok(OffHeapValue::Map(Arc::clone(&instance.inner)));
    }
    if OffHeapArray::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, OffHeapArray>::from_napi_value(raw_env, raw_val)? };
      return Ok(OffHeapValue::Array(Arc::clone(&instance.inner)));
    }
    if OffHeapSet::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, OffHeapSet>::from_napi_value(raw_env, raw_val)? };
      return Ok(OffHeapValue::Set(Arc::clone(&instance.inner)));
    }
    if OffHeapPrimitive::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, OffHeapPrimitive>::from_napi_value(raw_env, raw_val)? };
      return Ok(OffHeapValue::Primitive(instance.inner.clone()));
    }

    return Err(napi::Error::new(
      napi::Status::InvalidArg,
      "plain JS objects are not accepted; wrap with OffHeapMap/Array/Set",
    ));
  }
  Ok(OffHeapValue::Primitive(js_to_primitive(env, val)?))
}

/// Convert a JS primitive (or OffHeapPrimitive) to PrimitiveValue.
pub(crate) fn js_to_primitive(env: &Env, val: Unknown<'_>) -> napi::Result<PrimitiveValue> {
  if val.get_type()? == ValueType::Object {
    let raw_env = env.raw();
    let raw_val = val.value().value;
    if OffHeapPrimitive::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, OffHeapPrimitive>::from_napi_value(raw_env, raw_val)? };
      return Ok(instance.inner.clone());
    }
    return Err(napi::Error::new(
      napi::Status::InvalidArg,
      "value must be a primitive or an OffHeap type",
    ));
  }
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
      "value must be a primitive or an OffHeap type",
    )),
  }
}

// ─── Rust → JS type conversion (raw-pointer level) ───────────────────────────
//
// Helper functions work with the raw `sys::napi_env` to avoid tying the
// returned JS values to any local `Env` borrow, which would prevent them from
// being returned from `#[napi]` methods where `Env` is owned.

/// Convert OffHeapValue to a raw napi_value.
/// For container types, a new thin JS shell is created that shares the same Arc.
pub(crate) fn to_napi_value_inner(
  raw_env: sys::napi_env,
  val: &OffHeapValue,
) -> napi::Result<sys::napi_value> {
  match val {
    OffHeapValue::Primitive(p) => primitive_to_napi(raw_env, p),
    OffHeapValue::Map(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = OffHeapMap {
        inner: Arc::clone(arc),
      }
      .into_instance(&env)?;
      Ok(instance.value)
    }
    OffHeapValue::Array(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = OffHeapArray {
        inner: Arc::clone(arc),
      }
      .into_instance(&env)?;
      Ok(instance.value)
    }
    OffHeapValue::Set(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = OffHeapSet {
        inner: Arc::clone(arc),
      }
      .into_instance(&env)?;
      Ok(instance.value)
    }
  }
}

/// Convert PrimitiveValue to a raw napi_value.
pub(crate) fn primitive_to_napi(
  raw_env: sys::napi_env,
  val: &PrimitiveValue,
) -> napi::Result<sys::napi_value> {
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
pub(crate) unsafe fn to_unknown(
  raw_env: sys::napi_env,
  raw_val: sys::napi_value,
) -> Unknown<'static> {
  Unknown::from_raw_unchecked(raw_env, raw_val)
}

/// OffHeapValue → Unknown<'static>  (lifetime erased; valid within the napi scope)
pub(crate) fn val_to_unknown(
  raw_env: sys::napi_env,
  val: &OffHeapValue,
) -> napi::Result<Unknown<'static>> {
  let raw = to_napi_value_inner(raw_env, val)?;
  Ok(unsafe { to_unknown(raw_env, raw) })
}

/// PrimitiveValue → Unknown<'static>
pub(crate) fn prim_to_unknown(
  raw_env: sys::napi_env,
  val: &PrimitiveValue,
) -> napi::Result<Unknown<'static>> {
  let raw = primitive_to_napi(raw_env, val)?;
  Ok(unsafe { to_unknown(raw_env, raw) })
}
