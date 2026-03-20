import test from 'ava'

import { OffHeapObject } from '../index'

test('OffHeapObject: constructor creates empty object', (t) => {
  const obj = new OffHeapObject()
  t.is(obj.size, 0)
})

test('OffHeapObject: set/get roundtrip', (t) => {
  const obj = new OffHeapObject<{ x: number; y: string }>()
  obj.set('x', 1).set('y', 'hello')
  t.is(obj.get('x'), 1)
  t.is(obj.get('y'), 'hello')
})

test('OffHeapObject: get returns undefined for missing key', (t) => {
  const obj = new OffHeapObject()
  t.is(obj.get('missing'), undefined)
})

test('OffHeapObject: set overwrites existing key', (t) => {
  const obj = new OffHeapObject<{ k: number }>()
  obj.set('k', 1).set('k', 2)
  t.is(obj.get('k'), 2)
  t.is(obj.size, 1)
})

test('OffHeapObject: has', (t) => {
  const obj = new OffHeapObject()
  obj.set('k', 1)
  t.true(obj.has('k'))
  t.false(obj.has('missing'))
})

test('OffHeapObject: delete returns true when key exists', (t) => {
  const obj = new OffHeapObject()
  obj.set('k', 1)
  t.true(obj.delete('k'))
  t.false(obj.has('k'))
})

test('OffHeapObject: delete returns false for missing key', (t) => {
  const obj = new OffHeapObject()
  t.false(obj.delete('missing'))
})

test('OffHeapObject: clear', (t) => {
  const obj = new OffHeapObject()
  obj.set('a', 1).set('b', 2)
  obj.clear()
  t.is(obj.size, 0)
  t.false(obj.has('a'))
})

test('OffHeapObject: size tracks insertions and deletions', (t) => {
  const obj = new OffHeapObject()
  t.is(obj.size, 0)
  obj.set('a', 1)
  t.is(obj.size, 1)
  obj.set('b', 2)
  t.is(obj.size, 2)
  obj.delete('a')
  t.is(obj.size, 1)
})

test('OffHeapObject: keys preserves insertion order', (t) => {
  const obj = new OffHeapObject()
  obj.set('b', 2).set('a', 1).set('c', 3)
  t.deepEqual(obj.keys(), ['b', 'a', 'c'])
})

test('OffHeapObject: values', (t) => {
  const obj = new OffHeapObject()
  obj.set('a', 1).set('b', 2)
  t.deepEqual(obj.values(), [1, 2])
})

test('OffHeapObject: entries', (t) => {
  const obj = new OffHeapObject()
  obj.set('a', 1).set('b', 2)
  t.deepEqual(obj.entries(), [
    ['a', 1],
    ['b', 2],
  ])
})

test('OffHeapObject: forEach iterates in insertion order', (t) => {
  const obj = new OffHeapObject()
  obj.set('x', 10).set('y', 20)
  const result: [string, unknown][] = []
  obj.forEach((value, key) => result.push([key, value]))
  t.deepEqual(result, [
    ['x', 10],
    ['y', 20],
  ])
})

test('OffHeapObject: forEach callback can overwrite values without deadlock', (t) => {
  const obj = new OffHeapObject()
  obj.set('a', 1).set('b', 2)
  obj.forEach((_value, key) => {
    obj.set(key, 99)
  })
  t.is(obj.get('a'), 99)
  t.is(obj.get('b'), 99)
})

test('OffHeapObject: forEach callback can delete keys without deadlock', (t) => {
  const obj = new OffHeapObject()
  obj.set('a', 1).set('b', 2).set('c', 3)
  const visited: string[] = []
  obj.forEach((_value, key) => {
    visited.push(key)
    obj.delete(key)
  })
  // IndexMap shifts elements left after each delete, so the cursor skips every
  // other entry: 'a' (idx 0) and 'c' (now idx 1 after shift) are visited; 'b'
  // (which became idx 0 then shifted to idx 0 again) is skipped.
  t.deepEqual(visited, ['a', 'c'])
  t.is(obj.size, 1)
  t.true(obj.has('b'))
})

test('OffHeapObject: set returns this for chaining', (t) => {
  const obj = new OffHeapObject()
  const ret = obj.set('k', 1)
  t.is(ret, obj)
})

test('OffHeapObject: stores all primitive types as values', (t) => {
  const obj = new OffHeapObject()
  obj.set('str', 'hello')
  obj.set('int', 42)
  obj.set('float', 1.5)
  obj.set('bool', true)
  obj.set('null', null)
  obj.set('undef', undefined)
  t.is(obj.get('str'), 'hello')
  t.is(obj.get('int'), 42)
  t.is(obj.get('float'), 1.5)
  t.is(obj.get('bool'), true)
  t.is(obj.get('null'), null)
  t.is(obj.get('undef'), undefined)
})

test('OffHeapObject: keys/values/entries on empty object return empty arrays', (t) => {
  const obj = new OffHeapObject()
  t.deepEqual(obj.keys(), [])
  t.deepEqual(obj.values(), [])
  t.deepEqual(obj.entries(), [])
})
