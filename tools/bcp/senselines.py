# Writes the Prayer Book's sense-line layouts (tabs = indent) of the Lord's
# Prayer, the Nicene Creed and the Confession into the built-in library.
import sys
lib = sys.argv[1]

def para(role, text):
    out = []
    for l in text.strip('\n').split('\n'):
        n = len(l) - len(l.lstrip('\t'))
        body = l.lstrip('\t')
        if role in ('All', 'People'):
            body = f"**{body}**"
        out.append('\t' * n + body)
    if out[0].startswith('\t'):
        return f"[{role}]\n" + "\n".join(out)
    return f"[{role}] " + "\n".join(out)

def write(path, head, parts):
    open(lib + path, 'w').write(head.strip() + "\n\n" + "\n\n---\n\n".join(parts) + "\n")

def front(title, cite, rite):
    return f"---\ntitle: {title}\ntype: prayer\ncitation: {cite}\nrite: {rite}\n---"

lp2 = [para('All', '''Our Father in heaven,
	hallowed be your Name,
	your kingdom come,
	your will be done,
		on earth as in heaven.'''),
para('All', '''Give us today our daily bread.
Forgive us our sins
	as we forgive those who sin against us.
Save us from the time of trial,
	and deliver us from evil.'''),
para('All', '''For the kingdom, the power,
	and the glory are yours,
	now and for ever. Amen.''')]
lp1 = [para('All', '''Our Father, who art in heaven,
	hallowed be thy Name,
	thy kingdom come,
	thy will be done,
		on earth as it is in heaven.'''),
para('All', '''Give us this day our daily bread.
And forgive us our trespasses,
	as we forgive those
		who trespass against us.
And lead us not into temptation,
	but deliver us from evil.'''),
para('All', '''For thine is the kingdom,
	and the power, and the glory,
	for ever and ever. Amen.''')]
bold2 = "[Celebrant] And now, as our Savior Christ has taught us, we are bold to say,\n"
bold1 = "[Celebrant] And now, as our Savior Christ hath taught us, we are bold to say,\n"
write('lords-prayer.ewm', front("The Lord's Prayer", "BCP 364", "II"), [bold2 + lp2[0]] + lp2[1:])
write('lords-prayer-traditional.ewm', front("The Lord's Prayer (traditional)", "BCP 364", "II"), [bold2 + lp1[0]] + lp1[1:])
write('rite1/lords-prayer.ewm', front("The Lord's Prayer", "BCP 336", "I"), [bold1 + lp1[0]] + lp1[1:])

together = "[Rubric] *The Priest and People pray together*\n"
write('nicene-creed.ewm', front("The Nicene Creed", "BCP 358", "II"), [
together + para('All', '''We believe in one God,
	the Father, the Almighty,
	maker of heaven and earth,
	of all that is, seen and unseen.'''),
para('All', '''We believe in one Lord, Jesus Christ,
	the only Son of God,
	eternally begotten of the Father,
	God from God, Light from Light,
	true God from true God,
	begotten, not made,
	of one Being with the Father.'''),
para('All', '''	Through him all things were made.
	For us and for our salvation
		he came down from heaven:
	by the power of the Holy Spirit
		he became incarnate from the Virgin Mary,
		and was made man.'''),
para('All', '''	For our sake he was crucified under Pontius Pilate;
		he suffered death and was buried.
		On the third day he rose again
			in accordance with the Scriptures;'''),
para('All', '''		he ascended into heaven
			and is seated at the right hand of the Father.
		He will come again in glory to judge the living and the dead,
			and his kingdom will have no end.'''),
para('All', '''We believe in the Holy Spirit, the Lord, the giver of life,
	who proceeds from the Father and the Son.
	With the Father and the Son he is worshiped and glorified.
	He has spoken through the Prophets.'''),
para('All', '''	We believe in one holy catholic and apostolic Church.
	We acknowledge one baptism for the forgiveness of sins.
	We look for the resurrection of the dead,
		and the life of the world to come. Amen.''')])
write('rite1/nicene-creed.ewm', front("The Nicene Creed", "BCP 327", "I"), [
together + para('All', '''I believe in one God,
	the Father Almighty,
	maker of heaven and earth,
	and of all things visible and invisible;'''),
para('All', '''And in one Lord Jesus Christ,
	the only-begotten Son of God,
	begotten of his Father before all worlds,
	God of God, Light of Light,
	very God of very God,
	begotten, not made,
	being of one substance with the Father;
	by whom all things were made;'''),
para('All', '''	who for us men and for our salvation
		came down from heaven,
	and was incarnate by the Holy Ghost of the Virgin Mary,
		and was made man;
	and was crucified also for us under Pontius Pilate;
		he suffered and was buried;'''),
para('All', '''	and the third day he rose again
		according to the Scriptures,
	and ascended into heaven,
		and sitteth on the right hand of the Father;
	and he shall come again, with glory,
		to judge both the quick and the dead;
	whose kingdom shall have no end.'''),
para('All', '''And I believe in the Holy Ghost the Lord, and Giver of Life,
	who proceedeth from the Father and the Son;
	who with the Father and the Son together is worshiped and glorified;
	who spake by the Prophets.'''),
para('All', '''And I believe one holy Catholic and Apostolic Church;
	I acknowledge one Baptism for the remission of sins;
	and I look for the resurrection of the dead,
	and the life of the world to come. Amen.''')])

silence = "[Rubric] *Silence may be kept.*\n[Rubric] *Minister and People*\n"
write('confession.ewm', front("Confession of Sin", "BCP 360", "II"), [
"[Deacon] Let us confess our sins against God and our neighbor.\n" + silence + para('All', '''Most merciful God,
we confess that we have sinned against you
in thought, word, and deed,
by what we have done,
and by what we have left undone.
We have not loved you with our whole heart;
we have not loved our neighbors as ourselves.'''),
para('All', '''We are truly sorry and we humbly repent.
For the sake of your Son Jesus Christ,
have mercy on us and forgive us;
that we may delight in your will,
and walk in your ways,
to the glory of your Name. Amen.''')])
write('rite1/confession.ewm', front("Confession of Sin", "BCP 331", "I"), [
"[Deacon] Let us humbly confess our sins unto Almighty God.\n" + silence + para('All', '''Almighty God, Father of our Lord Jesus Christ,
maker of all things, judge of all men:
We acknowledge and bewail our manifold sins and wickedness,
which we from time to time most grievously have committed,
by thought, word, and deed,
against thy divine Majesty,
provoking most justly thy wrath and indignation against us.'''),
para('All', '''We do earnestly repent,
and are heartily sorry for these our misdoings;
the remembrance of them is grievous unto us,
the burden of them is intolerable.
Have mercy upon us,
have mercy upon us, most merciful Father;'''),
para('All', '''for thy Son our Lord Jesus Christ's sake,
forgive us all that is past;
and grant that we may ever hereafter
serve and please thee
in newness of life,
to the honor and glory of thy Name;
through Jesus Christ our Lord. Amen.''')])
