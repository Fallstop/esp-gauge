"""Bundle the license files shipped by installed Python/Qt distributions."""
import importlib.metadata
from pathlib import Path
import shutil
root=Path('dist/licenses')
for name in ('PySide6-Essentials','shiboken6','psutil','pyserial'):
    distribution=importlib.metadata.distribution(name)
    for entry in distribution.files or []:
        if 'license' in str(entry).lower() or 'copying' in str(entry).lower():
            source=Path(distribution.locate_file(entry))
            if source.is_file():
                target=root/name/str(entry)
                target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copyfile(source,target)
