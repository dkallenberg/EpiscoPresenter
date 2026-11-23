# Installing

There are two apps. Install **EpiscoPlanner** on the computers used for
planning, and **EpiscoPresenter** on the computer (or two computers) that
run the screens on Sunday. They can be installed side by side on the same
computer.

## Where to get them

Download the installers from the project's
[Releases page](https://github.com/dkallenberg/EpiscoPresenter/releases).
Open the newest release and choose, under **Assets**:

| | EpiscoPlanner | EpiscoPresenter |
| --- | --- | --- |
| **Mac** (Apple silicon and Intel) | `EpiscoPlanner_…_universal.dmg` | `EpiscoPresenter_…_universal.dmg` |
| **Windows** | `EpiscoPlanner_…_x64-setup.exe` | `EpiscoPresenter_…_x64-setup.exe` |

A release marked **Pre-release** is a beta. Windows also has `.msi`
installers, for IT departments that deploy software.

## On a Mac

1. Open the `.dmg` file and drag the app into **Applications**.
2. Open it from Applications.

### "The app is damaged and can't be opened"

The apps are not yet signed with an Apple Developer ID, so a Mac that
didn't build the app quarantines it when it arrives by download, AirDrop,
email or a cloud drive. It isn't damaged. Open **Terminal** and run, once:

```bash
xattr -dr com.apple.quarantine /Applications/EpiscoPresenter.app
```

and for the planner:

```bash
xattr -dr com.apple.quarantine /Applications/EpiscoPlanner.app
```

If instead macOS says the app is from an **unidentified developer**,
Control-click the app and choose **Open**, or go to **System Settings →
Privacy & Security** and click **Open Anyway**.

### A Mac managed by your diocese or IT department

A managed Mac may not let you run the Terminal command or choose **Open
Anyway**. Ask whoever manages the Mac to allow the app (or to run the
command above with an administrator account). EpiscoPresenter also asks for
**local network** access the first time it connects two computers; IT may
need to allow that too.

## On Windows

Run the `-setup.exe`. It installs for you alone (no administrator needed)
and adds the app to the Start menu. Windows needs the **WebView2** runtime,
which is already on up-to-date Windows 10 and 11.

The installers are not signed by Microsoft, so Windows SmartScreen says
**"Windows protected your PC"**: click **More info**, then **Run anyway**.

## Updating

The apps update themselves. A little after it starts, each app looks for a
newer version; if there is one, it downloads it in the background and a
note at the top of the window says so, with **Restart to update**. Nothing
is installed until someone clicks it, so an update never interrupts a
service. In EpiscoPresenter the note waits while a service is on the
screens, and comes back when they're idle (nothing taken yet, or the
sanctuary blacked out). If a screen is open, **Restart to update** asks
first: the screens close for about a minute while it restarts.

**Betas or full releases.** **⚙ Settings → Updates…** shows the version
installed and sets what this computer takes:

* **Full releases only:** tested versions. EpiscoPresenter starts this way,
  because it runs Sunday's services.
* **Include betas:** new features to try before they're finished.
  EpiscoPlanner starts this way.

Choosing full releases doesn't go back from a beta already installed; the
next full release replaces it. **Check now** looks for a newer version
straight away.

You can also install a new version over the old one by hand, from the
Releases page. Either way your services, library and settings are kept:
they are in the parish library folder and the app's own settings folder,
not inside the app.

(The apps look for updates only when the version was built with the
project's update key; see the README's *Updates* section.)
