import assert from "node:assert/strict";
import { test } from "node:test";
import { inflateSync } from "node:zlib";
import { encodePng, opaquePixels } from "./app-icon.mjs";

test("encodes an RGB PNG without an alpha channel", () => {
  const png = encodePng(2, 1, Buffer.from([255, 0, 0, 0, 0, 255]), 3);
  assert.deepEqual([...png.subarray(1, 4)], [0x50, 0x4e, 0x47]);
  assert.equal(png.readUInt32BE(16), 2);
  assert.equal(png.readUInt32BE(20), 1);
  assert.equal(png[25], 2, "colour type 2 is RGB");
  const length = png.readUInt32BE(33);
  const pixels = inflateSync(png.subarray(41, 41 + length));
  assert.deepEqual([...pixels], [0, 255, 0, 0, 0, 0, 255]);
});

test("encodes RGBA with colour type 6", () => {
  assert.equal(encodePng(1, 1, Buffer.from([1, 2, 3, 255]), 4)[25], 6);
});

test("refuses a master with transparent pixels", () => {
  assert.deepEqual([...opaquePixels(Buffer.from([1, 2, 3, 255]), 3)], [1, 2, 3]);
  assert.deepEqual([...opaquePixels(Buffer.from([1, 2, 3, 255]), 4)], [1, 2, 3, 255]);
  assert.throws(() => opaquePixels(Buffer.from([1, 2, 3, 0]), 3), /opaque/);
});
