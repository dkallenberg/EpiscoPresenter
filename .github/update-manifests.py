"""Writes presenter.json and planner.json: where each app finds its newest
version (src-tauri/src/updater.rs), betas included; and for a full release
(a tag with no label, v0.3.0) also presenter-release.json and
planner-release.json, which computers set to take only full releases read.
Run by the Build installers workflow after a release, from the signed update
files in `installers/`.

    python3 .github/update-manifests.py installers <owner/repo> <tag>
"""

import datetime
import json
import pathlib
import sys

folder, repo, tag = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
version = json.loads(pathlib.Path("src-tauri/tauri.conf.json").read_text())["version"]
base = f"https://github.com/{repo}/releases/download/{tag}"
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def signed(name: str):
    """The update file `name` and its signature, if both were built."""
    f, sig = folder / name, folder / f"{name}.sig"
    if f.is_file() and sig.is_file():
        return {"url": f"{base}/{name}", "signature": sig.read_text().strip()}
    return None


wrote = []
for app, product in (("presenter", "EpiscoPresenter"), ("planner", "EpiscoPlanner")):
    platforms = {}
    mac = signed(f"{product}.app.tar.gz")
    if mac:
        # One universal app for both kinds of Mac.
        platforms["darwin-aarch64"] = mac
        platforms["darwin-x86_64"] = mac
    win = next((signed(p.name) for p in sorted(folder.glob(f"{product}_*_x64-setup.exe"))), None)
    if win:
        platforms["windows-x86_64"] = win
    if not platforms:
        continue
    manifest = {
        "version": version,
        "notes": f"{product} {tag.lstrip('v')}: see https://github.com/{repo}/releases/tag/{tag}",
        "pub_date": now,
        "platforms": platforms,
    }
    text = json.dumps(manifest, indent=2) + "\n"
    names = [f"{app}.json"]
    # A full release (no "-beta.1" or the like) is for every computer.
    if "-" not in tag.lstrip("v"):
        names.append(f"{app}-release.json")
    for name in names:
        pathlib.Path(name).write_text(text)
        wrote.append(name)
        print(f"{name}: {version}, {', '.join(platforms)}")

if not wrote:
    print("No signed update files: the apps' update information is left as it was.")
