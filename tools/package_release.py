"""Collect platform bundles and build Tauri's signed, multi-platform update index."""
from pathlib import Path
import json
import shutil
import sys
from urllib.parse import quote

source, output, tag = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
output.mkdir(parents=True, exist_ok=True)
version = tag.removeprefix('v')
platforms = {}
for directory in sorted(source.iterdir()):
    if not directory.is_dir():
        continue
    if directory.name == 'firmware':
        for file in directory.iterdir():
            if file.is_file():
                shutil.copyfile(file, output / file.name)
        assert json.loads((output / 'firmware.json').read_text())['version'] == version
        continue
    platform = directory.name.removeprefix('desktop-')
    for file in directory.rglob('*'):
        if not file.is_file() or not file.name.endswith(('.dmg', '.deb', '.rpm', '.AppImage', '.exe', '.msi', '.app.tar.gz')):
            continue
        if file.name.endswith('.app.tar.gz'):
            name = f'ESP.Gauge_{version}_{platform}.app.tar.gz'
        else:
            name = file.name.replace(' ', '.')
        shutil.copyfile(file, output / name)
        signature = Path(str(file) + '.sig')
        if signature.exists():
            shutil.copyfile(signature, output / (name + '.sig'))
            if name.endswith(('.app.tar.gz', '.AppImage', '.exe')):
                assert platform not in platforms, f'Duplicate updater platform: {platform}'
                platforms[platform] = {'signature': signature.read_text().strip(), 'url': f'https://github.com/Fallstop/esp-gauge/releases/download/{tag}/{quote(name)}'}
expected = {'darwin-aarch64', 'darwin-x86_64', 'linux-x86_64', 'windows-x86_64'}
assert set(platforms) == expected, f'Missing updater platforms: {expected - set(platforms)}'
assert (output / 'firmware.json.sig').exists(), 'Unsigned firmware manifest'
(output / 'latest.json').write_text(json.dumps({'version': version, 'notes': f'ESP Gauge {version}', 'platforms': platforms}, indent=2) + '\n')
print(f'{version}: {len(platforms)} signed app platforms and firmware ready')
