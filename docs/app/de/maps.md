# Karten

Karten zeigt die Welt des Spiels und jede installierte Karten-Mod auf einem Bild, damit du siehst, welche Teile von Knox Country eine Karte ersetzt und wo zwei Karten-Mods um denselben Boden streiten. Außerdem listet es die Karten-Mods, die kein eigenes Land hinzufügen, und warnt vor Tile-Konflikten.

## Was du siehst

- Die echte Karte der Spielwelt, mit den Zellen jeder Karten-Mod in ihrer eigenen Farbe darüber gezeichnet.
- Rechts die Liste der Karten-Mods, die Zellen hinzufügen.
- Über der Karte eine Warnung für jedes Paar von Karten-Mods, das sich Zellen oder Tile-Nummern teilt.
- Darunter **Andere Karten-Mods**: Spawnpunkte, Kartenlisten und Tile-Pakete.

Die Welt ist in Zellen aufgeteilt; eine Zelle ist 256 x 256 Tiles groß.

## Auf der Karte bewegen

- Klick zuerst auf die Karte, dann dreh das **Mausrad**, um hinein- und herauszuzoomen. Gezoomt wird auf den Punkt unter dem Mauszeiger. Bis du klickst, scrollt das Rad die Seite (**Klick auf die Karte, um mit dem Mausrad zu zoomen** erinnert dich daran), und das tut es wieder, sobald der Mauszeiger die Karte verlässt.
- **Linke Taste halten und ziehen**, um die Karte zu verschieben.
- **Doppelklick** auf die Karte, oder der Knopf in ihrer oberen rechten Ecke (**Ganze Karte anzeigen**), zeigt wieder alles.
- Halte den Mauszeiger über eine Zelle, um ihre Nummer zu sehen, ob sie zur Spielwelt gehört und welche Karten sie abdecken (**Mehrere Karten hier**, wenn zwei sie sich teilen).

Das Bild wird aus den Kartenbildern zusammengesetzt, die das Spiel und die Mods mitbringen. Währenddessen siehst du **Setze die Karte zusammen…**. Eine Karten-Mod ohne Bild zeigt ihre Zellen trotzdem als farbige Quadrate.

## Was die Farben bedeuten

- **Die Welt des Spiels**: die Grundkarte, mit einem blassen Raster darüber.
- **Die Farbe jeder Karten-Mod**: ihre Zellen. Das kleine Quadrat neben ihrem Namen in der Liste zeigt die Farbe. Die Zellen einer Karten-Mod über der Spielwelt ersetzen, was dort war.
- **Rot**: Zellen, die sich zwei Karten-Mods teilen. Dort ersetzt in deinem Spiel die eine die andere, es sei denn, eine baut auf der anderen auf.

## Die Liste der Karten-Mods

Jede Zeile zeigt den Namen der Karte, die Mod, aus der sie kommt, wie viele Zellen sie hat, wie viele davon über der Spielwelt liegen (oder **außerhalb der Welt des Spiels**), auf welchen Karten sie **aufbaut**, und **aus**, wenn die Mod ausgeschaltet ist.

- Nimm den Haken aus dem Kästchen einer Zeile, um diese Karte im Bild auszublenden. Das ändert nur die Ansicht, nicht deine Mod-Liste.
- Drück das Listensymbol in einer Zeile (**In der Mod-Liste anzeigen**), um zu dieser Mod in der Mod-Liste zu springen, hervorgehoben.
- Hak **Nur aktive Karten-Mods meiner Liste** an, um nur die Karten zu zeigen, die du angeschaltet hast. Das blendet auch die Warnungen und die anderen Karten-Mods aus, die nur ausgeschaltete Mods betreffen. Der Haken wird gemerkt.
- Der Aktualisieren-Knopf oben liest die Karten neu ein. Sie werden auch neu eingelesen, wenn sich deine Mod-Liste ändert.

## Konflikte

- Eine **rote** Warnung bedeutet, dass zwei Karten-Mods sich Zellen teilen und beide an sind: In deinem nächsten Spiel ersetzt dort die eine die andere. Darunter hat jede Mod einen Knopf, um sie in der Mod-Liste zu sehen, und einen Knopf **Deaktivieren**, der sie ausschaltet (nach einer Rückfrage).
- Was nur kollidieren würde, wenn du heute ausgeschaltete Mods anschaltest, wird in einer grauen Zeile zusammengefasst (**N mögliche Konflikte, wenn du heute ausgeschaltete Mods anschaltest: jetzt nichts zu tun**). Öffne sie, um jeden einzeln zu sehen.
- Eine Warnung zur Tile-Definition bedeutet, dass zwei Mods dieselbe Tile-Definitionsnummer nutzen. Sind beide an, überschreiben sich ihre Tiles gegenseitig, und du bekommst falsche Wände, Böden oder Möbel. Lass nur eine an. Darunter hat jede Mod einen Knopf, um sie in der Mod-Liste zu sehen, und **Deaktivieren**, wenn mehr als eine an ist.

## Andere Karten-Mods

Diese Mods haben mit Karten zu tun, fügen aber keine Zellen hinzu: Spawnpunkte, eine Kartenliste für eine andere Karte oder die Tiles, aus denen Karten gebaut werden. Jede ist mit **Spawnpunkte / Kartenliste** oder **Kacheln** markiert. Sie brauchen die Karte oder Mod, für die sie sind, angeschaltet. Klick auf eine, um sie in der Mod-Liste zu sehen.

## Gut zu wissen

- Zwei Karten, die sich Zellen teilen, kollidieren nicht, wenn die eine die andere in ihrer `map.info` nennt (`lots=`); die Liste zeigt das als **baut auf**.
- Die Kartenbilder werden einmal erstellt und in `Zomboid/PZModStudio_Cache/maps/` aufbewahrt, sodass der Bildschirm beim nächsten Mal schneller öffnet.
- Nichts auf diesem Bildschirm ändert dein Spiel außer **Deaktivieren**.
