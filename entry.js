'use strict'

const binding = require('./index.js')

// [Symbol.iterator] yields [key, value] pairs, matching native Map behavior
binding.OffHeapMap.prototype[Symbol.iterator] = function () {
  return this.entries()[Symbol.iterator]()
}

// [Symbol.iterator] yields values, matching native Set behavior
binding.OffHeapSet.prototype[Symbol.iterator] = function () {
  return this.values()[Symbol.iterator]()
}

// [Symbol.iterator] yields values lazily by index, matching native Array behavior
binding.OffHeapArray.prototype[Symbol.iterator] = function* () {
  const len = this.length
  for (let i = 0; i < len; i++) {
    yield this.get(i)
  }
}

module.exports = binding
module.exports.OffHeapArray = binding.OffHeapArray
module.exports.OffHeapMap = binding.OffHeapMap
module.exports.OffHeapObject = binding.OffHeapObject
module.exports.OffHeapSet = binding.OffHeapSet
