import test from 'ava'

import { OffHeapArray } from '../index'

test('OffHeapArray: constructor creates empty array', (t) => {
  const arr = new OffHeapArray()
  t.is(arr.length, 0)
})

test('OffHeapArray: push/get', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(3)
  t.is(arr.get(0), 1)
  t.is(arr.get(1), 2)
  t.is(arr.get(2), 3)
})

test('OffHeapArray: get returns undefined for out-of-bounds index', (t) => {
  const arr = new OffHeapArray<number>()
  t.is(arr.get(0), undefined)
  arr.push(1)
  t.is(arr.get(1), undefined)
  t.is(arr.get(99), undefined)
})

test('OffHeapArray: pop removes and returns last element', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2)
  t.is(arr.pop(), 2)
  t.is(arr.length, 1)
  t.is(arr.pop(), 1)
  t.is(arr.length, 0)
})

test('OffHeapArray: pop on empty array returns undefined', (t) => {
  const arr = new OffHeapArray<number>()
  t.is(arr.pop(), undefined)
})

test('OffHeapArray: set overwrites element at index', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2)
  arr.set(0, 99)
  t.is(arr.get(0), 99)
  t.is(arr.get(1), 2)
})

test('OffHeapArray: set throws on out-of-bounds index', (t) => {
  const arr = new OffHeapArray<number>()
  t.throws(() => arr.set(0, 1))
  arr.push(10)
  t.throws(() => arr.set(1, 1))
})

test('OffHeapArray: length', (t) => {
  const arr = new OffHeapArray<number>()
  t.is(arr.length, 0)
  arr.push(1)
  t.is(arr.length, 1)
  arr.push(2).push(3)
  t.is(arr.length, 3)
  arr.pop()
  t.is(arr.length, 2)
})

test('OffHeapArray: splice removes elements', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(3).push(4)
  const removed = arr.splice(1, 2, [])
  t.deepEqual(removed, [2, 3])
  t.is(arr.length, 2)
  t.is(arr.get(0), 1)
  t.is(arr.get(1), 4)
})

test('OffHeapArray: splice inserts elements', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(4)
  const removed = arr.splice(1, 0, [2, 3])
  t.deepEqual(removed, [])
  t.is(arr.length, 4)
  t.deepEqual([arr.get(0), arr.get(1), arr.get(2), arr.get(3)], [1, 2, 3, 4])
})

test('OffHeapArray: splice replaces elements', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(3)
  const removed = arr.splice(1, 1, [99])
  t.deepEqual(removed, [2])
  t.is(arr.length, 3)
  t.is(arr.get(1), 99)
})

test('OffHeapArray: splice with start beyond length clamps to end', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2)
  const removed = arr.splice(99, 1, [3])
  t.deepEqual(removed, [])
  t.is(arr.length, 3)
  t.is(arr.get(2), 3)
})

test('OffHeapArray: splice at index 0 replaces from beginning', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(3)
  const removed = arr.splice(0, 1, [99])
  t.deepEqual(removed, [1])
  t.is(arr.get(0), 99)
  t.is(arr.length, 3)
})

test('OffHeapArray: splice removes all elements', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(3)
  const removed = arr.splice(0, 3, [])
  t.deepEqual(removed, [1, 2, 3])
  t.is(arr.length, 0)
})

test('OffHeapArray: splice inserts many items preserving order', (t) => {
  // Validates that all inserted items land in the correct order when
  // more than one item is spliced in (the previous O(n×m) loop could
  // misplace items if Vec::insert shifted elements incorrectly).
  const arr = new OffHeapArray<number>()
  arr.push(1).push(6)
  arr.splice(1, 0, [2, 3, 4, 5])
  t.is(arr.length, 6)
  t.deepEqual(
    [arr.get(0), arr.get(1), arr.get(2), arr.get(3), arr.get(4), arr.get(5)],
    [1, 2, 3, 4, 5, 6],
  )
})

test('OffHeapArray: splice replaces fewer items with more (array grows)', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(10)
  const removed = arr.splice(1, 1, [3, 4, 5, 6, 7, 8, 9])
  t.deepEqual(removed, [2])
  t.is(arr.length, 9)
  t.deepEqual(
    Array.from({ length: 9 }, (_, i) => arr.get(i)),
    [1, 3, 4, 5, 6, 7, 8, 9, 10],
  )
})

test('OffHeapArray: splice replaces more items with fewer (array shrinks)', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(3).push(4).push(5).push(6)
  const removed = arr.splice(1, 4, [99])
  t.deepEqual(removed, [2, 3, 4, 5])
  t.is(arr.length, 3)
  t.is(arr.get(0), 1)
  t.is(arr.get(1), 99)
  t.is(arr.get(2), 6)
})

test('OffHeapArray: splice tail is intact after multi-item insert', (t) => {
  // Ensures elements after the insertion point are not corrupted.
  // splice(1, 0, [10,20,30]) on [0,100,200] → [0,10,20,30,100,200]
  // The original tail (100, 200) shifts right by 3 (number of inserted items).
  const arr = new OffHeapArray<number>()
  arr.push(0).push(100).push(200)
  arr.splice(1, 0, [10, 20, 30])
  t.is(arr.length, 6)
  t.is(arr.get(4), 100)
  t.is(arr.get(5), 200)
})

test('OffHeapArray: forEach iterates with correct indices', (t) => {
  const arr = new OffHeapArray<string>()
  arr.push('a').push('b').push('c')
  const result: [string, number][] = []
  arr.forEach((value, index) => result.push([value as string, index as number]))
  t.deepEqual(result, [
    ['a', 0],
    ['b', 1],
    ['c', 2],
  ])
})

test('OffHeapArray: forEach callback can mutate elements without deadlock', (t) => {
  const arr = new OffHeapArray<number>()
  arr.push(1).push(2).push(3)
  arr.forEach((_value, index) => {
    arr.set(index as number, (index as number) * 10)
  })
  t.is(arr.get(0), 0)
  t.is(arr.get(1), 10)
  t.is(arr.get(2), 20)
})

test('OffHeapArray: forEach on empty array does not invoke callback', (t) => {
  const arr = new OffHeapArray()
  let called = false
  arr.forEach(() => {
    called = true
  })
  t.false(called)
})

test('OffHeapArray: push returns this for chaining', (t) => {
  const arr = new OffHeapArray()
  const ret = arr.push(1)
  t.is(ret, arr)
})

test('OffHeapArray: stores all primitive types', (t) => {
  const arr = new OffHeapArray()
  arr.push('str').push(42).push(1.5).push(true).push(null).push(undefined)
  t.is(arr.get(0), 'str')
  t.is(arr.get(1), 42)
  t.is(arr.get(2), 1.5)
  t.is(arr.get(3), true)
  t.is(arr.get(4), null)
  t.is(arr.get(5), undefined)
})
