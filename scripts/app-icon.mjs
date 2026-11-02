// Renders an app's icon master into the PNG files the stores and `dx` need.
//
//   node scripts/app-icon.mjs <path/to/icon.svg>
//
// Writes, next to the SVG:
//   icon-1024.png  `[ios] icon` in Dioxus.toml: dx compiles it into the iOS
//                  asset catalog. RGB with no alpha channel, which App Store
//                  Connect requires even of a fully opaque icon.
//   icon-512.png   The Google Play listing icon, a 32-bit PNG as Play asks.
//
// Android's launcher icons come from the SVG itself (`[bundle] icon`): dx's
// PNG path panics on an RGB image and never creates `mipmap-anydpi-v26`.
// The master must be full bleed (every platform applies its own mask), and
// its mark must sit inside the centre 62%: dx shrinks the artwork to 68% for
// the Android foreground layer, and the launcher shows only the middle two
// thirds of it.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { crc32, deflateSync } from "node:zlib";
import { chromium } from "@playwright/test";

/** `[file, size, channels]`: 3 is RGB, 4 is RGBA. */
const OUTPUTS = [
  ["icon-1024.png", 1024, 3],
  ["icon-512.png", 512, 4],
];

/** Encodes 8-bit RGB (3 channels) or RGBA (4) pixels as a PNG. */
export function encodePng(width, height, pixels, channels) {
  const row = width * channels;
  const raw = Buffer.alloc((row + 1) * height);
  for (let y = 0; y < height; y++) {
    // Filter byte 0 (none), then the row.
    pixels.copy(raw, y * (row + 1) + 1, y * row, (y + 1) * row);
  }
  const chunk = (type, data) => {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body));
    return Buffer.concat([length, body, crc]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8; // bit depth
  header[9] = channels === 4 ? 6 : 2; // colour type: RGBA or RGB
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Refuses any pixel that is not fully opaque, then keeps `channels` of each. */
export function opaquePixels(rgba, channels) {
  const out = Buffer.alloc((rgba.length / 4) * channels);
  for (let i = 0, j = 0; i < rgba.length; i += 4, j += channels) {
    if (rgba[i + 3] !== 255) {
      throw new Error("The icon master must be opaque edge to edge (full bleed).");
    }
    rgba.copy(out, j, i, i + channels);
  }
  return out;
}

async function render(svgPath) {
  const svg = readFileSync(svgPath).toString("base64");
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    for (const [name, size, channels] of OUTPUTS) {
      const pixels = await page.evaluate(
        async ({ svg, size }) => {
          const image = new Image();
          image.src = `data:image/svg+xml;base64,${svg}`;
          await image.decode();
          const canvas = document.createElement("canvas");
          canvas.width = size;
          canvas.height = size;
          const context = canvas.getContext("2d");
          context.drawImage(image, 0, 0, size, size);
          return Array.from(context.getImageData(0, 0, size, size).data);
        },
        { svg, size },
      );
      const png = encodePng(size, size, opaquePixels(Buffer.from(pixels), channels), channels);
      const out = join(dirname(svgPath), name);
      writeFileSync(out, png);
      console.log(`${out} (${size}×${size}, ${channels === 4 ? "RGBA" : "RGB"})`);
    }
  } finally {
    await browser.close();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const svgPath = process.argv[2];
  if (!svgPath) {
    console.error("usage: node scripts/app-icon.mjs <path/to/icon.svg>");
    process.exit(2);
  }
  await render(resolve(svgPath));
}
