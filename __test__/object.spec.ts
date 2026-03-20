import test from 'ava'

import { OffHeapObject } from '../entry'

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

test('OffHeapObject: forEach visits all entries even when current key is deleted', (t) => {
  // Mirrors JS Map.forEach: deleting the current key does not skip the next entry.
  const obj = new OffHeapObject()
  obj.set('a', 1).set('b', 2).set('c', 3)
  const visited: string[] = []
  obj.forEach((_value, key) => {
    visited.push(key)
    obj.delete(key)
  })
  t.deepEqual(visited, ['a', 'b', 'c'])
  t.is(obj.size, 0)
})

test('OffHeapObject: forEach visits new entries added during iteration', (t) => {
  // Mirrors JS Map.forEach: entries added during iteration are visited.
  const obj = new OffHeapObject()
  obj.set('a', 1)
  const visited: string[] = []
  obj.forEach((_value, key) => {
    visited.push(key)
    if (key === 'a') obj.set('b', 2)
    if (key === 'b') obj.set('c', 3)
  })
  t.deepEqual(visited, ['a', 'b', 'c'])
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

test('OffHeapObject: integer number key set/get roundtrip', (t) => {
  const obj = new OffHeapObject()
  obj.set(1, 'one').set(2, 'two')
  t.is(obj.get(1), 'one')
  t.is(obj.get(2), 'two')
})

test('OffHeapObject: float number key set/get roundtrip', (t) => {
  const obj = new OffHeapObject()
  obj.set(1.5, 'half')
  t.is(obj.get(1.5), 'half')
})

test('OffHeapObject: number key and string key "1" are the same key', (t) => {
  const obj = new OffHeapObject()
  obj.set(1, 'number-key')
  obj.set('1', 'string-key')
  t.is(obj.get(1), 'string-key')
  t.is(obj.get('1'), 'string-key')
  t.is(obj.size, 1)
})

test('OffHeapObject: has/delete with number key', (t) => {
  const obj = new OffHeapObject()
  obj.set(42, 'value')
  t.true(obj.has(42))
  t.true(obj.delete(42))
  t.false(obj.has(42))
})

test('OffHeapObject: keys() returns number keys coerced to strings', (t) => {
  const obj = new OffHeapObject()
  obj.set(1, 'a').set('b', 2).set(3, 'c')
  t.deepEqual(obj.keys(), ['1', 'b', '3'])
})

test('OffHeapObject: entries() number keys are coerced to strings', (t) => {
  const obj = new OffHeapObject()
  obj.set('a', 1).set(2, 'b')
  t.deepEqual(obj.entries(), [
    ['a', 1],
    ['2', 'b'],
  ])
})

test('OffHeapObject: forEach number keys are coerced to strings', (t) => {
  const obj = new OffHeapObject()
  obj.set(10, 'x').set(20, 'y')
  const result: [string, unknown][] = []
  obj.forEach((value, key) => result.push([key, value]))
  t.deepEqual(result, [
    ['10', 'x'],
    ['20', 'y'],
  ])
})
