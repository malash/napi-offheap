/* eslint-disable */

export type Primitive = string | number | boolean | null | undefined

export declare class OffHeapObject<T extends Record<string, unknown> = Record<string, unknown>> {
  constructor()
  /** Number keys are coerced to strings, matching JS object semantics. */
  set<K extends keyof T & string>(key: K | number, value: T[K]): this
  get<K extends keyof T & string>(key: K | number): T[K] | undefined
  has(key: string | number): boolean
  delete(key: string | number): boolean
  clear(): void
  get size(): number
  keys(): Array<keyof T & string>
  values(): Array<T[keyof T & string]>
  entries(): Array<[keyof T & string, T[keyof T & string]]>
  forEach(callback: (value: T[keyof T & string], key: keyof T & string) => unknown): void
}

export declare class OffHeapArray<T = unknown> {
  constructor()
  push(value: T): this
  pop(): T | undefined
  get(index: number): T | undefined
  /** throws if index is out of bounds */
  set(index: number, value: T): void
  get length(): number
  splice(start: number, deleteCount: number, items: T[]): T[]
  forEach(callback: (value: T, index: number) => unknown): void
  [Symbol.iterator](): IterableIterator<T>
}

export declare class OffHeapMap<K extends Primitive = Primitive, V = unknown> {
  constructor()
  set(key: K, value: V): this
  get(key: K): V | undefined
  has(key: K): boolean
  delete(key: K): boolean
  clear(): void
  get size(): number
  keys(): K[]
  values(): V[]
  entries(): [K, V][]
  forEach(callback: (value: V, key: K) => unknown): void
  [Symbol.iterator](): IterableIterator<[K, V]>
}

export declare class OffHeapSet<T extends Primitive = Primitive> {
  constructor()
  add(value: T): this
  has(value: T): boolean
  delete(value: T): boolean
  clear(): void
  get size(): number
  values(): T[]
  /** callback receives (value, value) per the JS Set.forEach spec */
  forEach(callback: (value: T, _value: T) => unknown): void
  [Symbol.iterator](): IterableIterator<T>
}
