use napi::bindgen_prelude::*;
use napi::{sys, Env, ValueType};
use ordered_float::OrderedFloat;
use std::sync::Arc;

use crate::types::{OffHeapArray, OffHeapMap, OffHeapObject, OffHeapSet, OffHeapValue, PrimitiveValue};

pub(crate) fn lock_err(e: impl std::fmt::Display) -> napi::Error {
  napi::Error::new(
    napi::Status::GenericFailure,
    format!("lock poisoned: {e}"),
  )
}

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
    if OffHeapObject::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, OffHeapObject>::from_napi_value(raw_env, raw_val)? };
      return Ok(OffHeapValue::Object(Arc::clone(&instance.inner)));
    }

    return Err(napi::Error::new(
      napi::Status::InvalidArg,
      "plain JS objects are not accepted; wrap with OffHeapMap/Array/Set/Object",
    ));
  }
  Ok(OffHeapValue::Primitive(js_to_primitive(val)?))
}

pub(crate) fn js_to_primitive(val: Unknown<'_>) -> napi::Result<PrimitiveValue> {
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

// Number keys are coerced to strings matching JS object semantics (e.g. 1 → "1", 1.5 → "1.5").
pub(crate) fn js_to_object_key(val: Unknown<'_>) -> napi::Result<String> {
  let v = val.value();
  match val.get_type()? {
    ValueType::String => Ok(unsafe { String::from_napi_value(v.env, v.value)? }),
    ValueType::Number => {
      let n = unsafe { f64::from_napi_value(v.env, v.value)? };
      if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        Ok(format!("{}", n as i64))
      } else {
        Ok(format!("{}", n))
      }
    }
    _ => Err(napi::Error::new(
      napi::Status::InvalidArg,
      "OffHeapObject key must be a string or number",
    )),
  }
}

// All helpers below accept raw `sys::napi_env` instead of `&Env` so that the
// returned values are not tied to a local borrow — `#[napi]` methods receive
// `Env` by value, so borrowing it would prevent returning the value.

pub(crate) fn to_napi_value_inner(
  raw_env: sys::napi_env,
  val: &OffHeapValue,
) -> napi::Result<sys::napi_value> {
  if let OffHeapValue::Primitive(p) = val {
    return primitive_to_napi(raw_env, p);
  }
  let env = Env::from_raw(raw_env);
  let instance_value = match val {
    OffHeapValue::Primitive(_) => unreachable!(),
    OffHeapValue::Map(arc) => OffHeapMap { inner: Arc::clone(arc) }.into_instance(&env)?.value,
    OffHeapValue::Array(arc) => OffHeapArray { inner: Arc::clone(arc) }.into_instance(&env)?.value,
    OffHeapValue::Set(arc) => OffHeapSet { inner: Arc::clone(arc) }.into_instance(&env)?.value,
    OffHeapValue::Object(arc) => OffHeapObject { inner: Arc::clone(arc) }.into_instance(&env)?.value,
  };
  Ok(instance_value)
}

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
      PrimitiveValue::Str(s) => <&str>::to_napi_value(raw_env, s.as_ref()),
    }
  }
}

/// # Safety
/// The caller must ensure the napi_value is valid for the intended lifetime.
#[inline]
pub(crate) unsafe fn to_unknown(
  raw_env: sys::napi_env,
  raw_val: sys::napi_value,
) -> Unknown<'static> {
  Unknown::from_raw_unchecked(raw_env, raw_val)
}

pub(crate) fn val_to_unknown(
  raw_env: sys::napi_env,
  val: &OffHeapValue,
) -> napi::Result<Unknown<'static>> {
  let raw = to_napi_value_inner(raw_env, val)?;
  Ok(unsafe { to_unknown(raw_env, raw) })
}

pub(crate) fn prim_to_unknown(
  raw_env: sys::napi_env,
  val: &PrimitiveValue,
) -> napi::Result<Unknown<'static>> {
  let raw = primitive_to_napi(raw_env, val)?;
  Ok(unsafe { to_unknown(raw_env, raw) })
}
