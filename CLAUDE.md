# OffHeap 堆外容器 — 完整实现规范

本文档面向 AI 代码生成。按此文档可完整还原所有源码，无需参考任何其他资料。

---

## 1. 背景

Node.js 在 V8 老生代堆上积累大量长寿命对象时，Mark-Compact GC 会周期性全量扫描对象图，4GB 数据量级下单次暂停可达数百毫秒。

**解法**：把数据存到 Rust 堆上，JS 端只持有极薄的 napi class 壳（每个实例仅一个 `Arc` 指针）。V8 GC 完全不可见 Rust 堆数据，扫描开销降到接近零。

`get` 返回的容器对象与原始数据共享同一个 `Arc`，修改立即生效，无需写回。

---

## 2. 依赖

```toml
# Cargo.toml

[package]
name = "napi_offheap"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
napi        = "3.0.0"
napi-derive = "3.0.0"
indexmap    = "2"
ordered-float = "4"

[build-dependencies]
napi-build = "2"

[profile.release]
lto    = true
strip  = "symbols"
```

**注意**：使用 napi **3.x**，不是 2.x。两个版本 API 差异极大，后续章节会详细说明。

---

## 3. 数据结构

### 3.1 PrimitiveValue

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveValue {
  Null,
  Undefined,
  Bool(bool),
  Int(i64),
  Float(OrderedFloat<f64>),  // f64 原生不能 Hash，用 OrderedFloat 包装
  Str(Arc<str>),             // Arc 避免 clone 时深拷贝字符串
}
```

- JS `number` 统一先读成 `f64`，若 `fract() == 0.0` 且在 `i64` 范围内则存为 `Int`，否则存为 `Float`。
- 变体名为 `Str`（不是 `String`，避免与 Rust 内置类型重名）。

### 3.2 OffHeapValue

```rust
#[derive(Debug, Clone)]
pub enum OffHeapValue {
  Primitive(PrimitiveValue),
  Map(Arc<Mutex<SharedMap>>),
  Array(Arc<Mutex<SharedArray>>),
  Set(Arc<Mutex<SharedSet>>),
  Object(Arc<Mutex<SharedObject>>),
}

pub type SharedMap    = IndexMap<PrimitiveValue, OffHeapValue>;
pub type SharedArray  = Vec<OffHeapValue>;
pub type SharedSet    = IndexSet<PrimitiveValue>;
// String keys only — number keys are coerced to strings on write, matching JS object semantics.
pub type SharedObject = IndexMap<String, OffHeapValue>;
```

**为什么 Set 只允许 PrimitiveValue**：Rust `HashSet` 要求 `Hash + Eq`，JS 对象没有稳定 hash，强行支持会引入语义不一致。

**为什么 `Arc<Mutex<T>>`**：
- `Arc`：多个 JS 壳可共享同一份数据
- `Mutex`：`#[napi]` 方法接收 `&self`，需要内部可变性
- 当前单线程 Node.js 场景下 Mutex 无竞争开销

### 3.3 napi class 壳

```rust
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
```

字段均为 `pub(crate)`：crate 内跨模块可访问，JS 端无法直接访问。

---

## 4. napi 3 API 速查（关键）

> napi 3 相对 napi 2 做了大幅重构。以下是本项目用到的全部 napi 3 API，逐条说明正确用法。

### 4.1 imports

```rust
use napi::bindgen_prelude::*;   // Unknown, Object, Array, ClassInstance,
                                 // FnArgs, This, Null, JavaScriptClassExt 等
use napi::{sys, Env, ValueType};
use napi_derive::napi;
```

`napi::bindgen_prelude::*` 导出了绝大多数常用类型，不需要单独 `use`。

### 4.2 Unknown<'env> — 对应 napi 2 的 JsUnknown

- 类型参数是生命周期，不是泛型类型。
- `val.get_type()? -> ValueType` 获取 JS 类型。
- `val.value()` 返回一个内部结构体，其 `.env` 字段是 `sys::napi_env`，`.value` 字段是 `sys::napi_value`。

### 4.3 原始值读取（JS → Rust）

**必须用 unsafe，直接调用 trait 方法，不能用 napi 2 的 cast 方式：**

```rust
let v = val.value();  // val: Unknown<'_>
let b = unsafe { bool::from_napi_value(v.env, v.value)? };
let n = unsafe { f64::from_napi_value(v.env, v.value)? };
let s = unsafe { String::from_napi_value(v.env, v.value)? };
// Null / Undefined 只需匹配 ValueType，不需要读值
```

### 4.4 原始值写出（Rust → sys::napi_value）

**必须用 unsafe，调用 `ToNapiValue` trait：**

```rust
unsafe { Null::to_napi_value(raw_env, Null) }
unsafe { <()>::to_napi_value(raw_env, ()) }          // undefined
unsafe { bool::to_napi_value(raw_env, b) }
unsafe { i64::to_napi_value(raw_env, i) }
unsafe { f64::to_napi_value(raw_env, f) }
unsafe { String::to_napi_value(raw_env, s.to_string()) }
unsafe { u32::to_napi_value(raw_env, n) }
```

返回值均为 `napi::Result<sys::napi_value>`。

### 4.5 Unknown<'static> — 解决生命周期问题

**核心问题**：`#[napi]` 方法接收 `env: Env`（按值，非引用）。如果辅助函数接受 `&Env` 并返回 `Unknown<'env>`，则该 `Unknown` 的生命周期与 `&env` 绑定，无法从方法中返回（`env` 是局部变量）。

**解法**：所有辅助函数改为接受 `raw_env: sys::napi_env`（裸指针，`Copy`，无生命周期），返回 `sys::napi_value` 或 `Unknown<'static>`。`#[napi]` 方法的返回类型也用 `Unknown<'static>`。

构造 `Unknown<'static>`：

```rust
unsafe fn to_unknown(raw_env: sys::napi_env, raw_val: sys::napi_value) -> Unknown<'static> {
  Unknown::from_raw_unchecked(raw_env, raw_val)
}
```

获取 `raw_env`：

```rust
let raw_env = env.raw();  // env: Env，消费 env 前调用
```

从 `raw_env` 重建 `Env`（仅在需要调用 `.into_instance()` 时使用）：

```rust
let env = Env::from_raw(raw_env);
```

### 4.6 napi class 实例化（Rust struct → JS 对象）

使用 `JavaScriptClassExt::into_instance`（已被 `bindgen_prelude::*` 导出）：

```rust
let instance = OffHeapMap { inner: Arc::clone(arc) }.into_instance(&env)?;
// instance.value 是 sys::napi_value
```

**不能**用 napi 2 的 `env.create_instance_of::<T>()`，该方法在 napi 3 中使用不同的包装机制，与 `#[napi]` class 不兼容。

### 4.7 napi class instanceof 检测

```rust
OffHeapMap::instance_of(env, &val)?    // -> napi::Result<bool>
OffHeapArray::instance_of(env, &val)?
OffHeapSet::instance_of(env, &val)?
OffHeapObject::instance_of(env, &val)?
```

参数：`env: &Env`，`val: &Unknown<'_>`。

### 4.8 napi class 实例解包（JS 对象 → Rust struct 引用）

```rust
let raw_val = val.value().value;  // sys::napi_value
let instance = unsafe {
  ClassInstance::<'_, OffHeapMap>::from_napi_value(raw_env, raw_val)?
};
// instance 解引用可访问 OffHeapMap 的字段
Arc::clone(&instance.inner)
```

**不能**用 napi 2 的 `env.unwrap::<T>(&obj)`，napi 3 中 `TaggedObject` 包装机制与 `#[napi]` class 不兼容。

### 4.9 This<'a> — 返回 this 实现链式调用

```rust
pub fn set<'a>(
  &self,
  this: This<'a>,      // napi 3 的 This 是 This<'a>，不是 This<JsObject>
  env: Env,
  key: Unknown<'_>,
  value: Unknown<'_>,
) -> napi::Result<Object<'a>> {
  // ...
  Ok(this.object)      // 访问 .object 字段，类型是 Object<'a>
}
```

**关键**：必须给方法加显式生命周期 `<'a>`，并让 `This<'a>` 和返回的 `Object<'a>` 使用同一生命周期。不加 `<'a>` 会导致编译器推断出两个不同生命周期，报错"返回值的生命周期与参数不匹配"。

本项目 `index.d.ts` 手动维护，Rust 侧无需加 `#[napi(ts_return_type = "this")]`，直接用 `#[napi]` 即可。

### 4.10 Function 类型 — 对应 napi 2 的 JsFunction

```rust
callback: Function<'_, FnArgs<(Unknown<'static>, Unknown<'static>)>, Unknown<'static>>
```

- 第一个泛型参数：生命周期（用 `'_`）
- 第二个泛型参数：参数类型，必须是 `FnArgs<T>`
- 第三个泛型参数：返回值类型

**FnArgs 是 struct，不是 tuple struct**，调用时：

```rust
callback.call(FnArgs { data: (arg1, arg2) })?;
// 错误写法（编译报错）：callback.call(FnArgs((arg1, arg2)))?;
```

### 4.11 Array::from_vec — 构造 JS 数组

```rust
let env_obj = Env::from_raw(raw_env);
let arr = Array::from_vec(&env_obj, vec![js_val1, js_val2])?;
// arr.raw() 返回 sys::napi_value
```

用于 `entries()` 把每个 `(key, value)` 对打包成 JS 的 `[key, value]`。

**注意**：napi 3 中 Rust tuple `(String, Unknown)` 不实现 `ToNapiValue`，不能直接作为 `Vec` 元素返回，必须手动构造 JS 数组。

---

## 5. 辅助函数

以下函数均为 crate 内部（`pub(crate)`），定义在 `src/convert.rs`。

### 5.1 lock_err — 统一锁错误转换

```rust
pub(crate) fn lock_err(e: impl std::fmt::Display) -> napi::Error {
  napi::Error::new(
    napi::Status::GenericFailure,
    format!("lock poisoned: {e}"),
  )
}
```

所有 `self.inner.lock()` 均用 `.map_err(lock_err)?` 处理。

### 5.2 js_to_persistent — JS 值 → OffHeapValue

```rust
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
```

- 先判断 `ValueType::Object`，再逐一 instanceof 检查，最后拒绝普通对象。
- 非 Object 类型走 `js_to_primitive`。

### 5.3 js_to_primitive — JS 原始值 → PrimitiveValue

```rust
pub(crate) fn js_to_primitive(val: Unknown<'_>) -> napi::Result<PrimitiveValue> {
  let v = val.value();
  match val.get_type()? {
    ValueType::Null      => Ok(PrimitiveValue::Null),
    ValueType::Undefined => Ok(PrimitiveValue::Undefined),
    ValueType::Boolean   => {
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
```

此函数**不接受 `env: &Env` 参数**，直接从 `val.value()` 取得裸指针。

### 5.4 js_to_object_key — JS string/number → String（OffHeapObject 专用）

```rust
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
```

匹配 JS 对象语义：number key 自动 coerce 为 string（`1` → `"1"`，`1.5` → `"1.5"`），其余类型报错。

### 5.5 to_napi_value_inner — OffHeapValue → sys::napi_value

```rust
pub(crate) fn to_napi_value_inner(
  raw_env: sys::napi_env,
  val: &OffHeapValue,
) -> napi::Result<sys::napi_value> {
  match val {
    OffHeapValue::Primitive(p) => primitive_to_napi(raw_env, p),
    OffHeapValue::Map(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = OffHeapMap { inner: Arc::clone(arc) }.into_instance(&env)?;
      Ok(instance.value)
    }
    OffHeapValue::Array(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = OffHeapArray { inner: Arc::clone(arc) }.into_instance(&env)?;
      Ok(instance.value)
    }
    OffHeapValue::Set(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = OffHeapSet { inner: Arc::clone(arc) }.into_instance(&env)?;
      Ok(instance.value)
    }
    OffHeapValue::Object(arc) => {
      let env = Env::from_raw(raw_env);
      let instance = OffHeapObject { inner: Arc::clone(arc) }.into_instance(&env)?;
      Ok(instance.value)
    }
  }
}
```

容器类型：`Arc::clone` 后用 `.into_instance(&env)?` 创建新 JS 壳，共享同一份 Rust 数据。

### 5.6 primitive_to_napi — PrimitiveValue → sys::napi_value

```rust
pub(crate) fn primitive_to_napi(
  raw_env: sys::napi_env,
  val: &PrimitiveValue,
) -> napi::Result<sys::napi_value> {
  unsafe {
    match val {
      PrimitiveValue::Null      => Null::to_napi_value(raw_env, Null),
      PrimitiveValue::Undefined => <()>::to_napi_value(raw_env, ()),
      PrimitiveValue::Bool(b)   => bool::to_napi_value(raw_env, *b),
      PrimitiveValue::Int(i)    => i64::to_napi_value(raw_env, *i),
      PrimitiveValue::Float(f)  => f64::to_napi_value(raw_env, f.0),
      PrimitiveValue::Str(s)    => String::to_napi_value(raw_env, s.to_string()),
    }
  }
}
```

### 5.7 to_unknown — 裸指针 → Unknown<'static>

```rust
#[inline]
pub(crate) unsafe fn to_unknown(
  raw_env: sys::napi_env,
  raw_val: sys::napi_value,
) -> Unknown<'static> {
  Unknown::from_raw_unchecked(raw_env, raw_val)
}
```

### 5.8 val_to_unknown / prim_to_unknown — 组合辅助

```rust
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
```

---

## 6. OffHeapMap 完整实现

```rust
#[napi]
impl OffHeapMap {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self { inner: Arc::new(Mutex::new(IndexMap::new())) }
  }

  #[napi]
  pub fn set<'a>(
    &self,
    this: This<'a>,
    env: Env,
    key: Unknown<'_>,
    value: Unknown<'_>,
  ) -> napi::Result<Object<'a>> {
    let k = js_to_primitive(key)?;
    let v = js_to_persistent(&env, value)?;
    self.inner.lock().map_err(lock_err)?.insert(k, v);
    Ok(this.object)
  }

  #[napi]
  pub fn get(&self, env: Env, key: Unknown<'_>) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let k = js_to_primitive(key)?;
    let guard = self.inner.lock().map_err(lock_err)?;
    match guard.get(&k) {
      None    => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, v),
    }
  }

  #[napi]
  pub fn has(&self, key: Unknown<'_>) -> napi::Result<bool> {
    let k = js_to_primitive(key)?;
    Ok(self.inner.lock().map_err(lock_err)?.contains_key(&k))
  }

  #[napi]
  pub fn delete(&self, key: Unknown<'_>) -> napi::Result<bool> {
    let k = js_to_primitive(key)?;
    Ok(self.inner.lock().map_err(lock_err)?.shift_remove(&k).is_some())
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
  pub fn keys(&self, env: Env) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    guard.keys().map(|k| prim_to_unknown(raw_env, k)).collect()
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
        let js_key  = prim_to_unknown(raw_env, k)?;
        let js_val  = val_to_unknown(raw_env, v)?;
        let env_obj = Env::from_raw(raw_env);
        let arr     = Array::from_vec(&env_obj, vec![js_key, js_val])?;
        Ok(unsafe { to_unknown(raw_env, arr.raw()) })
      })
      .collect()
  }

  // Lock is released before each callback so the callback can mutate the map without deadlock.
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
          let js_key = prim_to_unknown(raw_env, &key)?;
          callback.call(FnArgs { data: (js_val, js_key) })?;
          index += 1;
        }
      }
    }
    Ok(())
  }
}
```

---

## 7. OffHeapArray 完整实现

```rust
#[napi]
impl OffHeapArray {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self { inner: Arc::new(Mutex::new(Vec::new())) }
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
      None    => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, &v),
    }
  }

  #[napi]
  pub fn get(&self, env: Env, index: u32) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    match guard.get(index as usize) {
      None    => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
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
    let len   = guard.len();
    let start = (start as usize).min(len);
    let end   = (start + delete_count as usize).min(len);

    let removed: Vec<OffHeapValue> = guard.drain(start..end).collect();
    for (offset, item) in new_items.into_iter().enumerate() {
      guard.insert(start + offset, item);
    }
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
          let js_idx = unsafe { to_unknown(raw_env, u32::to_napi_value(raw_env, index as u32)?) };
          callback.call(FnArgs { data: (js_val, js_idx) })?;
          index += 1;
        }
      }
    }
    Ok(())
  }
}
```

---

## 8. OffHeapSet 完整实现

```rust
#[napi]
impl OffHeapSet {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self { inner: Arc::new(Mutex::new(IndexSet::new())) }
  }

  #[napi]
  pub fn add<'a>(
    &self,
    this: This<'a>,
    value: Unknown<'_>,
  ) -> napi::Result<Object<'a>> {
    let primitive = js_to_primitive(value)?;
    self.inner.lock().map_err(lock_err)?.insert(primitive);
    Ok(this.object)
  }

  #[napi]
  pub fn has(&self, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(value)?;
    Ok(self.inner.lock().map_err(lock_err)?.contains(&primitive))
  }

  #[napi]
  pub fn delete(&self, value: Unknown<'_>) -> napi::Result<bool> {
    let primitive = js_to_primitive(value)?;
    Ok(self.inner.lock().map_err(lock_err)?.shift_remove(&primitive))
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
  pub fn values(&self, env: Env) -> napi::Result<Vec<Unknown<'static>>> {
    let raw_env = env.raw();
    let guard = self.inner.lock().map_err(lock_err)?;
    guard.iter().map(|p| prim_to_unknown(raw_env, p)).collect()
  }

  // Lock is released before each callback so the callback can mutate the set without deadlock.
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
      };
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
```

---

## 9. OffHeapObject 完整实现

`OffHeapObject` 是 JS 普通对象的堆外替代。与 `OffHeapMap` 的差异：

- key 接受 `string` 或 `number`；number key 在写入时自动 coerce 为 string（匹配 JS 对象语义，`1` → `"1"`）
- 内部存储始终是 `IndexMap<String, OffHeapValue>`，读出的 key 均为 string
- TypeScript 类型参数 `T extends Record<string, unknown>` 提供按 key 的值类型

所有接受 key 的方法参数类型均为 `Unknown<'_>`，通过 `js_to_object_key` 转换。

```rust
#[napi]
impl OffHeapObject {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self { inner: Arc::new(Mutex::new(IndexMap::new())) }
  }

  #[napi]
  pub fn set<'a>(
    &self,
    this: This<'a>,
    env: Env,
    key: Unknown<'_>,
    value: Unknown<'_>,
  ) -> napi::Result<Object<'a>> {
    let k = js_to_object_key(key)?;
    let v = js_to_persistent(&env, value)?;
    self.inner.lock().map_err(lock_err)?.insert(k, v);
    Ok(this.object)
  }

  #[napi]
  pub fn get(&self, env: Env, key: Unknown<'_>) -> napi::Result<Unknown<'static>> {
    let raw_env = env.raw();
    let k = js_to_object_key(key)?;
    let guard = self.inner.lock().map_err(lock_err)?;
    match guard.get(&k) {
      None    => Ok(unsafe { to_unknown(raw_env, <()>::to_napi_value(raw_env, ())?) }),
      Some(v) => val_to_unknown(raw_env, v),
    }
  }

  #[napi]
  pub fn has(&self, key: Unknown<'_>) -> napi::Result<bool> {
    let k = js_to_object_key(key)?;
    Ok(self.inner.lock().map_err(lock_err)?.contains_key(&k))
  }

  #[napi]
  pub fn delete(&self, key: Unknown<'_>) -> napi::Result<bool> {
    let k = js_to_object_key(key)?;
    Ok(self.inner.lock().map_err(lock_err)?.shift_remove(&k).is_some())
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
        let js_key  = unsafe { to_unknown(raw_env, raw_key) };
        let js_val  = val_to_unknown(raw_env, v)?;
        let env_obj = Env::from_raw(raw_env);
        let arr     = Array::from_vec(&env_obj, vec![js_key, js_val])?;
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
```

---

## 10. 完整文件结构

```
src/lib.rs        — #![deny(clippy::all)] + mod 声明（无其他内容）
                    注：mod types 必须在所有 impl 模块之前声明，
                    否则 napi 宏在展开 impl 时找不到 struct 定义

src/types.rs      — 所有类型定义
                    PrimitiveValue enum
                    OffHeapValue enum（含 Object 变体）+ 4 个 type alias
                    4 个 #[napi] struct（OffHeapObject / OffHeapMap / OffHeapArray / OffHeapSet）
                    inner 字段为 pub(crate)

src/convert.rs    — JS ↔ Rust 转换辅助函数（均为 pub(crate)）
                    lock_err
                    js_to_persistent / js_to_primitive / js_to_object_key
                    to_napi_value_inner / primitive_to_napi
                    to_unknown（unsafe）/ val_to_unknown / prim_to_unknown

src/object.rs     — #[napi] impl OffHeapObject { ... }
src/map.rs        — #[napi] impl OffHeapMap { ... }
src/array.rs      — #[napi] impl OffHeapArray { ... }
src/set.rs        — #[napi] impl OffHeapSet { ... }
```

TypeScript 类型：

```
index.d.ts        — 手动维护，含完整泛型类型（不会被 napi build 覆盖）
_generated.d.ts   — napi build 自动生成，gitignored，不对外发布
```

---

## 11. 错误语义

| 操作 | error.code | 触发条件 | error.message |
|------|-----------|---------|---------------|
| set/push 传入普通 JS 对象 | `InvalidArg` | Object 但非 OffHeap 类型 | `plain JS objects are not accepted; wrap with OffHeapMap/Array/Set/Object` |
| set/push/add 传入函数、Symbol 等 | `InvalidArg` | 不可识别的值类型 | `value must be a primitive or an OffHeap type` |
| OffHeapSet.add 传入普通对象 | `InvalidArg` | Set 只接受基本类型 | `value must be a primitive or an OffHeap type` |
| OffHeapObject.set/get/has/delete 传入非 string/number key | `InvalidArg` | boolean、null、object 等 | `OffHeapObject key must be a string or number` |
| OffHeapArray.set 越界 | `GenericFailure` | index >= length | `index 5 out of bounds (length 3)` |
| 任意操作 Mutex 中毒 | `GenericFailure` | Rust panic 后 | `lock poisoned: ...` |

---

## 12. 不需要实现的内容

**`OffHeapPrimitive`**：原始值（number / string / boolean / null / undefined）本身不是 GC 负担，无需堆外包装。各容器的 `set` / `push` / `add` 直接接受 JS 原始值，不需要额外的包装类型。不实现。

---

## 13. 内存管理备注

- 删除/替换容器值时，`Arc` 引用计数自动递减，降至 0 时递归释放整棵子树，无需手动处理。
- **禁止循环引用**：`Arc` 无法处理循环引用，`a.set("b", b); b.set("a", a)` 会导致 Rust 数据永久泄漏。
- JS 壳被 GC 回收时，napi-rs 自动调用 Rust `drop`，`Arc` 引用计数 -1。
- `push` / `add` / `OffHeapMap.set` / `OffHeapObject.set` 通过 `This<'a>` 直接返回已有的 `this`，不创建新壳，无引用计数泄漏。
- `OffHeapArray.set` 返回 void，不支持链式调用。
