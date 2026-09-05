"""Launch the frozen application on every platform; never open USB hardware."""
from pathlib import Path
import os
import subprocess
import sys
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path('dist')
executable=root/'ESP-Gauge'/('ESP-Gauge.exe' if sys.platform=='win32' else 'ESP-Gauge')
subprocess.run([str(executable.resolve()),'--smoke-test'],check=True,timeout=30,
               env={**os.environ,'QT_QPA_PLATFORM':'offscreen'})
