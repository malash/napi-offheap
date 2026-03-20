use indexmap::{IndexMap, IndexSet};
use napi_derive::napi;
use ordered_float::OrderedFloat;
use std::sync::{Arc, Mutex};

// f64 is wrapped in OrderedFloat to satisfy Hash + Eq (required for Map keys and Set elements).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveValue {
  Null,
  Undefined,
  Bool(bool),
  Int(i64),
  Float(OrderedFloat<f64>),
  Str(Arc<str>),
}

#[derive(Debug, Clone)]
pub enum OffHeapValue {
  Primitive(PrimitiveValue),
  Map(Arc<Mutex<SharedMap>>),
  Array(Arc<Mutex<SharedArray>>),
  Set(Arc<Mutex<SharedSet>>),
  Object(Arc<Mutex<SharedObject>>),
}

pub type SharedMap = IndexMap<PrimitiveValue, OffHeapValue>;
pub type SharedObject = IndexMap<String, OffHeapValue>;
pub type SharedArray = Vec<OffHeapValue>;
// Set elements are limited to PrimitiveValue: object identity has no stable hash.
pub type SharedSet = IndexSet<PrimitiveValue>;

#[napi]
pub struct OffHeapObject {
  pub(crate) inner: Arc<Mutex<SharedObject>>,
}

#[napi]
pub struct OffHeapMap {
  pub(crate) inner: Arc<Mutex<SharedMap>>,
}

#[napi]
pub struct OffHeapArray {
  pub(crate) inner: Arc<Mutex<SharedArray>>,
}

#[napi]
pub struct OffHeapSet {
  pub(crate) inner: Arc<Mutex<SharedSet>>,
}
