import test from 'ava'

import { OffHeapSet } from '../index'

test('OffHeapSet: constructor creates empty set', (t) => {
  const set = new OffHeapSet()
  t.is(set.size, 0)
})

test('OffHeapSet: add/has', (t) => {
  const set = new OffHeapSet<number>()
  set.add(1).add(2)
  t.true(set.has(1))
  t.true(set.has(2))
  t.false(set.has(3))
})

test('OffHeapSet: add is idempotent', (t) => {
  const set = new OffHeapSet<number>()
  set.add(1).add(1).add(1)
  t.is(set.size, 1)
})

test('OffHeapSet: delete returns true when value exists', (t) => {
  const set = new OffHeapSet<number>()
  set.add(1)
  t.true(set.delete(1))
  t.false(set.has(1))
})

test('OffHeapSet: delete returns false for missing value', (t) => {
  const set = new OffHeapSet<number>()
  t.false(set.delete(1))
})

test('OffHeapSet: clear', (t) => {
  const set = new OffHeapSet<number>()
  set.add(1).add(2).add(3)
  set.clear()
  t.is(set.size, 0)
})

test('OffHeapSet: size tracks insertions and deletions', (t) => {
  const set = new OffHeapSet<number>()
  t.is(set.size, 0)
  set.add(1)
  t.is(set.size, 1)
  set.add(1) // duplicate
  t.is(set.size, 1)
  set.add(2)
  t.is(set.size, 2)
  set.delete(1)
  t.is(set.size, 1)
})

test('OffHeapSet: values preserves insertion order', (t) => {
  const set = new OffHeapSet<number>()
  set.add(3).add(1).add(2)
  t.deepEqual(set.values(), [3, 1, 2])
})

test('OffHeapSet: values on empty set returns empty array', (t) => {
  const set = new OffHeapSet()
  t.deepEqual(set.values(), [])
})

test('OffHeapSet: supports all primitive types', (t) => {
  const set = new OffHeapSet()
  set.add('str').add(42).add(true).add(null).add(undefined)
  t.is(set.size, 5)
  t.true(set.has('str'))
  t.true(set.has(42))
  t.true(set.has(true))
  t.true(set.has(null))
  t.true(set.has(undefined))
})

test('OffHeapSet: integer and float with same numeric value are the same key', (t) => {
  // In JS, 1 === 1.0, so both map to PrimitiveValue::Int(1)
  const set = new OffHeapSet<number>()
  set.add(1).add(1.0)
  t.is(set.size, 1)
})

test('OffHeapSet: integer and distinct float are different keys', (t) => {
  const set = new OffHeapSet<number>()
  set.add(1).add(1.5)
  t.is(set.size, 2)
})

test('OffHeapSet: forEach receives (value, value) per spec', (t) => {
  const set = new OffHeapSet<number>()
  set.add(1).add(2)
  const result: [unknown, unknown][] = []
  set.forEach((v1, v2) => result.push([v1, v2]))
  t.deepEqual(result, [
    [1, 1],
    [2, 2],
  ])
})

test('OffHeapSet: forEach callback can delete current value without deadlock', (t) => {
  const set = new OffHeapSet<number>()
  set.add(1).add(2).add(3)
  const visited: number[] = []
  set.forEach((v) => {
    visited.push(v as number)
    set.delete(v as number)
  })
  // IndexSet shifts elements left after each delete, so the cursor skips every
  // other entry: 1 (idx 0) and 3 (now idx 1 after shift) are visited; 2 is skipped.
  t.deepEqual(visited, [1, 3])
  t.is(set.size, 1)
  t.true(set.has(2))
})

test('OffHeapSet: forEach on empty set does not invoke callback', (t) => {
  const set = new OffHeapSet()
  let called = false
  set.forEach(() => {
    called = true
  })
  t.false(called)
})

test('OffHeapSet: add returns this for chaining', (t) => {
  const set = new OffHeapSet()
  const ret = set.add(1)
  t.is(ret, set)
})
