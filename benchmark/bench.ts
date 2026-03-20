/**
 * GC pause benchmark
 *
 * Demonstrates how large long-lived data on the V8 heap causes Mark-Compact
 * GC pauses, and how moving that data off-heap eliminates the problem.
 *
 * Requires --expose-gc (added by the bench npm script).
 *
 * Methodology:
 *   - beforeAll: allocate 20M long-lived elements, promote to old generation
 *   - each iteration: allocate 500K temp objects, then call gc() and report the pause time
 *   - afterAll: release the dataset and GC it away before the next task starts
 */

import { Bench } from 'tinybench'

import { OffHeapArray } from '../index.js'

declare function gc(): void // exposed by --expose-gc

const LARGE_N = 20_000_000
const TEMP_N = 500_000

const bench = new Bench({ warmup: false, iterations: 10 })

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let keepAlive: any = null

bench.add(
  'JS Array — GC pause with 20M live objects',
  () => {
    const temp: { x: number }[] = []
    for (let i = 0; i < TEMP_N; i++) temp.push({ x: i })
    void temp

    const t = performance.now()
    gc()
    return { overriddenDuration: performance.now() - t }
  },
  {
    beforeAll() {
      const arr: { data: number }[] = []
      for (let i = 0; i < LARGE_N; i++) arr.push({ data: i })
      keepAlive = arr
      gc()
      gc()
    },
    afterAll() {
      keepAlive = null
      gc()
    },
  },
)

bench.add(
  'OffHeapArray — GC pause with 20M live elements',
  () => {
    const temp: { x: number }[] = []
    for (let i = 0; i < TEMP_N; i++) temp.push({ x: i })
    void temp

    const t = performance.now()
    gc()
    return { overriddenDuration: performance.now() - t }
  },
  {
    beforeAll() {
      const arr = new OffHeapArray<number>()
      for (let i = 0; i < LARGE_N; i++) arr.push(i)
      keepAlive = arr
      gc()
      gc()
    },
    afterAll() {
      keepAlive = null
      gc()
    },
  },
)

await bench.run()

console.table(bench.table())
