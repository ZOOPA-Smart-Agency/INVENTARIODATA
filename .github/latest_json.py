"""Writes latest.json, which the app reads on opening to find updates (Tauri updater format).

Run in the release job with the release's files and their .sig files in one folder:
    python3 latest_json.py <folder> <version> <download base URL>
Each platform entry points at the file that copy installs: the Mac app (.app.tar.gz), the Windows installers and
portable .exe, the Linux packages. The app looks for "<os>-<arch>-<how it was installed>", then "<os>-<arch>".
Fails when a file or its signature is missing, so a release never offers an update that cannot install.
"""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote

folder, version, base = Path(sys.argv[1]), sys.argv[2], sys.argv[3].rstrip("/")
name = f"Registro-de-equipos_{version}"
files = {
    "darwin-aarch64": f"{name}_macos-universal.app.tar.gz",
    "darwin-x86_64": f"{name}_macos-universal.app.tar.gz",
}
for arch, label in (("x86_64", "x64"), ("aarch64", "arm64")):
    files[f"windows-{arch}-nsis"] = f"{name}_windows-{label}-setup.exe"
    files[f"windows-{arch}"] = f"{name}_windows-{label}-setup.exe"
    files[f"windows-{arch}-portable"] = f"{name}_windows-{label}-portable.exe"
    files[f"linux-{arch}-appimage"] = f"{name}_linux-{label}.AppImage"
    files[f"linux-{arch}"] = f"{name}_linux-{label}.AppImage"
    files[f"linux-{arch}-deb"] = f"{name}_linux-{label}.deb"
    files[f"linux-{arch}-rpm"] = f"{name}_linux-{label}.rpm"
files["windows-x86_64-msi"] = f"{name}_windows-x64.msi"  # no ARM MSI is built

platforms, missing = {}, []
for target, file in files.items():
    path, sig = folder / file, folder / f"{file}.sig"
    if not path.is_file() or not sig.is_file():
        missing.append(file)
        continue
    platforms[target] = {"signature": sig.read_text().strip(), "url": f"{base}/{quote(file)}"}
if missing:
    sys.exit("Missing update files or signatures: " + ", ".join(sorted(set(missing))))

manifest = {"version": version, "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"), "platforms": platforms}
(folder / "latest.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"latest.json: {version}, {len(platforms)} platform entries")
