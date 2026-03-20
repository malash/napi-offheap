# napi-offheap

Off-heap containers for Node.js — store large, long-lived data on the Rust heap so V8's Mark-Compact GC never has to scan it.

## Why

When the V8 old-generation heap accumulates gigabytes of long-lived objects, a single Mark-Compact GC cycle can pause for hundreds of milliseconds. The root cause is that V8 must walk the entire object graph to find live references.

`napi-offheap` moves your data to the Rust heap. V8 only sees a thin wrapper object per container (one `Arc` pointer worth of memory). The actual data is completely invisible to the GC, so scan time drops to near zero regardless of how much data you store.

Shared references work transparently: when you `get` a nested container, the returned object shares the same underlying Rust allocation — mutations are immediately visible through all references, no copy involved.

## Install

```bash
npm install napi-offheap
```

## API

### `OffHeapObject<T>`

Strongly-typed off-heap replacement for plain JS objects. String keys, with per-key value types via the `T` type parameter.

```ts
import { OffHeapObject } from 'napi-offheap'

interface User {
  name: string
  age: number
}

const user = new OffHeapObject<User>()

user.set('name', 'alice').set('age', 30) // chainable
user.get('name') // → 'alice'  (type: string | undefined)
user.get('age')  // → 30      (type: number | undefined)
user.has('name') // → true
user.delete('name') // → true

user.size      // getter → number
user.keys()    // → (keyof User & string)[]
user.values()  // → User[keyof User][]
user.entries() // → [keyof User & string, User[keyof User]][]

user.forEach((value: User[keyof User], key: keyof User & string) => {
  /* ... */
})
```

> **vs `OffHeapMap<string, V>`**: `OffHeapObject<T>` provides per-key TypeScript types via `T[K]`, matching how plain JS objects are typed. `OffHeapMap<K, V>` supports any primitive key but all values share one type `V`.

### `OffHeapMap<K, V>`

Ordered key-value map (preserves insertion order, like JS `Map`). Keys can be any primitive: `string`, `number`, `boolean`, `null`, or `undefined`.

```ts
import { OffHeapMap } from 'napi-offheap'

const map = new OffHeapMap<string, number>()

map.set('a', 1).set('b', 2) // returns this — chainable

map.get('a')    // → 1  (type: number | undefined)
map.has('a')    // → true
map.delete('a') // → true (false if not found)
map.clear()

map.size       // getter → number
map.keys()     // → string[]
map.values()   // → number[]
map.entries()  // → [string, number][]

map.forEach((value: number, key: string) => {
  /* ... */
})

// Number keys work too
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
arr.pop()   // → 3  (type: number | undefined)
arr.get(0)  // → 1  (type: number | undefined)
arr.set(0, 99) // throws if index out of bounds
arr.length  // getter → number

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
set.has(1)    // → true
set.delete(1) // → true
set.clear()

set.size      // getter → number
set.values()  // → number[]

// callback receives (value, value) per JS Set.forEach spec
set.forEach((value: number, _value: number) => {
  /* ... */
})
```

## Nesting containers

With generics, nested containers are fully typed — no casts needed:

```ts
import { OffHeapMap } from 'napi-offheap'

const inner = new OffHeapMap<number>()
inner.set('a', 1)

const outer = new OffHeapMap<OffHeapMap<number>>()
outer.set('inner', inner)

const ref = outer.get('inner') // type: OffHeapMap<number> | undefined
ref?.set('a', 99)

inner.get('a') // → 99
```

> **Warning:** circular references (`a.set('b', b); b.set('a', a)`) cause memory leaks — `Arc` cannot break reference cycles.

## Accepted value types

| Type                                   | Map/Array/Object | Set |
| -------------------------------------- | ---------------- | --- |
| `null` / `undefined`                   | ✓                | ✓   |
| `boolean`                              | ✓                | ✓   |
| `number`                               | ✓                | ✓   |
| `string`                               | ✓                | ✓   |
| `OffHeapObject`                        | ✓                | ✗   |
| `OffHeapMap`                           | ✓                | ✗   |
| `OffHeapArray`                         | ✓                | ✗   |
| `OffHeapSet`                           | ✓                | ✗   |
| Plain JS objects / functions / Symbols | ✗                | ✗   |

## Build from source

```bash
# Prerequisites: Rust toolchain, Node.js >= 10.17
yarn install
yarn build
yarn test
```
