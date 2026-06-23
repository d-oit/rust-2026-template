#!/usr/bin/env node
/**
 * export_excalidraw.mjs — Convert Excalidraw scene JSON to SVG or PNG.
 *
 * Uses @moona3k/excalidraw-export for pure-computation rendering (no browser).
 *
 * Usage:
 *   node scripts/export_excalidraw.mjs \
 *     --input .template/architecture.excalidraw \
 *     --output .template/architecture.svg \
 *     --format svg
 */

import { readFileSync, writeFileSync } from "node:fs";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    input:  { type: "string", short: "i" },
    output: { type: "string", short: "o" },
    format: { type: "string", short: "f", default: "svg" },
  },
});

if (!values.input || !values.output) {
  console.error("Usage: node export_excalidraw.mjs -i <input.excalidraw> -o <output> -f svg|png");
  process.exit(1);
}

async function main() {
  const { renderToSvg, exportDiagram } = await import("@moona3k/excalidraw-export");
  const scene = JSON.parse(readFileSync(values.input, "utf-8"));

  if (values.format === "svg") {
    const svgString = renderToSvg(scene);
    writeFileSync(values.output, svgString);
    console.log(`Written: ${values.output}`);
  } else if (values.format === "png") {
    await exportDiagram(values.input, values.output, { scale: 2 });
    console.log(`Written: ${values.output}`);
  } else {
    console.error(`Unknown format: ${values.format}`);
    process.exit(1);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
