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

### `OffHeapMap`

Ordered key-value map (preserves insertion order, like JS `Map`).

```ts
const map = new OffHeapMap()

map.set('key', 'value') // returns this — chainable
map.set('n', 42).set('ok', true)

map.get('key') // → 'value'
map.has('key') // → true
map.delete('key') // → true (false if not found)
map.clear()

map.size // getter → number
map.keys() // → string[]
map.values() // → unknown[]
map.entries() // → [string, unknown][]

map.forEach((value, key) => {
  /* ... */
})
```

### `OffHeapArray`

Ordered list with O(1) push/pop and O(n) splice.

```ts
const arr = new OffHeapArray()

arr.push(1).push(2).push(3) // chainable
arr.pop() // → 3
arr.get(0) // → 1
arr.set(0, 99) // throws if index out of bounds
arr.length // getter → number

// splice(start, deleteCount, ...items) → removed elements
arr.splice(1, 1, 'a', 'b')

arr.forEach((value, index) => {
  /* ... */
})
```

### `OffHeapSet`

Ordered set of primitive values (preserves insertion order, like JS `Set`).

Only accepts primitives: `null`, `undefined`, `boolean`, `number`, `string`, or `OffHeapPrimitive`.

```ts
const set = new OffHeapSet()

set.add(1).add('hello').add(true) // chainable
set.has(1) // → true
set.delete(1) // → true
set.clear()

set.size // getter → number
set.values() // → unknown[]

set.forEach((value, value) => {
  /* with JS Set.forEach semantics */
})
```

## Nesting containers

Containers can be nested freely. A nested container shares its underlying data — mutations propagate instantly:

```ts
const inner = new OffHeapMap()
inner.set('a', 1)

const outer = new OffHeapMap()
outer.set('inner', inner)

const ref = outer.get('inner') // same Arc as `inner`
ref.set('a', 99)

inner.get('a') // → 99
```

> **Warning:** circular references (`a.set('b', b); b.set('a', a)`) cause memory leaks — `Arc` cannot break reference cycles.

## Accepted value types

| Type                                   | Map/Array     | Set           |
| -------------------------------------- | ------------- | ------------- |
| `null` / `undefined`                   | ✓             | ✓             |
| `boolean`                              | ✓             | ✓             |
| `number`                               | ✓             | ✓             |
| `string`                               | ✓             | ✓             |
| `OffHeapMap`                           | ✓             | ✗             |
| `OffHeapArray`                         | ✓             | ✗             |
| `OffHeapSet`                           | ✓             | ✗             |
| Plain JS objects / functions / Symbols | ✗             | ✗             |

## Build from source

```bash
# Prerequisites: Rust toolchain, Node.js >= 10.17
yarn install
yarn build
yarn test
```
