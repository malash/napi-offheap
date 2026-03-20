import test from 'ava'

import { OffHeapArray, OffHeapMap, OffHeapObject, OffHeapSet } from '../entry'

// ─── Single-level nesting ────────────────────────────────────────────────────

test('nesting: OffHeapMap inside OffHeapArray — shared reference', (t) => {
  const inner = new OffHeapMap<string, number>()
  inner.set('x', 1)
  const arr = new OffHeapArray<OffHeapMap<string, number>>()
  arr.push(inner)
  const ref = arr.get(0) as OffHeapMap<string, number>
  ref.set('x', 99)
  t.is(inner.get('x'), 99)
})

test('nesting: OffHeapArray inside OffHeapMap — shared reference', (t) => {
  const inner = new OffHeapArray<number>()
  inner.push(1).push(2)
  const map = new OffHeapMap<string, OffHeapArray<number>>()
  map.set('arr', inner)
  const ref = map.get('arr') as OffHeapArray<number>
  ref.push(3)
  t.is(inner.length, 3)
})

test('nesting: OffHeapObject inside OffHeapMap — shared reference', (t) => {
  const obj = new OffHeapObject<{ n: number }>()
  obj.set('n', 42)
  const map = new OffHeapMap<string, OffHeapObject<{ n: number }>>()
  map.set('obj', obj)
  const ref = map.get('obj') as OffHeapObject<{ n: number }>
  ref.set('n', 100)
  t.is(obj.get('n'), 100)
})

test('nesting: OffHeapSet inside OffHeapArray — shared reference', (t) => {
  const inner = new OffHeapSet<number>()
  inner.add(1).add(2)
  const arr = new OffHeapArray<OffHeapSet<number>>()
  arr.push(inner)
  const ref = arr.get(0) as OffHeapSet<number>
  ref.add(3)
  t.true(inner.has(3))
})

test('nesting: OffHeapObject inside OffHeapObject — shared reference', (t) => {
  const child = new OffHeapObject<{ v: number }>()
  child.set('v', 1)
  const parent = new OffHeapObject<{ child: OffHeapObject<{ v: number }> }>()
  parent.set('child', child)
  const ref = parent.get('child') as OffHeapObject<{ v: number }>
  ref.set('v', 2)
  t.is(child.get('v'), 2)
})

// ─── Multi-level nesting ─────────────────────────────────────────────────────

test('nesting: OffHeapArray inside OffHeapArray — shared reference', (t) => {
  const inner = new OffHeapArray<number>()
  inner.push(1)
  const outer = new OffHeapArray<OffHeapArray<number>>()
  outer.push(inner)
  const ref = outer.get(0) as OffHeapArray<number>
  ref.push(2)
  t.is(inner.length, 2)
  t.is(inner.get(1), 2)
})

test('nesting: OffHeapMap inside OffHeapMap — shared reference', (t) => {
  const inner = new OffHeapMap<string, number>()
  inner.set('v', 10)
  const outer = new OffHeapMap<string, OffHeapMap<string, number>>()
  outer.set('inner', inner)
  const ref = outer.get('inner') as OffHeapMap<string, number>
  ref.set('v', 20)
  t.is(inner.get('v'), 20)
})

test('nesting: same container referenced from multiple parents', (t) => {
  const shared = new OffHeapArray<number>()
  shared.push(1)
  const map1 = new OffHeapMap<string, OffHeapArray<number>>()
  const map2 = new OffHeapMap<string, OffHeapArray<number>>()
  map1.set('arr', shared)
  map2.set('arr', shared)
  // Mutate via map1's reference
  ;(map1.get('arr') as OffHeapArray<number>).push(2)
  // map2 sees the change too
  t.is((map2.get('arr') as OffHeapArray<number>).length, 2)
  t.is(shared.length, 2)
})

test('nesting: three levels deep — mutations visible at every level', (t) => {
  const leaf = new OffHeapMap<string, number>()
  leaf.set('val', 0)
  const mid = new OffHeapArray<OffHeapMap<string, number>>()
  mid.push(leaf)
  const root = new OffHeapObject<{ mid: OffHeapArray<OffHeapMap<string, number>> }>()
  root.set('mid', mid)

  // Mutate through the full chain
  const midRef = root.get('mid') as OffHeapArray<OffHeapMap<string, number>>
  const leafRef = midRef.get(0) as OffHeapMap<string, number>
  leafRef.set('val', 42)

  t.is(leaf.get('val'), 42)
})
