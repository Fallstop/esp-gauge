"""Assemble the signed release's fixed flash regions, leaving NVS untouched."""
from pathlib import Path
import hashlib
import json
import re
import shutil
import sys

root = Path(__file__).resolve().parents[1]
output = Path(sys.argv[1]) if len(sys.argv) > 1 else root / 'artifacts/firmware'
output.mkdir(parents=True, exist_ok=True)
version = re.search(r'FIRMWARE_VERSION "([^"]+)"', (root / 'firmware/include/version.h').read_text())[1]
settings = json.loads((root / 'desktop/src-tauri/tauri.conf.json').read_text())
assert version == settings['version'], 'Firmware and desktop versions must agree for a release'
build = root / 'firmware/.pio/build/esp32dev'
boot_app = Path.home() / '.platformio/packages/framework-arduinoespressif32/tools/partitions/boot_app0.bin'
segments = []
for name, offset, source in [('bootloader.bin', 0x1000, build), ('partitions.bin', 0x8000, build), ('boot_app0.bin', 0xe000, boot_app.parent), ('firmware.bin', 0x10000, build)]:
    data = (source / name).read_bytes()
    shutil.copyfile(source / name, output / name)
    segments.append(dict(name=name, offset=offset, size=len(data), sha256=hashlib.sha256(data).hexdigest()))
manifest = dict(version=version, chip='esp32', layout='esp32-4mb-huge-app-v1', segments=segments)
(output / 'firmware.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(f'Firmware {version}: {output}')
