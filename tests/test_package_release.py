"""Release downloads stay easy to identify without breaking signed updates."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / 'tools/package_release.py'
VERSION = '2.2.4'


class ReleasePackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.source = Path(self.temp.name) / 'collected'
        self.output = Path(self.temp.name) / 'release'
        self.assets = {}
        for platform, original, renamed in [
            ('darwin-aarch64', 'ESP Gauge.app.tar.gz', 'macOS-Apple-Silicon.app.tar.gz'),
            ('darwin-aarch64', 'ESP Gauge_2.2.4_aarch64.dmg', 'macOS-Apple-Silicon.dmg'),
            ('linux-x86_64', 'ESP Gauge_2.2.4_amd64.AppImage', 'Linux-x64.AppImage'),
            ('linux-x86_64', 'ESP Gauge_2.2.4_amd64.deb', 'Linux-x64.deb'),
            ('linux-x86_64', 'ESP Gauge-2.2.4-1.x86_64.rpm', 'Linux-x64.rpm'),
            ('windows-x86_64', 'ESP Gauge_2.2.4_x64-setup.exe', 'Windows-x64-Setup.exe'),
            ('windows-x86_64', 'ESP Gauge_2.2.4_x64_en-US.msi', 'Windows-x64.msi'),
        ]:
            file = self.source / f'desktop-{platform}' / 'bundle' / original
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(f'original bytes: {original}'.encode())
            Path(str(file) + '.sig').write_text(f'signature for {original}\n')
            self.assets[f'ESP-Gauge-{VERSION}-{renamed}'] = file
        firmware = self.source / 'firmware'
        firmware.mkdir()
        (firmware / 'firmware.json').write_text(json.dumps({'version': VERSION}))
        (firmware / 'firmware.json.sig').write_bytes(b'signed manifest')
        (firmware / 'firmware.bin').write_bytes(b'original firmware')

    def package(self):
        return subprocess.run(
            [sys.executable, str(SCRIPT), str(self.source), str(self.output), f'v{VERSION}'],
            capture_output=True, text=True,
        )

    def test_named_downloads_preserve_payloads_signatures_and_update_urls(self):
        result = self.package()
        self.assertEqual(result.returncode, 0, result.stderr)
        for name, original in self.assets.items():
            self.assertEqual((self.output / name).read_bytes(), original.read_bytes())
            self.assertEqual(
                (self.output / (name + '.sig')).read_bytes(),
                Path(str(original) + '.sig').read_bytes(),
            )
        index = json.loads((self.output / 'latest.json').read_text())
        self.assertEqual(index['version'], VERSION)
        self.assertEqual(set(index['platforms']), {'darwin-aarch64', 'linux-x86_64', 'windows-x86_64'})
        expected = {
            'darwin-aarch64': f'ESP-Gauge-{VERSION}-macOS-Apple-Silicon.app.tar.gz',
            'linux-x86_64': f'ESP-Gauge-{VERSION}-Linux-x64.AppImage',
            'windows-x86_64': f'ESP-Gauge-{VERSION}-Windows-x64-Setup.exe',
        }
        for platform, update in index['platforms'].items():
            name = expected[platform]
            self.assertEqual(update['url'], f'https://github.com/Fallstop/esp-gauge/releases/download/v{VERSION}/{name}')
            self.assertEqual(update['signature'], (self.output / (name + '.sig')).read_text().strip())
        for name in ('firmware.bin', 'firmware.json', 'firmware.json.sig'):
            self.assertEqual((self.output / name).read_bytes(), (self.source / 'firmware' / name).read_bytes())

    def test_missing_windows_update_signature_blocks_publication(self):
        original = self.assets[f'ESP-Gauge-{VERSION}-Windows-x64-Setup.exe']
        Path(str(original) + '.sig').unlink()
        result = self.package()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Missing updater platforms', result.stderr)

    def test_duplicate_package_cannot_overwrite_a_download(self):
        original = self.assets[f'ESP-Gauge-{VERSION}-Windows-x64-Setup.exe']
        original.with_name('another-setup.exe').write_bytes(b'duplicate')
        result = self.package()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Duplicate release asset', result.stderr)


if __name__ == '__main__':
    unittest.main()
