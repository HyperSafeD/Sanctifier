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
const MANIFEST_PATH = path.join(__dirname, "manifest.json");

/**
 * Validates that generated icon sizes match those declared in manifest.json
 */
function validateIconSizes() {
  try {
    const manifestContent = fs.readFileSync(MANIFEST_PATH, "utf8");
    const manifest = JSON.parse(manifestContent);

    // Extract icon sizes from manifest
    const manifestSizes = new Set();

    // Check action.default_icon if it exists
    if (manifest.action && manifest.action.default_icon) {
      const icon = manifest.action.default_icon;
      if (typeof icon === "object") {
        Object.keys(icon).forEach((size) => manifestSizes.add(parseInt(size)));
      }
    }

    // Check icons field if it exists
    if (manifest.icons) {
      Object.keys(manifest.icons).forEach((size) =>
        manifestSizes.add(parseInt(size)),
      );
    }

    // Compare with SIZES array
    const missingSizes = Array.from(manifestSizes).filter(
      (size) => !SIZES.includes(size),
    );
    const extraSizes = SIZES.filter(
      (size) => !manifestSizes.has(size) && manifestSizes.size > 0,
    );

    if (missingSizes.length > 0) {
      console.warn(
        `⚠️  Warning: manifest.json declares icon sizes [${Array.from(manifestSizes).join(", ")}] but build-icons.js only generates [${SIZES.join(", ")}]`,
      );
      console.warn(`   Missing sizes: ${missingSizes.join(", ")}`);
      return false;
    }

    if (extraSizes.length > 0 && manifestSizes.size > 0) {
      console.warn(
        `⚠️  Warning: build-icons.js generates sizes [${SIZES.join(", ")}] but manifest.json only declares [${Array.from(manifestSizes).join(", ")}]`,
      );
      console.warn(`   Extra sizes: ${extraSizes.join(", ")}`);
      return false;
    }

    console.log(`✓ Icon sizes match manifest.json: [${SIZES.join(", ")}]`);
    return true;
  } catch (error) {
    console.error("Error validating icon sizes:", error.message);
    return false;
  }
}

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
      execSync(
        `convert -background none -resize ${size}x${size} "${SVG}" "${out}"`,
      );
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
    "Could not generate icons. Install sharp (npm install sharp), ImageMagick, or librsvg.",
  );
  process.exit(1);
}

build().then(() => {
  // Validate that generated sizes match manifest.json
  const isValid = validateIconSizes();
  if (!isValid) {
    console.error(
      "\n❌ Icon size mismatch detected. Please update SIZES in build-icons.js or manifest.json.",
    );
    process.exit(1);
  }
  console.log("\n✅ Icon generation complete and validated.");
});
