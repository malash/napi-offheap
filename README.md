# napi-offheap

[English](./README.md) | [中文](./README.zh-CN.md)

Off-heap containers for Node.js — store large, long-lived data on the Rust heap so V8's Mark-Compact GC never has to scan it.

> **⚠️ Vibe coded — use with caution.**
> This project was built entirely through AI-assisted vibe coding and has not been audited for production use. The implementation may contain subtle bugs, unsound unsafe code, or edge cases that were never considered. Review carefully before deploying in any critical context.

## Why

When the V8 old-generation heap accumulates gigabytes of long-lived objects, a single Mark-Compact GC cycle can pause for hundreds of milliseconds. The root cause is that V8 must walk the entire object graph to find live references.

`napi-offheap` moves your data to the Rust heap. V8 only sees a thin wrapper object per container (one `Arc` pointer worth of memory). The actual data is completely invisible to the GC, so scan time drops to near zero regardless of how much data you store.

Shared references work transparently: when you `get` a nested container, the returned object shares the same underlying Rust allocation — mutations are immediately visible through all references, no copy involved.

The trade-off is that data stored in OffHeap containers is invisible to the GC and will **not** be reclaimed automatically. Memory is freed only when the last JavaScript reference to a container is garbage collected (which drops the underlying `Arc`). If you hold a long-lived reference — e.g. a module-level variable — you are responsible for calling `.clear()` or reassigning the variable when the data is no longer needed.

## Install

```bash
npm install napi-offheap
```

## API

### `OffHeapObject<T>`

Off-heap replacement for plain JS objects. Accepts `string` or `number` keys — number keys are coerced to strings on write, matching JS object semantics (`1` → `"1"`). Keys are always returned as strings.

```ts
import { OffHeapObject } from 'napi-offheap'

interface User {
  name: string
  age: number
}

const user = new OffHeapObject<User>()

user.set('name', 'alice').set('age', 30) // chainable
user.get('name') // → 'alice'  (type: string | undefined)
user.get('age') // → 30      (type: number | undefined)
user.has('name') // → true
user.delete('name') // → true

user.size // getter → number
user.keys() // → string[]
user.values() // → User[keyof User][]
user.entries() // → [string, User[keyof User]][]

user.forEach((value, key) => {
  /* key is string */
})

// Number keys are accepted and coerced to strings
user.set(1, 'one') // stored as key "1"
user.get(1) // → 'one'
user.get('1') // → 'one'  (same key)
user.keys() // → ['1']
```

> **vs `OffHeapMap<string, V>`**: `OffHeapObject<T>` provides per-key TypeScript types via `T[K]`. `OffHeapMap<K, V>` supports any primitive key type (including `boolean`, `null`, `undefined`) and keeps keys as their original type.

### `OffHeapMap<K, V>`

Ordered key-value map (preserves insertion order, like JS `Map`). Keys can be any primitive: `string`, `number`, `boolean`, `null`, or `undefined`. Keys are stored and returned as their original type.

```ts
import { OffHeapMap } from 'napi-offheap'

const map = new OffHeapMap<string, number>()

map.set('a', 1).set('b', 2) // returns this — chainable

map.get('a') // → 1  (type: number | undefined)
map.has('a') // → true
map.delete('a') // → true (false if not found)
map.clear()

map.size // getter → number
map.keys() // → string[]
map.values() // → number[]
map.entries() // → [string, number][]

map.forEach((value: number, key: string) => {
  /* ... */
})

// Number keys work too — stored as number, not coerced
const byId = new OffHeapMap<number, string>()
byId.set(1, 'alice').set(2, 'bob')
byId.get(1) // → 'alice'
```

### `OffHeapArray<T>`

Ordered list with O(1) push/pop and O(n) splice.

```ts
import { OffHeapArray } from 'napi-offheap'

const arr = new OffHeapArray<number>()

arr.push(1).push(2).push(3) // chainable
arr.pop() // → 3  (type: number | undefined)
arr.get(0) // → 1  (type: number | undefined)
arr.set(0, 99) // throws if index out of bounds
arr.length // getter → number

// splice(start, deleteCount, items) → removed elements
const removed: number[] = arr.splice(1, 1, [10, 20])

arr.forEach((value: number, index: number) => {
  /* ... */
})
```

### `OffHeapSet<T>`

Ordered set of primitive values (preserves insertion order, like JS `Set`).

`T` is constrained to `Primitive = string | number | boolean | null | undefined`.

```ts
import { OffHeapSet } from 'napi-offheap'

const set = new OffHeapSet<number>()

set.add(1).add(2).add(3) // chainable
set.has(1) // → true
set.delete(1) // → true
set.clear()

set.size // getter → number
set.values() // → number[]

// callback receives (value, value) per JS Set.forEach spec
set.forEach((value: number, _value: number) => {
  /* ... */
})
```

### Nesting containers

With generics, nested containers are fully typed — no casts needed:

```ts
import { OffHeapMap } from 'napi-offheap'

const inner = new OffHeapMap<string, number>()
inner.set('a', 1)

const outer = new OffHeapMap<string, OffHeapMap<string, number>>()
outer.set('inner', inner)

const ref = outer.get('inner') // type: OffHeapMap<string, number> | undefined
ref?.set('a', 99)

inner.get('a') // → 99
```

> **Warning:** circular references (`a.set('b', b); b.set('a', a)`) cause memory leaks — `Arc` cannot break reference cycles.

### Accepted value types

| Type                                                           | Map/Array/Object value | Object key   | Map key | Set element |
| -------------------------------------------------------------- | ---------------------- | ------------ | ------- | ----------- |
| `string`                                                       | ✓                      | ✓            | ✓       | ✓           |
| `number`                                                       | ✓                      | ✓ (→ string) | ✓       | ✓           |
| `boolean`                                                      | ✓                      | ✗            | ✓       | ✓           |
| `null` / `undefined`                                           | ✓                      | ✗            | ✓       | ✓           |
| `OffHeapObject` / `OffHeapMap` / `OffHeapArray` / `OffHeapSet` | ✓                      | ✗            | ✗       | ✗           |
| Plain JS objects / functions / Symbols                         | ✗                      | ✗            | ✗       | ✗           |

## Benchmark

See [benchmark/bench.ts](./benchmark/bench.ts) for a GC pause benchmark comparing 20M live objects on the V8 heap vs. in an `OffHeapArray`.

```bash
yarn bench
```

With 20M long-lived elements, a forced full GC averages **~160ms** with a plain JS array vs. **~1ms** with `OffHeapArray` — roughly a **100× reduction** in GC pause time.

## Build from source

```bash
# Prerequisites: Rust toolchain, Node.js >= 10.17
yarn install
yarn build
yarn test
```

Built with [napi-rs](https://github.com/napi-rs/napi-rs), a framework for building pre-compiled Node.js addons in Rust.
