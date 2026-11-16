# The parish library in Google Drive (without Google Drive for desktop)

EpiscoPlanner and EpiscoPresenter can reach the shared parish folder in
Google Drive themselves. Each person signs in with their own Google account
(a personal Gmail account works), picks the shared folder, and the app keeps
a full copy of it on the computer, in step with Drive every few seconds.
Google Drive for desktop is not needed.

Compared with a folder synced by Google Drive for desktop:

* Files are always on the computer, never streamed on first use, so the
  apps start quickly and the presentation computer never waits on the
  internet during a service.
* A file arrives whole (it is written under a temporary name and then put
  in place), so nothing reads a half-downloaded file.
* No "(1)" conflict copies: when the same file changed on two computers,
  the later change wins.

## Once for the parish: a Google Cloud project (about 15 minutes)

The parish owns the connection to Google: a free Google Cloud project, set
up once by whoever owns the parish folder in Google Drive (or a parish Gmail
account). Nothing in it costs money.

1. Go to **console.cloud.google.com** and sign in.
2. At the top, open the project list and choose **New project**. Name it,
   for example, *EpiscoPresenter St James*, and click **Create**. Make sure
   the new project is selected at the top.
3. **Turn on Google Drive.** Menu ☰ → **APIs & Services → Library**. Search
   for **Google Drive API**, open it and click **Enable**.
4. **Describe the app.** Menu ☰ → **Google Auth Platform** (on older screens
   *APIs & Services → OAuth consent screen*) → **Get started**:
   * App name: *EpiscoPresenter*; user support email: yours.
   * Audience: **External**.
   * Contact email: yours. Agree to the policy and click **Create**.
5. **What the app may do.** In Google Auth Platform → **Data Access** →
   **Add or remove scopes**. Find `.../auth/drive` ("See, edit, create, and
   delete all of your Google Drive files"), tick it, **Update**, then **Save**.
   The app uses it only for the parish folder you pick, but Google has no
   narrower permission that can see a folder someone else shared with you.
6. **Publish it (without verification).** Google Auth Platform → **Audience**
   → **Publish app** → **Confirm**. The status becomes *In production*.
   Do **not** submit it for verification. In *Testing* Google signs everyone
   out every 7 days; in production, unverified, sign-ins last and up to 100
   people can sign in, plenty for a parish.
7. **The client file.** Google Auth Platform → **Clients** → **Create client**:
   * Application type: **Desktop app**; name: *EpiscoPresenter*.
   * Click **Create**, then **Download JSON**.

   This file (named like `client_secret_….json`) is what every computer
   needs. It is not a password: for a desktop app Google expects it to be
   shared with the app, and each person still signs in with their own
   account. Keep it to the parish all the same (email it to the team, or put
   it on a USB stick).

## Share the parish folder

In Google Drive (drive.google.com), right-click the parish library folder →
**Share**, and add each person's Google address as an **Editor**. The
presentation computer signs in with an account that has the folder too.

## On each computer

1. Open **⚙ Settings** (the gear at the right end of the top bar) →
   **Parish library folder…**, and choose the **Google Drive** tab. (On
   EpiscoPlanner's first run this dialog opens by itself.)
2. **Add the client file…** and choose the JSON file from step 7.
3. **Sign in with Google…** The browser opens Google's sign-in. Because the
   app is not verified by Google, it says **"Google hasn't verified this
   app"**: click **Advanced**, then **Go to EpiscoPresenter (unsafe)**, then
   **Continue** / **Allow**. (It is the parish's own project; Google shows
   this for any app it has not reviewed.) The browser then says you can
   close the tab.
4. **Choose the folder…**: your folders and those shared with you are
   listed, newest first; search by name if it is not there. Click the
   parish folder.
5. Enter your name (EpiscoPlanner) and click **Save**.

The first copy of the folder can take a few minutes (pictures and videos
included). **⚙ Settings → Parish library** says *Copying…* (hover over it)
until it is done, then the Services screen and the Library fill in. From then on:

* changes made on this computer go to Drive within a few seconds;
* changes made elsewhere arrive within about 10 seconds (Drive's change list
  is checked every 10 seconds, and the whole folder compared every 6 hours
  to be safe);
* the Parish library dialog shows **✓ In step with Google Drive** and when
  it last checked, or **⚠** and what went wrong (for example a sign-in that
  needs renewing: sign in again there).

The copy is kept in the app's local data folder (on macOS
`~/Library/Application Support/org.episcopresenter.planner/google-drive/`,
on Windows `%LOCALAPPDATA%\org.episcopresenter.planner\google-drive\`; for
EpiscoPresenter, `org.episcopresenter.app`). Do not edit files there by hand
while the app is running.

### Moving from Google Drive for desktop

Choose the same folder on the **Google Drive** tab. The app makes its own
copy (the folder synced by Drive for desktop is left alone), and once it
says it is in step you can stop syncing that folder in Drive for desktop, or
quit Drive for desktop on that computer.

### Signing out

**Sign out** in the Parish library dialog removes the sign-in from this
computer; the copy of the folder stays. To revoke the app's access
completely, go to **myaccount.google.com → Security → Your connections to
third-party apps & services**, choose *EpiscoPresenter* and remove access.

## Good to know

* The sign-in is kept in the app's settings folder on this computer,
  readable only by your user account; the app never sees your password.
* Google Docs, Sheets and Slides files in the parish folder are not copied
  (they have no file to copy); the announcements deck is fetched separately
  by its link, as before.
* A file you delete in the app is moved to the Drive trash (recoverable for
  30 days). In a folder shared from someone's My Drive, Google may only let
  the owner remove a file they own; then it stays in Drive and comes back.
* Two services can't be edited at once: the "Open on Rector's computer" marks
  work the same way through Drive.
