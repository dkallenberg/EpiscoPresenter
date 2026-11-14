# Installing

There are two apps. Install **EpiscoPlanner** on the computers used for
planning, and **EpiscoPresenter** on the computer (or two computers) that
run the screens on Sunday. They can be installed side by side on the same
computer.

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

Run the installer. Windows needs the **WebView2** runtime, which is already
on up-to-date Windows 10 and 11.

## Updating

Install the new version over the old one. Your services, library and
settings are kept: they are in the parish library folder and the app's own
settings folder, not inside the app.
