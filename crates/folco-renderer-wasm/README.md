# folco-renderer-wasm

WASM bindings for [folco-renderer](https://github.com/ecoates2/folco-renderer), enabling HTML canvas rendering in web environments.

## Installation

```bash
bun add folco-renderer-wasm
```

## Usage

### In a Tauri/Vite Application

```typescript
import init, { CanvasRenderer } from 'folco-renderer-wasm';

// Initialize the WASM module (do this once at app startup)
await init();

// Get your canvas element
const canvas = document.getElementById('preview-canvas') as HTMLCanvasElement;

// Load a PNG as Uint8Array (e.g., from fetch or file input)
const response = await fetch('/path/to/icon.png');
const pngData = new Uint8Array(await response.arrayBuffer());

// Create renderer with base icon
const renderer = CanvasRenderer.fromPng(pngData, 1.0);

// Apply customizations
renderer.setHueRotation(180.0);

// Render to canvas at specified size
renderer.renderToCanvas(canvas, 256);

// Export the customization profile as JSON when done
const profileJson = renderer.exportProfileJson();
```

### With Multiple Resolutions

```typescript
const scales = [1.0, 2.0];
const pngArrays = [png1x, png2x]; // Uint8Array[]

const renderer = CanvasRenderer.fromPngMultiple(pngArrays, new Float32Array(scales));
```

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

### Build Commands

```bash
# Build for web (ES modules)
wasm-pack build --target web

# Build release version
wasm-pack build --target web --release

# Pack without publishing (creates tarball)
wasm-pack pack

# Publish to npm
wasm-pack publish --access public
```

## Local Development with folco-gui

For local development, reference the built `pkg` directory in your folco-gui `package.json`:

```json
{
  "dependencies": {
    "folco-renderer-wasm": "file:../folco-renderer-wasm/pkg"
  }
}
```

Then rebuild whenever you make changes:

```bash
wasm-pack build --target web
```

## Publishing

This package uses `wasm-pack publish` for npm publishing.

### Manual Publishing

```bash
wasm-pack build --target web --release
wasm-pack publish --access public
```

### Automated Publishing (GitHub Actions)

1. Add `NPM_TOKEN` secret to your GitHub repository settings
2. Tag a release:
   ```bash
   git tag v0.1.0
   git push --tags
   ```
3. The workflow will automatically build and publish to npm

### Tagged Releases

```bash
wasm-pack publish --access public --tag beta
```

## API Reference

### `CanvasRenderer`

The main class for rendering icons to HTML canvas elements.

#### Static Methods

- `fromPng(pngData: Uint8Array, scale: number): CanvasRenderer` - Create from a single PNG
- `fromPngMultiple(pngDataArray: Uint8Array[], scales: Float32Array): CanvasRenderer` - Create from multiple PNG images

#### Instance Methods

- `setHueRotation(degrees: number | null)` - Set hue rotation angle (null to disable)
- `setHueRotationEnabled(enabled: boolean)` - Toggle hue rotation without changing angle
- `setDecal(svgData: string | null, scale: number)` - Set decal overlay
- `renderToCanvas(canvas: HTMLCanvasElement, size: number)` - Render to canvas
- `exportProfileJson(): string` - Export customization settings as JSON

## License

MIT
