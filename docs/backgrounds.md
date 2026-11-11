# Backgrounds and layouts

Every cue has a **look** on each output (the sanctuary screen and the live
stream). A look has four settings:

| Setting | Values |
| --- | --- |
| Background | an image file, a built-in design, a solid colour (`#000000`), or `key` (the stream key colour) |
| Layout | where the text goes; see the table below |
| Text | `dark` (dark text for light backgrounds) or `light` (cream and white text for dark ones) |
| Fit | `fill` (crop to cover), `fit` (letterbox), `stretch` |

## Where looks come from

Each layer overrides only the settings it names, in this order:

1. **Built-in default for the item type.** For example, a collect gets light glass on the sanctuary and the dark left panel on the stream.
2. **Service default for the type** (`[looks.collect.stream]` in the plan). The editor's **Use for all … items** button sets this.
3. **The `.ewm` file's `default_background:`** (sanctuary only).
4. **The cue's own look** (`[cue.sanctuary]` / `[cue.stream]` in the plan). The editor's **Backgrounds & layout** panel sets this.

If a layer changes the **layout** without naming a background or text tone,
the background and tone switch to the new layout's defaults. A lower third
never inherits the glass-bar artwork.

```toml
stream_key = "#0000ff"            # optional; blue by default

[looks.collect.stream]
background = "backgrounds/Stream Prayer.png"

[[cue]]
file = "liturgies/dismissal.ewm"
[cue.stream]
layout = "lower_third"
```

Image paths are relative to the service plan's folder. Put the parish's
images in `backgrounds/` beside the plan, or use **Import to library…** in the
background picker. Images in the app's own library are stored with absolute
paths.

## Built-in defaults

| Item type | Sanctuary | Stream |
| --- | --- | --- |
| Prayer, collect, acclamation, other | Glass · light, centered, dark text | Stream panel · colour, left panel |
| Eucharistic Prayer | Glass · light, centered | Stream bottom bar |
| Hymn | Black, lyrics | Stream panel · light, left panel (wide), dark text |
| Psalm | Glass · charcoal, centered, light text | Stream bottom bar |
| Gospel | Glass · colour, centered, light text | Stream bottom bar |
| Reading, sermon, anthem, announcements | Glass · colour, centered, light text | Key colour, lower third |

For **green**, the stream designs (panel, light panel, bottom bar, lower third)
are the parish's own artwork, bundled with the app in `ui/backgrounds/green/`.
For other colours, and for the sanctuary designs, the app draws them in the
liturgical colour with the same geometry. To add artwork for another colour,
put `panel-dark`, `panel-light`, `bar-bottom` and `lower-third` images in
`ui/backgrounds/<colour>/` and list them in `ARTWORK` in
`ui/js/backgrounds.js`.

## Text areas (for making your own backgrounds)

Positions are percentages of a 16:9 canvas, so they hold for 1920×1080 or 1280×720.

| Layout | Text area | Notes |
| --- | --- | --- |
| `centered` | x 3.6–96.4 %, y 12.5–87.5 % | between top and bottom borders of about 11 % |
| `lyrics` | x 5.7–94.3 %, y 6.5–91.7 % | centered, bold; caption at the bottom-left |
| `left_panel` | x 3.9–45.5 %, y 2–98 % | inside the panel at x 3.2–46.8 % (border columns at 1.7–3.0 % and 47.0–48.3 %) |
| `left_panel_wide` | x 3.9–53.9 %, y 2–92.6 % | inside the panel at x 3.0–54.5 %; hymn credits at the bottom-left |
| `bottom_bar` | x 2.9–97.1 %, y 62.2–94.1 % | the bar is y 60.7–94.8 % between brick rows; label tag at x 4.9–24.8 %, y 28.1–35.7 % |
| `lower_third` | x 4.1–68 %, y 78.3–93.9 % | inside a framed bar at x 1.5–70.4 %, y 74.8–97.5 % (panel x 3.3–68.5 %, y 78–94.1 %); the rest is key colour |

For `lower_third` with the **built-in** background, the app paints the key
colour with the frame. With the default blue key it uses the parish artwork,
whose blue is a slight gradient (about `#1D1CE5` to `#3736FC`); with any other
key colour it draws the frame on a solid fill. With your **own** image, include both the key colour and
the frame in the image.

## Keying the stream

Lower thirds are keyed by the video switcher on the stream key colour
(`#0000FF` by default). Change it in **⚙ Service** (Blue, Green, Transparent
or a custom colour). **Blackout** on the stream output fills it with the key
colour, so only the camera shows.
