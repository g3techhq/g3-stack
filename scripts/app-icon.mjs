// Renders an app's icon master into the PNG files the stores and `dx` need.
//
//   node scripts/app-icon.mjs <path/to/icon.svg>
//
// Writes, next to the SVG:
//   icon-1024.png  `[bundle] icon` in Dioxus.toml. dx compiles it into the
//                  iOS asset catalog and builds the Android launcher icons
//                  from it. Also the App Store listing icon.
//   icon-512.png   The Google Play listing icon.
//
// Both are opaque RGB: App Store Connect rejects an icon with an alpha
// channel, even a fully opaque one. The master must be full bleed (every
// platform applies its own mask), and its mark must sit inside the centre
// 62%: dx shrinks the artwork to 68% for the Android foreground layer, and
// the launcher shows only the middle two thirds of it.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { crc32, deflateSync } from "node:zlib";
import { chromium } from "@playwright/test";

const SIZES = [
  ["icon-1024.png", 1024],
  ["icon-512.png", 512],
];

/** Encodes 8-bit RGB pixels as a PNG (colour type 2, no alpha). */
export function encodeRgbPng(width, height, rgb) {
  const row = width * 3;
  const raw = Buffer.alloc((row + 1) * height);
  for (let y = 0; y < height; y++) {
    // Filter byte 0 (none), then the row.
    rgb.copy(raw, y * (row + 1) + 1, y * row, (y + 1) * row);
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
  header[9] = 2; // colour type: RGB
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Drops the alpha channel, refusing any pixel that is not fully opaque. */
export function opaqueRgb(rgba) {
  const rgb = Buffer.alloc((rgba.length / 4) * 3);
  for (let i = 0, j = 0; i < rgba.length; i += 4, j += 3) {
    if (rgba[i + 3] !== 255) {
      throw new Error("The icon master must be opaque edge to edge (full bleed).");
    }
    rgb[j] = rgba[i];
    rgb[j + 1] = rgba[i + 1];
    rgb[j + 2] = rgba[i + 2];
  }
  return rgb;
}

async function render(svgPath) {
  const svg = readFileSync(svgPath).toString("base64");
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    for (const [name, size] of SIZES) {
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
      const png = encodeRgbPng(size, size, opaqueRgb(Buffer.from(pixels)));
      const out = join(dirname(svgPath), name);
      writeFileSync(out, png);
      console.log(`${out} (${size}×${size}, RGB)`);
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
