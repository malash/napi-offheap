use indexmap::{IndexMap, IndexSet};
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
pub enum OffHeapValue {
  Primitive(PrimitiveValue),
  Map(Arc<Mutex<SharedMap>>),
  Array(Arc<Mutex<SharedArray>>),
  Set(Arc<Mutex<SharedSet>>),
  Object(Arc<Mutex<SharedObject>>),
}

/// IndexMap preserves insertion order, matching JS Map semantics.
pub type SharedMap = IndexMap<PrimitiveValue, OffHeapValue>;
/// IndexMap with string keys, matching JS object semantics.
pub type SharedObject = IndexMap<String, OffHeapValue>;
/// Plain Vec.
pub type SharedArray = Vec<OffHeapValue>;
/// IndexSet preserves insertion order, matching JS Set semantics.
/// Elements are limited to PrimitiveValue because object identity hash is unstable.
pub type SharedSet = IndexSet<PrimitiveValue>;

// ─── napi class shells ────────────────────────────────────────────────────────

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
