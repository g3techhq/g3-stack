import assert from "node:assert/strict";
import { test } from "node:test";
import { inflateSync } from "node:zlib";
import { encodeRgbPng, opaqueRgb } from "./app-icon.mjs";

test("encodes an RGB PNG without an alpha channel", () => {
  const png = encodeRgbPng(2, 1, Buffer.from([255, 0, 0, 0, 0, 255]));
  assert.deepEqual([...png.subarray(1, 4)], [0x50, 0x4e, 0x47]);
  assert.equal(png.readUInt32BE(16), 2);
  assert.equal(png.readUInt32BE(20), 1);
  assert.equal(png[25], 2, "colour type 2 is RGB");
  const length = png.readUInt32BE(33);
  const pixels = inflateSync(png.subarray(41, 41 + length));
  assert.deepEqual([...pixels], [0, 255, 0, 0, 0, 0, 255]);
});

test("refuses a master with transparent pixels", () => {
  assert.deepEqual([...opaqueRgb(Buffer.from([1, 2, 3, 255]))], [1, 2, 3]);
  assert.throws(() => opaqueRgb(Buffer.from([1, 2, 3, 0])), /opaque/);
});
