use arcstr::ArcStr;
use napi::bindgen_prelude::*;
use napi::{sys, Env, ValueType};
use ordered_float::OrderedFloat;
use triomphe::Arc;

use crate::types::{
  OffHeapArray, OffHeapMap, OffHeapObject, OffHeapSet, OffHeapValue, PrimitiveValue,
};

pub(crate) fn js_to_persistent(env: &Env, val: Unknown<'_>) -> napi::Result<OffHeapValue> {
  let ty = val.get_type()?;
  if ty == ValueType::Object {
    let raw_env = env.raw();
    let raw_val = val.value().value;

    if OffHeapMap::instance_of(env, &val)? {
      let instance = unsafe { ClassInstance::<'_, OffHeapMap>::from_napi_value(raw_env, raw_val)? };
      return Ok(OffHeapValue::Map(Arc::clone(&instance.inner)));
    }
    if OffHeapArray::instance_of(env, &val)? {
      let instance =
        unsafe { ClassInstance::<'_, OffHeapArray>::from_napi_value(raw_env, raw_val)? };
      return Ok(OffHeapValue::Array(Arc::clone(&instance.inner)));
    }
    if OffHeapSet::instance_of(env, &val)? {
      let instance = unsafe { ClassInstance::<'_, OffHeapSet>::from_napi_value(raw_env, raw_val)? };
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
  js_primitive_ty_to_value(ty, val)
}

pub(crate) fn js_to_primitive(val: Unknown<'_>) -> napi::Result<PrimitiveValue> {
  js_to_primitive_ty(val.get_type()?, val)
}

fn js_to_primitive_ty(ty: ValueType, val: Unknown<'_>) -> napi::Result<PrimitiveValue> {
  match ty {
    ValueType::Null => Ok(PrimitiveValue::Null),
    ValueType::Undefined => Ok(PrimitiveValue::Undefined),
    ValueType::Boolean => Ok(PrimitiveValue::Bool(bool_from_unknown(val)?)),
    ValueType::Number => {
      let n = f64_from_unknown(val)?;
      if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        Ok(PrimitiveValue::Int(n as i64))
      } else {
        Ok(PrimitiveValue::Float(OrderedFloat(n)))
      }
    }
    ValueType::String => Ok(PrimitiveValue::Str(ArcStr::from(
      string_from_unknown(val)?.as_str(),
    ))),
    _ => Err(napi::Error::new(
      napi::Status::InvalidArg,
      "value must be a primitive or an OffHeap type",
    )),
  }
}

fn js_primitive_ty_to_value(ty: ValueType, val: Unknown<'_>) -> napi::Result<OffHeapValue> {
  match ty {
    ValueType::Null => Ok(OffHeapValue::Null),
    ValueType::Undefined => Ok(OffHeapValue::Undefined),
    ValueType::Boolean => Ok(OffHeapValue::Bool(bool_from_unknown(val)?)),
    ValueType::Number => {
      let n = f64_from_unknown(val)?;
      if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        Ok(OffHeapValue::Int(n as i64))
      } else {
        Ok(OffHeapValue::Float(OrderedFloat(n)))
      }
    }
    ValueType::String => Ok(OffHeapValue::Str(ArcStr::from(
      string_from_unknown(val)?.as_str(),
    ))),
    _ => Err(napi::Error::new(
      napi::Status::InvalidArg,
      "value must be a primitive or an OffHeap type",
    )),
  }
}

// Number keys are coerced to strings matching JS object semantics (e.g. 1 → "1", 1.5 → "1.5").
pub(crate) fn js_to_object_key(val: Unknown<'_>) -> napi::Result<String> {
  match val.get_type()? {
    ValueType::String => string_from_unknown(val),
    ValueType::Number => {
      let n = f64_from_unknown(val)?;
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

// Safe wrappers around `from_napi_value`. The pointers in `val.value()` are
// guaranteed valid for the lifetime of `val`, so the extraction is safe.
fn bool_from_unknown(val: Unknown<'_>) -> napi::Result<bool> {
  let v = val.value();
  unsafe { bool::from_napi_value(v.env, v.value) }
}

fn f64_from_unknown(val: Unknown<'_>) -> napi::Result<f64> {
  let v = val.value();
  unsafe { f64::from_napi_value(v.env, v.value) }
}

fn string_from_unknown(val: Unknown<'_>) -> napi::Result<String> {
  let v = val.value();
  unsafe { String::from_napi_value(v.env, v.value) }
}

pub(crate) fn to_napi_value_inner(
  raw_env: sys::napi_env,
  val: &OffHeapValue,
) -> napi::Result<sys::napi_value> {
  unsafe {
    match val {
      OffHeapValue::Null => Null::to_napi_value(raw_env, Null),
      OffHeapValue::Undefined => <()>::to_napi_value(raw_env, ()),
      OffHeapValue::Bool(b) => bool::to_napi_value(raw_env, *b),
      OffHeapValue::Int(i) => i64::to_napi_value(raw_env, *i),
      OffHeapValue::Float(f) => f64::to_napi_value(raw_env, f.0),
      OffHeapValue::Str(s) => <&str>::to_napi_value(raw_env, s.as_ref()),
      OffHeapValue::Map(arc) => {
        let env = Env::from_raw(raw_env);
        Ok(
          OffHeapMap {
            inner: Arc::clone(arc),
          }
          .into_instance(&env)?
          .value,
        )
      }
      OffHeapValue::Array(arc) => {
        let env = Env::from_raw(raw_env);
        Ok(
          OffHeapArray {
            inner: Arc::clone(arc),
          }
          .into_instance(&env)?
          .value,
        )
      }
      OffHeapValue::Set(arc) => {
        let env = Env::from_raw(raw_env);
        Ok(
          OffHeapSet {
            inner: Arc::clone(arc),
          }
          .into_instance(&env)?
          .value,
        )
      }
      OffHeapValue::Object(arc) => {
        let env = Env::from_raw(raw_env);
        Ok(
          OffHeapObject {
            inner: Arc::clone(arc),
          }
          .into_instance(&env)?
          .value,
        )
      }
    }
  }
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

#[inline]
pub(crate) fn to_unknown(raw_env: sys::napi_env, raw_val: sys::napi_value) -> Unknown<'static> {
  unsafe { Unknown::from_raw_unchecked(raw_env, raw_val) }
}

pub(crate) fn val_to_unknown(
  raw_env: sys::napi_env,
  val: &OffHeapValue,
) -> napi::Result<Unknown<'static>> {
  Ok(to_unknown(raw_env, to_napi_value_inner(raw_env, val)?))
}

pub(crate) fn prim_to_unknown(
  raw_env: sys::napi_env,
  val: &PrimitiveValue,
) -> napi::Result<Unknown<'static>> {
  Ok(to_unknown(raw_env, primitive_to_napi(raw_env, val)?))
}

pub(crate) fn undefined_to_unknown(raw_env: sys::napi_env) -> napi::Result<Unknown<'static>> {
  prim_to_unknown(raw_env, &PrimitiveValue::Undefined)
}

pub(crate) fn str_to_unknown(raw_env: sys::napi_env, s: &str) -> napi::Result<Unknown<'static>> {
  let raw = unsafe { <&str>::to_napi_value(raw_env, s)? };
  Ok(to_unknown(raw_env, raw))
}

pub(crate) fn array_to_unknown(raw_env: sys::napi_env, arr: Array) -> Unknown<'static> {
  to_unknown(raw_env, arr.raw())
}
