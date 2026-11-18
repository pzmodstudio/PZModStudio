# Mod-Liste

Deine installierten Mods: welche an sind und in welcher Reihenfolge das Spiel sie lädt. Jede Änderung hier wird sofort im aktiven Profil und in `Zomboid/mods/default.txt` gespeichert, der Liste, die das Spiel liest.

## Was du siehst

- Die Kopfzeile zählt Workshop-Artikel, Mods, **aktive** Mods sowie Karten-, Bibliotheks- und Skript-Mods. Rote und gelbe Zähler (**Abhängigkeiten fehlen**, **Abhängigkeiten aus**, **Unverträglichkeit**, **Reihenfolge**) erscheinen, wenn es ein Problem gibt; klick auf einen, um durch diese Mods zu springen.
- Die Werkzeugleiste: Suche, Filter, Gruppen und die Knöpfe **Alle aktivieren** / **Alle deaktivieren**.
- Die Liste. Der grüne Balken oben (**LÄDT ZUERST**) ist der Platz für Bibliotheken; der gelbe Balken unten (**LÄDT ZULETZT**) ist der Platz, an dem eine Mod jede Datei gewinnt, die sich zwei Mods teilen.
- Rechts das Feld der gewählten Mod (leer, bis du auf eine klickst). Mehr dazu unten, unter **Das Feld rechts**.

## Das Feld rechts

Oben, immer im Blick: das Bild der Mod (klick darauf, um es in voller Größe zu sehen), ob sie eine Karten-, Bibliotheks- oder Skript-Mod ist, woher sie kommt (Steam, kompatible Kopie, lokal...), ihr Name, ihre ID, ihr Autor und ihre Version, und die Knöpfe **Den Autor unterstützen** mit den Links, die ihr Autor ins Changelog oder in den Workshop geschrieben hat (Ko-fi, Patreon...), geöffnet in deinem Browser. Darunter ein Knopf, der sie an- oder ausschaltet und sagt, wo sie lädt, und **In Mods finden ansehen**.

Dann drei Reiter (der gewählte bleibt, wenn du die Mod wechselst):

- **Studio**: was die App über diese Mod in deiner Liste weiß.
  - **Notizen**: deine ganze Notiz (**Notiz bearbeiten** oder **Notiz hinzufügen**), deine Gruppen und die Profile, in denen sie ist, das aktive hervorgehoben.
  - **Was zu tun ist**: jede Warnung der Prüfung mit allem, was sie sagt, ob du handeln musst, ob es sich lohnt oder ob es nur gut zu wissen ist, und ihr Lösungsknopf. Gibt es nichts zu tun, sagt es das in Grün.
  - **Letzte Sitzung**: die Fehler oder Warnungen, die das Spiel aufgezeichnet hat (mit der letzten Zeile), wie lange sie zum Laden gebraucht hat und ihr Anteil an der Zeit des Spiels.
  - **Abhängigkeiten**: was sie braucht und welche Mods sie brauchen; klick auf eine, um zu ihrer Zeile zu gehen.
  - **Mod-Metadaten** (was ihre `mod.info` sagt, mit **Neuestes aus dem Changelog**) und **Klassen, die ihr Java ändert**, eingeklappt am Ende.
  - **Was zu tun ist** und **Letzte Sitzung** klappen mit einem Klick auf ihren Titel ein und bleiben so, wie du sie gelassen hast.
- **Workshop-Seite**: die Seite der Mod, ohne die App zu verlassen: Bilder und Videos, Abonnenten, Favoriten, Größe, Daten, Autor, Kategorien, die Artikel, die sie benötigt (grün, wenn du ihn hast: klick, um zu seiner Zeile zu gehen; gelb, wenn nicht: er öffnet sich unter Mods finden), und die ganze Beschreibung mit **Übersetzen**. Eine Mod, die nicht im Workshop ist, zeigt hier ihre eigene Beschreibung.
- **Kommentare**: die Kommentare auf ihrer Workshop-Seite, die neuesten zuerst, mit **Mehr laden**. Ist das Profil des Autors privat, zeigt Steam sie nicht, und das Feld sagt das.

## Die Spalte #

Die Nummer in jeder Zeile ist der Platz der Mod unter den Mods, die **an** sind: die Ladereihenfolge, die das Spiel liest, und dieselbe Nummer, die das Spiel zeigt. Eine Mod, die aus ist, zeigt **—** und ist ausgegraut, weil sie keinen Platz in der Ladereihenfolge hat. Ausgeschaltete Mods behalten ihren Platz in der Liste, also kommt eine meist dorthin zurück, wo sie war, wenn du sie anschaltest.

## Mods an- und ausschalten

- Klick auf das Kästchen einer Zeile, oder Rechtsklick und **Einschalten** / **Ausschalten** wählen.
- Braucht eine Mod andere Mods, listet ein Fenster auf, was mit ihr angeht (und was mit ihr ausgeht). Wähle **Alle N aktivieren** oder **Nur ...** für diese eine Mod allein.
- **Alle aktivieren** und **Alle deaktivieren** wirken nur auf die gerade gezeigten Zeilen (nach Suche und Filtern) und fragen vorher.
- **Rückgängig** / **Wiederholen** (die Pfeile daneben, oder Strg+Z / Strg+Y) gehen durch die hier gemachten Änderungen zurück und vor: Verschieben, Ziehen, Schalten, Sortieren. Jedes Profil hat seine eigenen, auch nach dem Schließen der App.
- Strg+Klick wählt mehrere Mods, Umschalt+Klick einen Bereich, Esc lässt sie los. Rechtsklick auf die gewählten: anschalten oder ausschalten, aus dem Profil nehmen oder ihre Workshop-Artikel deabonnieren.
- Ein offenes Entwurfspaket aus dem Mod-Merger lässt sich erst anschalten, wenn es gepackt ist.

## Die Ladereihenfolge ändern

- Zieh eine Zeile an ihrer Nummer und lass sie über oder unter einer anderen Zeile los. Versteckte Zeilen dazwischen werden berücksichtigt.
- Die Pfeilknöpfe rechts: ganz nach oben, eins hoch, eins runter, ganz nach unten. Hoch und runter tauschen mit der nächsten Mod der vollen Liste, die auch eine ausgeschaltete Mod sein kann.
- **Pos. #**: Gib eine Nummer ein und drück **Los** oder Enter. Die Mod geht an diesen Platz unter den Mods, die an sind.
- **Abhängigkeiten auto-sortieren**: setzt jede Mod hinter das, was sie benötigt, und hinter ihre `loadModAfter`-Mods, vor ihre `loadModBefore`-Mods und hinter die Mods, die deine eigenen Regeln nennen. Alles andere behält deine Reihenfolge; Pakete von PZ Mod Studio kommen ans Ende. Ein Fenster sagt, wie viele Mods verschoben wurden, mit **Rückgängig**.

## Mods finden

- Das Suchfeld findet den Namen, die Mod-ID, die Workshop-ID und deine Notizen.
- Die Filterknöpfe: **Alle**, **An**, **Aus**, **Favoriten**, **Aktualisiert**, **Probleme**, **Karten**, **Bibliotheken**, **Lädt nicht**. Jeder zeigt seine Anzahl.
- Darunter: **Sortieren: Ladereihenfolge**, **Sortieren: Name**, **Sortieren: neueste Installationen** oder **Sortieren: älteste Installationen** (danach, wann jede Mod installiert wurde: wann ihr Ordner erstellt wurde; bei einer Workshop-Mod der erste Download, den Updates nicht ändern; jede Zeile zeigt das Datum, und nicht installierte Mods kommen ans Ende), das Menü **Gruppen**, eine deiner Gruppen, die Quelle (**Steam Workshop**, **Deine Workshop-Uploads**, **Von Hand installiert**, **Kompatible Kopien**...), ein Tag (Fehler oder Warnungen in der letzten Sitzung, langsam beim Laden, schwer für das Spiel, ein Problem der Diagnose) und **Mods des Profils** / **Alle installierten**.
- Der Mods-Bildschirm des Spiels (Mod Studio Link) hat dieselben Filter in derselben Reihenfolge. Aktualisiert, Bibliotheken, die Gruppe, die Quelle und das Tag brauchen PZ Mod Studio; ohne es sagt das Spiel das dort, wo sie wären.
- Ziehen funktioniert auch bei Sortierung nach Name: Die Mod geht an den Platz der Mod, auf der du sie loslässt.
- **Filter zurücksetzen** zeigt wieder jede Mod.

## Gruppen und Notizen

- Das Menü **Gruppen** ordnet die Liste in einklappbaren Abschnitten an: **Gruppen: keine**, **Gruppen: Autor**, **Gruppen: Kategorie** oder **Gruppen: meine**. Jede Überschrift zeigt, wie viele an sind, mit **Alle an** / **Alle aus**. In **Gruppen: meine** bearbeitet ein Notiz-Knopf die Notiz der Gruppe.
- Rechtsklick auf eine Mod und **Gruppen…** wählen, um die Gruppen anzuhaken, in denen sie ist, eine neue zu erstellen (**Hinzufügen**) oder eine umzubenennen oder zu löschen. Eine leer gewordene Gruppe verschwindet. Gruppen ändern nie die Ladereihenfolge.
- Wählst du eine deiner Gruppen, erscheinen **Gruppe einschalten** (mit dem, was ihre Mods benötigen) und **Gruppe ausschalten**.
- Rechtsklick und **Notiz hinzufügen…** wählen, um eine Notiz zu einer Mod zu schreiben (das Feld rechts kann das auch). Ihre Zeile zeigt den Anfang der Notiz neben dem Namen: Fahr mit der Maus darüber, um alles zu lesen, oder klick darauf, um sie zu bearbeiten.
- Jedes Profil hat seine eigenen Gruppen: Eine Gruppe gehört zu dem Profil, in dem du sie erstellt hast. Notizen gehören zu einer Mod und sind in jedem Profil gleich.
- Notizen und Gruppen gehören nur dir. Das Spiel zeigt sie (die Gruppen des aktiven Profils) auf dem Mods-Bildschirm der Mod Studio Link.

## Die Mods des Profils

Mit aktivem Profil zeigt die Liste die Mods dieses Profils. Der Knopf **Mods des Profils** wechselt zu **Alle installierten**, das jede installierte Mod auflistet und jede mit den Profilen markiert, in denen sie ist. Installierte Mods, die in keinem Profil sind (zum Beispiel über die Steam-Website abonniert), bekommen einen blauen Balken mit **Zum Profil hinzufügen** (sie kommen ausgeschaltet dazu), **Anzeigen** oder dem X, um ihn auszublenden.

## Das Rechtsklick-Menü

**Aktivieren/Deaktivieren**, **Zum Profil hinzufügen** / **Aus dem Profil nehmen** (herausnehmen schaltet sie auch aus), **Ganz nach oben**, **Ganz nach unten**, **In Mods finden öffnen** (**In Mods finden suchen** für eine nicht installierte Mod), **Ihren Ordner öffnen**, **Notiz hinzufügen…** (**Notiz bearbeiten…**, sobald sie eine hat), **Gruppen…**, **Laderegeln…**, **Diese Version behalten**, **Versionen…**, **Favorit**, **Zu einer Liste…**, **Zu einem Profil…**, **Mod-ID kopieren** und **Workshop-Artikel deabonnieren** (Steam löscht ihre Dateien; andere Mods im selben Artikel gehen mit).

- **Laderegeln…**: Leg fest, dass eine Mod nach einer anderen laden muss (Dinge, die du beim Spielen herausgefunden hast). Die Auto-Sortierung folgt der Regel, und die Diagnose warnt, wenn die Liste sie bricht. Um deine Reihenfolge an jemand anderen weiterzugeben, exportiere eine Voreinstellung: Sie enthält die Liste in der Reihenfolge, die deine Regeln ihr gegeben haben.
## Zu einer früheren Version zurückgehen

Steam behält keine alten Versionen einer Workshop-Mod: Ein Update ersetzt ihre Dateien, und die vorherige lässt sich nicht herunterladen. Deshalb behält das Studio von selbst eine Kopie der installierten Version jeder Workshop-Mod, die ein Profil nutzt, bevor Steam sie ersetzt. Macht ein Update etwas kaputt, ist die Version von vorher schon auf diesem PC.

- **Versionen…** (Rechtsklick; auch in Einfach) listet die gespeicherten Versionen der Mod: wann jede gespeichert wurde, ihre modversion, wann ihr Autor sie veröffentlicht hat, und Tags: **jetzt installiert** (dieselben Dateien, die Steam hat), **in Nutzung**, **vom Studio gespeichert** oder **von dir gespeichert**.
- **Diese Version nutzen** legt sie als lokale Kopie in `Zomboid/mods`, die das Spiel statt der Workshop-Version liest (wenn es aus dem Studio gestartet wird). Der Workshop-Ordner wird nicht angerührt, also macht Steam es nicht rückgängig. Abgelehnt, solange das Spiel läuft.
- **Zurück zur Workshop-Version** (im selben Fenster, oder die Lösung in der Diagnose, wo die Mod **Gespeicherte Version in Gebrauch** zeigt) verschiebt die Kopie in die Backups; das Spiel liest wieder die Workshop-Version.
- Das Feld der Mod sagt, wenn eine frühere Version gespeichert oder eine in Gebrauch ist, mit einem Knopf **Versionen…**.
- **Diese Version behalten** (Vollständig) speichert die Mod, wie sie heute ist, von Hand: Das Größenlimit wirft sie nie heraus.

Die Kopien liegen in `Zomboid/PZModStudio_Versions` oder in dem Ordner, der unter Einstellungen, App gewählt wurde (**Ordner wählen…**, zum Beispiel ein anderes Laufwerk). Jede Datei wird einmal gespeichert, also kostet eine Version nach einem Update nur das, was das Update geändert hat. Das Limit (anfangs 10 GB; auf dem Laufwerk bleiben immer 15 % seiner Größe frei) und der Schalter sind unter Einstellungen, App; über dem Limit fallen zuerst die ältesten früheren Versionen weg, dann Kopien von Mods, die nicht im aktiven Profil sind. Früher gemachte Kopien (`Zomboid/PZModStudio_Snapshots`) werden weiter aufgelistet.

## Tags in den Zeilen

Eine Zeile zeigt nur, was nach etwas verlangt, damit die Liste lesbar bleibt:

- Warnungen der Diagnose, bei denen du handeln musst oder bei denen es sich lohnt (wie **Braucht eine Mod, die aus ist** oder **Lädt zu früh**), mit dem Schraubenschlüssel-Knopf daneben, der die Lösung anwendet. Klick auf das Tag, um zur betroffenen Mod oder zur Diagnose zu gehen.
- Die, die nur gut zu wissen sind, werden in einem kleinen Tag mit einem **i** und ihrer Anzahl zusammengefasst; das Feld rechts zeigt sie alle.
- Fehler oder Warnungen, die das Spiel beim letzten Mal für die Mod aufgezeichnet hat, Laden über 1 Sekunde und ihr Anteil an der Zeit des Spiels ab 5 % (gemessen unter Diagnose > Mods > Zeit pro Mod).
- Woher sie kommt, nur wenn nicht von Steam (**Lokal**, **Kompat. Kopie**...).
- Bei **Alle installierten** ein Profil-Symbol mit einer Zahl: **✓**, wenn sie im aktiven Profil ist (und **+N**, wenn auch in anderen), oder wie viele Profile sie enthalten. Fahr mit der Maus darüber für ihre Namen.

Kürzere Ladezeiten, die Herkunft Steam und alles andere stehen im Feld rechts.

## Frühere Listen und Voreinstellungen

- **Frühere Listen** ist Rückgängig für die Liste. Jedes Mal, wenn das Studio sie geschrieben hat, wurde die vorherige aufbewahrt. Wähle eine und drück **Diese zurückholen**; deine aktuelle Liste wird auch aufbewahrt.
- Der Knopf **Voreinstellungen** speichert die Liste als benannte Voreinstellung, lädt eine oder exportiert und importiert `.pzpack`-Dateien. Ein importiertes Paket listet die Mods, die dir fehlen, mit **Alle abonnieren**.

## Gut zu wissen

- **Aktualisieren** durchsucht die Mod-Ordner neu, nachdem du abonniert oder deabonniert hast.
- Die Mod PZ Mod Studio Link bleibt in jedem Profil an und ist in der Liste ausgeblendet, solange ein Profil aktiv ist.
- Während das Spiel läuft, erreichen Änderungen von hier es im Hauptmenü (über die Mod Studio Link) oder mit **Mit PZ Mod Studio abgleichen** auf seinem Mods-Bildschirm.
