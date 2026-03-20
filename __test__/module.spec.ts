/**
 * Build artifact / module structure tests.
 *
 * These tests verify that the compiled native module:
 *   - exports all expected classes
 *   - registers each class so that instanceof works correctly
 *   - does not accidentally expose unrelated constructors
 */
import test from 'ava'

import * as Module from '../index'
import { OffHeapArray, OffHeapMap, OffHeapObject, OffHeapSet } from '../index'

// ─── Exports ─────────────────────────────────────────────────────────────────

test('module: OffHeapObject is exported', (t) => {
  t.is(typeof Module.OffHeapObject, 'function')
})

test('module: OffHeapMap is exported', (t) => {
  t.is(typeof Module.OffHeapMap, 'function')
})

test('module: OffHeapArray is exported', (t) => {
  t.is(typeof Module.OffHeapArray, 'function')
})

test('module: OffHeapSet is exported', (t) => {
  t.is(typeof Module.OffHeapSet, 'function')
})

test('module: all four OffHeap classes are exported', (t) => {
  // Check presence without asserting exact key set — the CJS/ESM bridge from
  // napi-rs injects additional keys like `default` and `module.exports`.
  const keys = Object.keys(Module)
  t.true(keys.includes('OffHeapArray'))
  t.true(keys.includes('OffHeapMap'))
  t.true(keys.includes('OffHeapObject'))
  t.true(keys.includes('OffHeapSet'))
})

// ─── instanceof ──────────────────────────────────────────────────────────────

test('module: OffHeapObject instanceof', (t) => {
  const obj = new OffHeapObject()
  t.true(obj instanceof OffHeapObject)
})

test('module: OffHeapMap instanceof', (t) => {
  const map = new OffHeapMap()
  t.true(map instanceof OffHeapMap)
})

test('module: OffHeapArray instanceof', (t) => {
  const arr = new OffHeapArray()
  t.true(arr instanceof OffHeapArray)
})

test('module: OffHeapSet instanceof', (t) => {
  const set = new OffHeapSet()
  t.true(set instanceof OffHeapSet)
})

// ─── Cross-class instanceof ───────────────────────────────────────────────────

test('module: OffHeapMap is not instanceof OffHeapArray', (t) => {
  const map = new OffHeapMap()
  t.false(map instanceof OffHeapArray)
})

test('module: OffHeapArray is not instanceof OffHeapMap', (t) => {
  const arr = new OffHeapArray()
  t.false(arr instanceof OffHeapMap)
})

test('module: OffHeapSet is not instanceof OffHeapMap', (t) => {
  const set = new OffHeapSet()
  t.false(set instanceof OffHeapMap)
})

test('module: OffHeapObject is not instanceof OffHeapMap', (t) => {
  const obj = new OffHeapObject()
  t.false(obj instanceof OffHeapMap)
})

// ─── Initial state ───────────────────────────────────────────────────────────

test('module: fresh OffHeapObject has size 0', (t) => {
  t.is(new OffHeapObject().size, 0)
})

test('module: fresh OffHeapMap has size 0', (t) => {
  t.is(new OffHeapMap().size, 0)
})

test('module: fresh OffHeapArray has length 0', (t) => {
  t.is(new OffHeapArray().length, 0)
})

test('module: fresh OffHeapSet has size 0', (t) => {
  t.is(new OffHeapSet().size, 0)
})

// ─── Accepted container types as values ──────────────────────────────────────

test('module: OffHeapMap accepts all four OffHeap types as values', (t) => {
  const map = new OffHeapMap()
  t.notThrows(() => {
    map.set('obj', new OffHeapObject())
    map.set('arr', new OffHeapArray())
    map.set('map', new OffHeapMap())
    map.set('set', new OffHeapSet())
  })
  t.is(map.size, 4)
})

test('module: OffHeapArray accepts all four OffHeap types as elements', (t) => {
  const arr = new OffHeapArray()
  t.notThrows(() => {
    arr.push(new OffHeapObject())
    arr.push(new OffHeapArray())
    arr.push(new OffHeapMap())
    arr.push(new OffHeapSet())
  })
  t.is(arr.length, 4)
})

test('module: OffHeapObject accepts all four OffHeap types as values', (t) => {
  const obj = new OffHeapObject()
  t.notThrows(() => {
    obj.set('obj', new OffHeapObject())
    obj.set('arr', new OffHeapArray())
    obj.set('map', new OffHeapMap())
    obj.set('set', new OffHeapSet())
  })
  t.is(obj.size, 4)
})

// ─── Retrieved containers retain their type ───────────────────────────────────

test('module: container retrieved from OffHeapMap is instanceof correct class', (t) => {
  const map = new OffHeapMap()
  const inner = new OffHeapArray()
  map.set('arr', inner)
  const retrieved = map.get('arr')
  t.true(retrieved instanceof OffHeapArray)
  t.false(retrieved instanceof OffHeapMap)
})
