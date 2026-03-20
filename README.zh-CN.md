# napi-offheap

[English](./README.md) | [中文](./README.zh-CN.md)

Node.js 堆外容器 — 将大量长寿命数据存储在 Rust 堆上，让 V8 的 Mark-Compact GC 完全无需扫描。

> **⚠️ 由 AI 辅助编写，请谨慎使用。**
> 本项目完全通过 AI 辅助的"氛围编程"构建，未经过生产环境审计。实现中可能存在难以察觉的 bug、不安全的 unsafe 代码或从未被考虑到的边界情况。在任何关键场景中部署前请仔细审查。

## 为什么

当 V8 老生代堆上积累了大量长寿命对象时，单次 Mark-Compact GC 可能暂停数百毫秒。根本原因是 V8 必须遍历整个对象图来查找存活引用。

`napi-offheap` 将你的数据移到 Rust 堆上。V8 每个容器只需持有一个极薄的包装对象（仅一个 `Arc` 指针大小的内存）。实际数据对 GC 完全不可见，因此无论存储多少数据，扫描时间都接近零。

共享引用透明可用：当你 `get` 一个嵌套容器时，返回的对象与原始数据共享同一份 Rust 内存——修改会立即通过所有引用可见，不涉及任何数据拷贝。

需要注意的是，存储在 OffHeap 容器中的数据对 GC **不可见**，**不会**被自动回收。只有当容器的最后一个 JavaScript 引用被 GC 回收时（此时会 drop 底层的 `Arc`），内存才会被释放。如果你持有长寿命引用——例如模块级变量——在数据不再需要时，你有责任调用 `.clear()` 或重新赋值该变量。

## 安装

```bash
npm install napi-offheap
```

## API

### `OffHeapObject<T>`

普通 JS 对象的堆外替代。接受 `string` 或 `number` 类型的键——写入时 number 键会被强制转换为字符串，与 JS 对象语义一致（`1` → `"1"`）。键始终以字符串形式返回。

```ts
import { OffHeapObject } from 'napi-offheap'

interface User {
  name: string
  age: number
}

const user = new OffHeapObject<User>()

user.set('name', 'alice').set('age', 30) // 支持链式调用
user.get('name') // → 'alice'  (类型: string | undefined)
user.get('age') // → 30      (类型: number | undefined)
user.has('name') // → true
user.delete('name') // → true

user.size // getter → number
user.keys() // → string[]
user.values() // → User[keyof User][]
user.entries() // → [string, User[keyof User]][]

user.forEach((value, key) => {
  /* key 是 string */
})

// Number 键会被强制转换为字符串
user.set(1, 'one') // 以键 "1" 存储
user.get(1) // → 'one'
user.get('1') // → 'one'  (同一个键)
user.keys() // → ['1']
```

> **对比 `OffHeapMap<string, V>`**：`OffHeapObject<T>` 通过 `T[K]` 提供按键的 TypeScript 类型。`OffHeapMap<K, V>` 支持任意基本类型键（包括 `boolean`、`null`、`undefined`），并以原始类型保存和返回键。

### `OffHeapMap<K, V>`

有序键值映射（保持插入顺序，类似 JS `Map`）。键可以是任意基本类型：`string`、`number`、`boolean`、`null` 或 `undefined`。键以原始类型存储和返回。

```ts
import { OffHeapMap } from 'napi-offheap'

const map = new OffHeapMap<string, number>()

map.set('a', 1).set('b', 2) // 返回 this，支持链式调用

map.get('a') // → 1  (类型: number | undefined)
map.has('a') // → true
map.delete('a') // → true（未找到返回 false）
map.clear()

map.size // getter → number
map.keys() // → string[]
map.values() // → number[]
map.entries() // → [string, number][]

map.forEach((value: number, key: string) => {
  /* ... */
})

// Number 键同样支持——以 number 存储，不做强制转换
const byId = new OffHeapMap<number, string>()
byId.set(1, 'alice').set(2, 'bob')
byId.get(1) // → 'alice'
```

### `OffHeapArray<T>`

有序列表，push/pop 为 O(1)，splice 为 O(n)。

```ts
import { OffHeapArray } from 'napi-offheap'

const arr = new OffHeapArray<number>()

arr.push(1).push(2).push(3) // 支持链式调用
arr.pop() // → 3  (类型: number | undefined)
arr.get(0) // → 1  (类型: number | undefined)
arr.set(0, 99) // 越界时抛出异常
arr.length // getter → number

// splice(start, deleteCount, items) → 被删除的元素
const removed: number[] = arr.splice(1, 1, [10, 20])

arr.forEach((value: number, index: number) => {
  /* ... */
})
```

### `OffHeapSet<T>`

有序基本值集合（保持插入顺序，类似 JS `Set`）。

`T` 被限制为 `Primitive = string | number | boolean | null | undefined`。

```ts
import { OffHeapSet } from 'napi-offheap'

const set = new OffHeapSet<number>()

set.add(1).add(2).add(3) // 支持链式调用
set.has(1) // → true
set.delete(1) // → true
set.clear()

set.size // getter → number
set.values() // → number[]

// 回调接收 (value, value)，符合 JS Set.forEach 规范
set.forEach((value: number, _value: number) => {
  /* ... */
})
```

### 嵌套容器

通过泛型，嵌套容器可以完整类型化——无需类型断言：

```ts
import { OffHeapMap } from 'napi-offheap'

const inner = new OffHeapMap<string, number>()
inner.set('a', 1)

const outer = new OffHeapMap<string, OffHeapMap<string, number>>()
outer.set('inner', inner)

const ref = outer.get('inner') // 类型: OffHeapMap<string, number> | undefined
ref?.set('a', 99)

inner.get('a') // → 99
```

> **警告：** 循环引用（`a.set('b', b); b.set('a', a)`）会导致内存泄漏——`Arc` 无法打破引用循环。

### 支持的值类型

| 类型                                                           | Map/Array/Object 值 | Object 键    | Map 键 | Set 元素 |
| -------------------------------------------------------------- | ------------------- | ------------ | ------ | -------- |
| `string`                                                       | ✓                   | ✓            | ✓      | ✓        |
| `number`                                                       | ✓                   | ✓ (→ string) | ✓      | ✓        |
| `boolean`                                                      | ✓                   | ✗            | ✓      | ✓        |
| `null` / `undefined`                                           | ✓                   | ✗            | ✓      | ✓        |
| `OffHeapObject` / `OffHeapMap` / `OffHeapArray` / `OffHeapSet` | ✓                   | ✗            | ✗      | ✗        |
| 普通 JS 对象 / 函数 / Symbol                                   | ✗                   | ✗            | ✗      | ✗        |

## 性能基准

参见 [benchmark/bench.ts](./benchmark/bench.ts)，该基准测试对比了 V8 堆上 2000 万个存活对象与 `OffHeapArray` 中同等数量数据的 GC 暂停时间。

```bash
yarn bench
```

在 2000 万个长寿命元素的情况下，强制触发完整 GC 时，普通 JS 数组平均暂停约 **160ms**，而 `OffHeapArray` 仅约 **1ms**——GC 暂停时间约减少 **100 倍**。

## 从源码构建

```bash
# 前置条件：Rust 工具链，Node.js >= 10.17
yarn install
yarn build
yarn test
```

本项目使用 [napi-rs](https://github.com/napi-rs/napi-rs) 构建，这是一个用于在 Rust 中编写预编译 Node.js 原生插件的框架。
