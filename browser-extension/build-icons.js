/**
 * Generates PNG icons from icon.svg for the browser extension.
 * Requires `sharp` (npm install sharp) or ImageMagick (`convert`).
 *
 * Usage:
 *   node build-icons.js
 *   # or
 *   npm run build:ext-icons
 */

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const SIZES = [16, 48, 128];
const SVG = path.join(__dirname, "icon.svg");

async function build() {
  // Try sharp first (npm package)
  try {
    const sharp = require("sharp");
    for (const size of SIZES) {
      const out = path.join(__dirname, `icon-${size}.png`);
      await sharp(SVG).resize(size, size).png().toFile(out);
      console.log(`Generated ${out}`);
    }
    return;
  } catch {
    // sharp not available, try ImageMagick
  }

  // Try ImageMagick convert
  try {
    for (const size of SIZES) {
      const out = path.join(__dirname, `icon-${size}.png`);
      execSync(`convert -background none -resize ${size}x${size} "${SVG}" "${out}"`);
      console.log(`Generated ${out}`);
    }
    return;
  } catch {
    // ImageMagick not available either
  }

  // Try rsvg-convert
  try {
    for (const size of SIZES) {
      const out = path.join(__dirname, `icon-${size}.png`);
      execSync(`rsvg-convert -w ${size} -h ${size} "${SVG}" -o "${out}"`);
      console.log(`Generated ${out}`);
    }
    return;
  } catch {
    // None available
  }

  console.error(
    "Could not generate icons. Install sharp (npm install sharp), ImageMagick, or librsvg."
  );
  process.exit(1);
}

build();
