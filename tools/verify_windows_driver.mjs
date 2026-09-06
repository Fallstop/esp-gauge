import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const driver = new URL('../desktop/src-tauri/windows/drivers/CH341SER.EXE', import.meta.url);
const expected = '458c37bdafbe4ce3cd0baf728c232b4b765b36d7463956e9b94cdf099212cad1';
const actual = createHash('sha256').update(readFileSync(driver)).digest('hex');
if (actual !== expected) {
  throw new Error(`Bundled WCH driver checksum mismatch: expected ${expected}, got ${actual}`);
}
console.log('Bundled WCH CH340/CH341 driver 4.0 checksum verified.');
