// Adapted from Pixel Agents, webview-ui/src/office/projection.ts.
// Copyright (c) 2026 Pablo De Lucca. MIT license: ../../third_party/PIXEL_AGENTS_LICENSE.txt
const TILE_SIZE = 16;

export function mapOffset(canvasWidth: number, canvasHeight: number, cols: number, rows: number, zoom: number, panX: number, panY: number) {
  const mapW = cols * TILE_SIZE * zoom;
  const mapH = rows * TILE_SIZE * zoom;
  return {
    offsetX: Math.floor((canvasWidth - mapW) / 2) + Math.round(panX),
    offsetY: Math.floor((canvasHeight - mapH) / 2) + Math.round(panY),
  };
}
