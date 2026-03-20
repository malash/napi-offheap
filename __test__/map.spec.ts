import test from 'ava'

import { OffHeapMap } from '../entry'

test('OffHeapMap: constructor creates empty map', (t) => {
  const map = new OffHeapMap()
  t.is(map.size, 0)
})

test('OffHeapMap: set/get with string keys', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('a', 1).set('b', 2)
  t.is(map.get('a'), 1)
  t.is(map.get('b'), 2)
})

test('OffHeapMap: set/get with number keys', (t) => {
  const map = new OffHeapMap<number, string>()
  map.set(1, 'one').set(2, 'two')
  t.is(map.get(1), 'one')
  t.is(map.get(2), 'two')
})

test('OffHeapMap: integer and float keys are distinct', (t) => {
  const map = new OffHeapMap<number, string>()
  map.set(1, 'int').set(1.5, 'float')
  t.is(map.get(1), 'int')
  t.is(map.get(1.5), 'float')
  t.is(map.size, 2)
})

test('OffHeapMap: set/get with boolean keys', (t) => {
  const map = new OffHeapMap<boolean, string>()
  map.set(true, 'yes').set(false, 'no')
  t.is(map.get(true), 'yes')
  t.is(map.get(false), 'no')
})

test('OffHeapMap: set/get with null and undefined keys', (t) => {
  const map = new OffHeapMap()
  map.set(null, 'null-val').set(undefined, 'undef-val')
  t.is(map.get(null), 'null-val')
  t.is(map.get(undefined), 'undef-val')
})

test('OffHeapMap: get returns undefined for missing key', (t) => {
  const map = new OffHeapMap<string, number>()
  t.is(map.get('missing'), undefined)
})

test('OffHeapMap: has', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('k', 1)
  t.true(map.has('k'))
  t.false(map.has('missing'))
})

test('OffHeapMap: delete returns true when key exists', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('k', 1)
  t.true(map.delete('k'))
  t.false(map.has('k'))
})

test('OffHeapMap: delete returns false for missing key', (t) => {
  const map = new OffHeapMap<string, number>()
  t.false(map.delete('missing'))
})

test('OffHeapMap: clear', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('a', 1).set('b', 2)
  map.clear()
  t.is(map.size, 0)
})

test('OffHeapMap: overwriting same key does not grow size', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('a', 1).set('a', 2)
  t.is(map.size, 1)
  t.is(map.get('a'), 2)
})

test('OffHeapMap: keys preserves insertion order', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('b', 2).set('a', 1).set('c', 3)
  t.deepEqual(map.keys(), ['b', 'a', 'c'])
})

test('OffHeapMap: values', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('a', 1).set('b', 2)
  t.deepEqual(map.values(), [1, 2])
})

test('OffHeapMap: entries', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('a', 1).set('b', 2)
  t.deepEqual(map.entries(), [
    ['a', 1],
    ['b', 2],
  ])
})

test('OffHeapMap: keys/values/entries on empty map return empty arrays', (t) => {
  const map = new OffHeapMap()
  t.deepEqual(map.keys(), [])
  t.deepEqual(map.values(), [])
  t.deepEqual(map.entries(), [])
})

test('OffHeapMap: forEach iterates in insertion order', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('x', 10).set('y', 20)
  const result: [unknown, unknown][] = []
  map.forEach((value, key) => result.push([key, value]))
  t.deepEqual(result, [
    ['x', 10],
    ['y', 20],
  ])
})

test('OffHeapMap: forEach callback can overwrite values without deadlock', (t) => {
  const map = new OffHeapMap<string, number>()
  map.set('a', 1).set('b', 2)
  map.forEach((_value, key) => {
    map.set(key as string, 99)
  })
  t.is(map.get('a'), 99)
  t.is(map.get('b'), 99)
})

test('OffHeapMap: forEach visits all entries even when current key is deleted', (t) => {
  // JS Map.forEach: deleting the current key does not skip the next entry.
  const map = new OffHeapMap<string, number>()
  map.set('a', 1).set('b', 2).set('c', 3)
  const visited: string[] = []
  map.forEach((_value, key) => {
    visited.push(key as string)
    map.delete(key as string)
  })
  t.deepEqual(visited, ['a', 'b', 'c'])
  t.is(map.size, 0)
})

test('OffHeapMap: forEach visits new entries added during iteration', (t) => {
  // JS Map.forEach: entries added during iteration are visited.
  const map = new OffHeapMap<string, number>()
  map.set('a', 1)
  const visited: string[] = []
  map.forEach((_value, key) => {
    visited.push(key as string)
    if (key === 'a') map.set('b', 2)
    if (key === 'b') map.set('c', 3)
  })
  t.deepEqual(visited, ['a', 'b', 'c'])
})

test('OffHeapMap: forEach on empty map does not invoke callback', (t) => {
  const map = new OffHeapMap()
  let called = false
  map.forEach(() => {
    called = true
  })
  t.false(called)
})

test('OffHeapMap: set returns this for chaining', (t) => {
  const map = new OffHeapMap()
  const ret = map.set('k', 1)
  t.is(ret, map)
})

test('OffHeapMap: all primitive key types coexist', (t) => {
  const map = new OffHeapMap()
  map.set('str', 1)
  map.set(42, 2)
  map.set(true, 3)
  map.set(null, 4)
  map.set(undefined, 5)
  t.is(map.size, 5)
  t.is(map.get('str'), 1)
  t.is(map.get(42), 2)
  t.is(map.get(true), 3)
  t.is(map.get(null), 4)
  t.is(map.get(undefined), 5)
})

test('OffHeapMap: [Symbol.iterator] yields [key, value] pairs in insertion order', (t) => {
  const map = new OffHeapMap()
  map.set('a', 1).set('b', 2).set('c', 3)
  t.deepEqual([...map], [['a', 1], ['b', 2], ['c', 3]])
})

test('OffHeapMap: for...of destructuring works', (t) => {
  const map = new OffHeapMap()
  map.set('x', 10).set('y', 20)
  const result: [unknown, unknown][] = []
  for (const [k, v] of map) {
    result.push([k, v])
  }
  t.deepEqual(result, [['x', 10], ['y', 20]])
})

test('OffHeapMap: [Symbol.iterator] on empty map yields nothing', (t) => {
  const map = new OffHeapMap()
  t.deepEqual([...map], [])
})
