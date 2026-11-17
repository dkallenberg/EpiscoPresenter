# Troubleshooting

### "EpiscoPresenter is damaged and can't be opened" (Mac)

It isn't damaged; macOS quarantines apps that aren't signed by an Apple
developer. See [Installing](Installing#the-app-is-damaged-and-cant-be-opened).

### The screens don't appear

The output screens don't open by themselves. Click the **monitor icon** at
the right end of the top bar (it's gold while the screens are closed) and
choose **Full screen** on the right monitor.

### Swiping between desktops on the Mac

A full-screen output stays on every desktop of its monitor, so switching
desktops on the control screen leaves it showing. If it doesn't, close the
screen and open it again in **Full screen**.

### Blackout won't turn off (or on)

It's locked. Click **🔒** beside Blackout to unlock it.

### The live stream still shows text during Blackout

That's intended: Blackout is for the sanctuary screen only. To hide the
words on both, use **Clear Text** (Esc), or set the item's stream look to
**Camera only**.

### I can't edit the Creed (or another Prayer Book text)

Prayer Book texts are protected. Click **Make a parish version to edit** in
the Editor. See [Editing items](Editing-Items#prayer-book-texts-are-protected).

### A hymn saved to the library is called "Entrance Hymn"

Hymns saved before November 2026 could be named for their place. Delete it
in the Library, give the hymn its **Hymn #** and **Hymn title** in the
Editor, and click **Save to Library** again.

### "Save to Library" says the hymn needs a number or title

The item only has its place's name (*Entrance Hymn*). Fill in **Hymn #** or
**Hymn title** in the Editor first.

### The service didn't arrive on the presentation computer

* In the planner, check the service is marked **✓ Ready**.
* On the presentation computer, check **⚙ Settings → Parish library folder…**
  is the same shared folder. Hover over it: it should say it is in step.
  With a folder synced by Google Drive for desktop, check Drive is running
  and the folder is **Available offline**.
* Give it a minute: services are checked for about once a minute.

### ⚠ or a gold ⚙ gear, or "the folder can't be found"

The parish library folder can't be reached. Hover over **⚙ Settings →
Parish library folder…** to see why. With the app connected to Google Drive
itself, it's usually no internet, or the sign-in has lapsed: open
**Parish library folder…** and **Sign in with Google…** again. With Google
Drive for desktop, it isn't running or isn't signed in. Either way the app
keeps working from this computer's copy and catches up by itself.

### The app takes a long time to start with a Google Drive folder

Connecting the app to Google Drive itself avoids this: it keeps a whole
copy on the computer (see [First-time setup](First-Time-Setup)). With
Google Drive for desktop, Drive normally downloads a file only when it's opened,
so a folder of many small files is slow the first time. On each computer,
right-click the parish library folder in Finder (or Explorer) and choose
**Google Drive → Available offline**, or set Google Drive to **Mirror
files** in its preferences. The apps also remember the list of services and
library items on each computer and re-read only what has changed, so the
first start after an update is the slowest.

### No "Restart to update" note appears

The apps look for a newer version about ten seconds after they start, so
leave the app open a moment. The note appears only once the new version is
fully downloaded. If there's still nothing, install the newest version by
hand from the
[Releases page](https://github.com/dkallenberg/EpiscoPresenter/releases)
(see [Installing](Installing#updating)).

### The two computers won't connect

See [Two computers](Two-Computers#if-it-wont-connect).

### A web video doesn't play

Check the internet connection and that the video can be embedded (private
and age-restricted videos can't). Instagram videos may need a click on the
sanctuary screen. If you're allowed to download the video, import the file
into **Library → Media** instead.

### The announcements don't show

Check the deck is shared as **Anyone with the link can view**, then click
**↻ Refresh slides** in **⚙ Settings → Service settings**. If the deck can't
be downloaded, the screens fall back to Google's own player.

### A reading shows "[The text of … goes here]"

The reading has no text yet. See
[Bulletin and script](Bulletin-and-Script#reading-texts).

### When something goes wrong

Error messages say what to try first, with the technical details at the end
for whoever helps you. Please include those details if you report a
problem.
