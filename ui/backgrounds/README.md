Bundled background artwork: the parish's designs, a set per season.

* `green/`: green.
* `white/`: white (Christmas, Easter, feasts).
* `advent/`: blue, and violet in Advent.
* `lent/`: violet (Lent).
* `blood-red/`: red in Holy Week (Palm Sunday, Maundy Thursday).
* `black/`: black (Good Friday, Holy Saturday).
* `bright-red/`: red on other days (Pentecost, martyrs).

Each set has the sanctuary's `glass-light` (prayers), `glass-dark`
(readings, title cards) and `glass-charcoal` (psalms), and the stream's
`panel-dark` (prayers), `panel-light` (hymns), `bar-bottom` (psalms,
Eucharistic Prayer, Gospel), `lower-third` (readings, sermon, announcements,
on a blue key), `lower-third-frame` (the same box on a transparent layer,
drawn over any key colour) and `window` (welcome / postlude), all 1920×1080;
some also have `lyrics`, the sanctuary's background for hymn words.
A set may lack one (Advent and Lent have no charcoal glass); `ARTWORK` in
`ui/js/backgrounds.js` says which file stands in. They replace the drawn
built-in designs of the same names. See docs/backgrounds.md.

Service-specific images belong in a `backgrounds/` folder beside the service
plan, not here.
