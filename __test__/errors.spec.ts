import test from 'ava'

import { OffHeapArray, OffHeapMap, OffHeapObject, OffHeapSet } from '../entry'

// ─── Plain JS object rejection ───────────────────────────────────────────────

test('error: plain object rejected in OffHeapMap.set value', (t) => {
  const map = new OffHeapMap()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => map.set('k', {} as any), { message: /plain JS objects are not accepted/ })
})

test('error: plain object rejected in OffHeapArray.push', (t) => {
  const arr = new OffHeapArray()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => arr.push({} as any), { message: /plain JS objects are not accepted/ })
})

test('error: plain object rejected in OffHeapObject.set value', (t) => {
  const obj = new OffHeapObject()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => obj.set('k', {} as any), { message: /plain JS objects are not accepted/ })
})

test('error: array literal rejected in OffHeapMap.set value', (t) => {
  const map = new OffHeapMap()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => map.set('k', [] as any), { message: /plain JS objects are not accepted/ })
})

// ─── Unsupported type rejection ──────────────────────────────────────────────

test('error: function rejected in OffHeapMap.set value', (t) => {
  const map = new OffHeapMap()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => map.set('k', (() => {}) as any), { message: /value must be a primitive/ })
})

test('error: plain object rejected in OffHeapSet.add', (t) => {
  const set = new OffHeapSet()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => set.add({} as any), { message: /value must be a primitive/ })
})

test('error: OffHeapMap rejected in OffHeapSet.add', (t) => {
  const set = new OffHeapSet()
  const inner = new OffHeapMap()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => set.add(inner as any), { message: /value must be a primitive/ })
})

// ─── OffHeapObject invalid key type ──────────────────────────────────────────

test('error: OffHeapObject.set rejects non-string/number key', (t) => {
  const obj = new OffHeapObject()
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t.throws(() => obj.set(true as any, 1), { message: /OffHeapObject key must be a string or number/ })
})

// ─── Out-of-bounds ───────────────────────────────────────────────────────────

test('error: OffHeapArray.set throws on out-of-bounds with message', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2)
  t.throws(() => arr.set(5, 99), { message: /out of bounds/ })
})

test('error: OffHeapArray.set throws on empty array', (t) => {
  const arr = new OffHeapArray<number>()
  const err = t.throws(() => arr.set(0, 1))
  t.regex(err!.message, /out of bounds/)
})

test('error: OffHeapArray.set error message contains index and length', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(10).push(20)
  const err = t.throws(() => arr.set(5, 99))
  t.regex(err!.message, /5/)
  t.regex(err!.message, /2/)
})
