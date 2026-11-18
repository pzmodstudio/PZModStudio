# Entwickler

Der Bildschirm Entwickler ist für Leute, die Mods machen. Du kannst eine neue Mod erstellen, jede Datei einer Mod prüfen, bevor das Spiel es tut, das Spiel nur mit dieser Mod starten, ihr Lua bei laufendem Spiel neu laden, sie auf Mehrspieler und gegen den installierten Build prüfen, Übersetzungsentwürfe schreiben, sie im Steam Workshop veröffentlichen und die Karteneditoren starten.

## Was du siehst

Der Bildschirm erscheint nur in der Seitenleiste, wenn **Entwicklerfunktionen** in den **App-Einstellungen** an ist, und wie die anderen Bildschirme erst, wenn ein Profil aktiv ist. Oben enthält **Die Mod, an der du arbeitest** die Mod, mit der jedes Werkzeug arbeitet, mit **Ihren Ordner öffnen** und **Spiel nur mit dieser Mod starten**. Darunter gibt es einen Reiter pro Werkzeug: **Dateien prüfen**, **Live-Reload**, **Mehrspieler**, **Gegen den Build**, **Übersetzungen**, **Workshop**, **Karten** und **Neue Mod**. Ein Reiter zeigt ein kleines Abzeichen, wenn er etwas zu sagen hat (gefundene Probleme, Beobachtung läuft, das Mehrspieler-Ergebnis). Das Studio merkt sich den zuletzt geöffneten Reiter. Jeder Reiter außer **Workshop**, **Karten** und **Neue Mod** braucht zuerst eine gewählte Mod.

## Eine neue Mod erstellen

1. Öffne den Reiter **Neue Mod**.
2. Fülle **Name**, **ID** (der Ordner und der Listeneintrag: Buchstaben, Ziffern, `_` und `-`; sie folgt dem Namen, bis du sie änderst), **Autor** und **Beschreibung** aus.
3. Hak **Auch Build 41** an, wenn die Mod auch auf Build 41 funktionieren soll.
4. Drück **Anlegen**.

Das Studio erstellt die Mod in `Zomboid/mods` so, wie Build 42 sie liest: `42/mod.info`, die Ordner `42/media/lua/client`, `server` und `shared`, `42/media/scripts`, `common/media/lua`, ein Poster zum Ersetzen und eine Lua-Datei, die den Namen der Mod ausgibt, wenn das Spiel sie lädt. Es lehnt eine ID ab, die kein einfacher Name ist, oder einen Ordner, der schon existiert. **Ordner öffnen** öffnet ihn im Windows Explorer, und die neue Mod wird als die gewählt, an der du arbeitest. Schalte sie in der Mod-Liste an, wenn sie fertig ist.

## Die Mod wählen, an der du arbeitest

Unter **Die Mod, an der du arbeitest** wählst du eine Mod aus der Liste. Die Mods sind nach ihrer Herkunft gruppiert, die, an denen du arbeitest, zuerst: **Deine Workshop-Uploads (Zomboid/Workshop)**, **Ordner in Zomboid/mods**, **Installiert von Nexus, aus einer Datei oder einem Link**, die Kopien, die das Studio an deinen Build angepasst hat, **Die eigenen Mods des Studios** und zuletzt **Workshop-Abos**, zum Ansehen. Unter der Liste siehst du ID, Version, Workshop-Nummer und Ordner der Mod. **Ihren Ordner öffnen** öffnet den Ordner der Mod. Das Studio merkt sich deine Wahl.

## Jede Datei prüfen

Drück im Reiter **Dateien prüfen** auf **Alle Dateien prüfen**. Jede `.lua`-Datei geht durch denselben Lua-Parser, den der Mod-Merger nutzt, und jede `media/scripts/*.txt` durch eine Klammerprüfung (das Spiel verliert stillschweigend eine ganze Skriptdatei beim ersten unausgeglichenen Block). Das Ergebnis erscheint direkt unter dem Knopf, mit der Anzahl der geprüften Lua- und Skriptdateien. Es sagt **Jede Datei lässt sich lesen** oder listet die Probleme nach Datei mit Zeile und Spalte. Klick auf ein Problem, um die Datei an dieser Zeile in VS Code zu öffnen, wenn es installiert ist, sonst in dem Programm, das Windows für diese Datei nutzt.

Das prüft nur die Syntax. Was das Spiel mit den Dateien macht, zeigt der Monitor.

## Das Spiel nur mit dieser Mod starten

**Spiel nur mit dieser Mod starten**, oben auf dem Bildschirm, startet das Spiel in Debug (Fenstermodus) nur mit dieser Mod, dem, was sie benötigt, und der Bridge des Studios. Deine Mod-Liste wird beiseitegelegt, und ein Banner sagt das. Sie kommt zurück, wenn das Spiel sich schließt, oder sofort mit **Meine Liste jetzt zurückholen**. Die Liste wird außerdem unter **Frühere Listen** in der Mod-Liste aufbewahrt. Der Knopf ist aus, solange so ein Start läuft.

## Im laufenden Spiel neu laden

So kannst du eine Lua-Datei ändern und sie ohne Neustart im Spiel sehen.

1. Starte das Spiel mit der Bridge-Mod (`Z_PZModStudio_Bridge`) an.
2. Öffne den Reiter **Live-Reload** und schalte den Schalter rechts an. Er zeigt **Beobachte (Name der Mod)**.
3. Speichere eine Lua-Datei der Mod. Das Studio sieht sie im Moment des Speicherns, prüft sie mit dem Parser und schickt sie ans Spiel, das sie im Hauptmenü, im Spiel oder pausiert neu lädt.

**Alle Lua-Dateien dieser Mod jetzt neu laden** schickt alle Lua-Dateien der Mod auf einmal.

Das Log unter den Knöpfen zeigt jeden Schritt:

- **geändert:** Eine Datei wurde ans Spiel geschickt.
- **nicht gesendet, nicht lesbar:** Die Datei hat einen Syntaxfehler, mit Zeile, Spalte und Meldung. Sie wird nicht geschickt, also führt das Spiel nie kaputten Code aus.
- Die Antwort des Spiels für jede Datei, grün, wenn es geklappt hat, und rot, wenn es fehlgeschlagen ist.
- Die Zeiten: erkannt, gesendet, im Spiel angewendet und zurück im Studio, in Millisekunden.
- **Das Spiel läuft nicht** oder **Keine Antwort vom Spiel in 6 s**, wenn niemand zum Neuladen da war (prüfe, ob die Bridge-Mod an ist).

Wird eine Datei neu geladen, läuft ihr Code auf oberster Ebene noch einmal, und die Bridge bewahrt, was das kaputt machen würde:

- **Event-Handler**: Jeder, den die Datei hinzufügt, läuft einmal, ein Handler, den du zum ersten Mal hinzufügst, fängt an zu arbeiten, und einer, den du aus der Datei nimmst, hört auf.
- **Tabellen**: Erstellt die Datei eine globale Tabelle neu (`MyMod = {}`, `MyPanel = ISPanel:derive(...)`), wird die alte Tabelle behalten und bekommt die neuen Funktionen, sodass offene Fenster und gespeicherte Referenzen den neuen Code mit ihrem Zustand ausführen. Ein Wert, den die alte Tabelle schon hatte, behält seinen alten Wert (die Bridge kann deinen Zustand nicht von einer geänderten Konstante unterscheiden): Für eine geänderte Konstante nimm ein vollständiges Reset Lua.
- **Deine eigene Einrichtung**: `if PZStudio then PZStudio.onReload(function(file) ... end) end` läuft nach jedem Neuladen dieser Datei, für das, was `OnGameStart` getan hat und nicht noch einmal tut.
- **Dateien, die sie per `require` laden**: Die anderen Dateien der Mod, die die geänderte Datei mit `require` laden, werden direkt danach neu geladen, damit sie ihren neuen Code bekommen.

Das Log sagt, was bewahrt wurde, zum Beispiel "1 handler(s), 1 table(s) kept".

## Mehrspieler-Prüfung

Im Reiter **Mehrspieler** sucht **Diese Mod auf Mehrspieler prüfen** im Client- und Shared-Lua der Mod nach Aufrufen, die die Welt, ein Fahrzeug, ein Tier oder die Karte direkt ändern, wo der eigene Code des Spiels stattdessen den Server fragt. Jeder Befund zeigt die Datei, die Zeile und den Handler des Spiels, der das auf dem Server erledigt. Ein Aufruf innerhalb eines Zweigs `if isClient()` gilt als behandelt. Klick auf einen Befund, um ihn in deinem Editor zu öffnen. Es prüft nur diese Aufrufe: Eine Mod ohne Einträge kann sich im Mehrspieler trotzdem auf Arten danebenbenehmen, die das hier nicht sieht.

## Gegen den installierten Build

Im Reiter **Gegen den Build** schlägt **Namen gegen diesen Build prüfen** jeden Namen, den das Lua der Mod aufruft, im installierten Spiel nach: seine Java-Klassen und -Methoden, seine globalen Funktionen, sein eigenes Lua und die Events, die es auslöst, dazu das, was diese Mod und deine anderen aktiven Mods definieren. Was übrig bleibt, ist ein Aufruf, der an dieser Zeile fehlschlagen würde: eine Funktion, die Build 42 gestrichen hat, eine umbenannte Methode, ein Event, das nicht mehr ausgelöst wird, ein Tippfehler. Die Befunde sind gruppiert nach globalen Funktionen, Methoden und Events, jeder mit Datei und Zeile, und einem "Meintest du", wenn ein bekannter Name ähnlich ist. Klick auf einen, um ihn in deinem Editor zu öffnen. Ein Aufruf, den die Datei auch ohne Aufruf nennt (`if foo then`), gilt als abgesichert und wird weggelassen.

Die erste Prüfung liest die Dateien des Spiels und dauert ein paar Sekunden; danach wird sie pro Build aufbewahrt. Sie geht nur nach Namen: Ein Name, der erst bei laufendem Spiel gebildet wird, wird nicht gesehen, und eine Mod mit Java kann Namen definieren, die Lua nicht zeigen kann (das Ergebnis sagt das). Öffne die Zeile, um es zu beurteilen.

## Übersetzungsentwürfe

Der Reiter **Übersetzungen** zeigt die Sprache, in der die Mod geschrieben ist, wie viele Texte sie hat, und ihre Sprachen in Gruppen: die mit fehlenden Texten, die vollständigen und Ordner der Mod, die keine Sprachen des Spiels sind. Hak die gewünschten Sprachen an und drück **Entwürfe schreiben für…**: Das Studio schreibt einen Entwurf jedes Textes, der einer Sprache fehlt, übersetzt vom selben Dienst, der die Beschreibungen von Mods übersetzt (**Übersetzung** in den **App-Einstellungen**), in den eigenen Ordner `Translate/<LANG>/` der Mod. Was schon da ist, bleibt, wie es ist. **Translate-Ordner öffnen** öffnet ihn. Ein Entwurf ist ein Anfang, keine Übersetzung: Lies ihn vor dem Veröffentlichen durch.

## Im Workshop veröffentlichen

Der Reiter **Workshop** lädt eine Mod über den Steam-Client, der auf diesem PC läuft, in den Steam Workshop hoch. Er nutzt dieselben Dateien wie der eigene Uploader des Spiels, in `Zomboid/Workshop/<item>/`: `workshop.txt`, `preview.png` und `Contents/mods/<mod>/`.

1. Wähle einen Artikel in der Liste. Für eine eigene Mod in `Zomboid/mods`, die noch keinen Artikel hat, drück **(Mod) für den Workshop vorbereiten**: Das erstellt den Artikelordner mit einer Kopie der Mod, eine `workshop.txt` aus ihrer `mod.info` und ihr Poster als Vorschau.
2. Fülle **Titel**, **Sichtbarkeit** (**Öffentlich**, **Nur Freunde**, **Privat** oder **Nicht gelistet (per Link)**), **Tags (getrennt durch ;)** und **Beschreibung** aus (Steam nimmt bis zu 8000 Zeichen) und drück **workshop.txt speichern**.
3. **Dateien ansehen** zeigt, was gesendet würde und was sich seit dem letzten Upload aus dem Studio geändert hat. Eine `.workshopignore` im Artikelordner (ein Muster pro Zeile, etwa `*.psd` oder `.git/`) hält Dateien draußen.
4. Schreib, wenn du willst, einen Änderungshinweis und drück **Im Workshop veröffentlichen** (oder **Update hochladen** für einen schon veröffentlichten Artikel). Du wirst vorher gefragt. Steam muss laufen und mit dem Konto angemeldet sein, dem der Artikel gehört.

Gesendet wird, was in `workshop.txt` steht: Eine Änderung auf der Steam-Seite wird beim nächsten Upload ersetzt, so wie es das Spiel macht. Hält Steam einen neuen Artikel versteckt, bis du die rechtliche Vereinbarung des Workshops akzeptierst, sagt das Ergebnis das, mit einem Link. **Workshop-Seite ansehen** öffnet den Artikel in deinem Browser. Das Studio lädt keine Mod von jemand anderem hoch: Enthält der Artikel eine Mod, die du abonniert hast, sagt es das, und der Knopf bleibt aus. Der Papierkorb-Knopf neben der Liste nimmt den Artikelordner aus `Zomboid/Workshop` (in den Papierkorb); auf Steam ändert sich nichts.

## Karten-Werkzeuge

Im Reiter **Karten** zeichnet TileZed die Tiles jeder Zelle (Gebäude, Räume, Möbel, Boden); WorldEd legt Zellen auf das Weltraster und schreibt die Kartendateien, die das Spiel liest. Sie kommen von The Indie Stone als ein Zip, das du irgendwo entpackst. Das Studio sucht sie an den üblichen Orten (dein Benutzerordner, Dokumente, Desktop, Downloads, Laufwerk C:) und in dem Ordner, den du in das Feld unter jedem eintippst. Findet es eins, startet **TileZed starten** oder **WorldEd starten** es aus seinem eigenen Ordner. Die Zellen einer Karten-Mod gehören in `media/maps/<name>` der Mod.

## Gut zu wissen

- Neuladen braucht das laufende Spiel mit der Bridge-Mod an; die anderen Werkzeuge brauchen das Spiel nicht.
- Neuladen im Spiel funktioniert ohne Neustart; eine Konstante, die du in einer Tabelle geändert hast, behält ihren alten Wert bis zu einem vollständigen Reset Lua.
- Jedes Werkzeug außer **Workshop**, **Karten** und **Neue Mod** braucht eine gewählte Mod, deren Ordner das Studio kennt.
