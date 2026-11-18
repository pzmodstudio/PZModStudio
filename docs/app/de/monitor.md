# Monitor

Der Monitor zeigt das Log des Spiels, während es geschrieben wird, und macht aus jedem Fehler eine Karte, die sagt, welcher Mod das Spiel die Schuld gab und was du dagegen tun kannst. Er verfolgt `console.txt`, egal ob du das Spiel aus dem Studio oder aus Steam gestartet hast.

## Was du siehst

- Links die Log-Ansicht, mit einem Reiter pro Log.
- Rechts **FEHLER UND WARNUNGEN**: die Fehlerkarten, und darüber das Feld **Die Mod finden, die es auslöst**.
- Oben öffnet **Sehen, wie das Spiel läuft** Diagnose > Spiel > Live: Arbeitsspeicher, CPU und auf welchem Bildschirm das Spiel ist.
- Über dem Log, solange das Spiel läuft (volle Ansicht), das Feld **Live**: siehe unten.
- Oben öffnet **📁 Log-Ordner** den Ordner mit `console.txt` und dem Unterordner `Logs` im Windows Explorer.
- Daneben der Spamfilter-Knopf. Solange er **Spam-Filter: AKTIV** zeigt, blendet er die Zeilen über fehlende Übersetzungen aus; drück ihn, um zu **Filter: ALLES ZEIGEN** zu wechseln.

Ist der Spielordner noch nicht gesetzt, zeigt der Bildschirm nur **Ordnerpfade in den Einstellungen festlegen**.

## Live (während das Spiel läuft)

In der vollen Ansicht sitzt bei offenem Spiel ein Feld **Live** über dem Log. Seine Titelzeile fasst es zusammen (FPS, Java-Speicher, CPU, Lua-Fehler); ein Klick klappt es ein. Es wird alle 2 Sekunden gelesen:

- **FPS, letzte Minuten**: eine Linie der letzten 10 Minuten (fahr mit der Maus darüber, um einen Moment abzulesen); die gestrichelte Linie ist 30 FPS. Darunter der durchschnittliche Frame und der längste Frame der letzten 10 Sekunden, der Speicher, den Java gegenüber seinem Limit nutzt, der Speicher des ganzen Prozesses mit dem, was der PC frei hat, und die CPU. Eine zweite Linie verfolgt den Speicher.
- **Lua-Zeit jetzt, nach Mod**: die 5 Mods, deren Lua in den letzten 10 Sekunden die meiste Zeit gebraucht hat, in Millisekunden pro Sekunde Spielzeit. Das kommt vom Profiler des Studios. Ist der Profiler aus, wird nichts gemessen, und das Feld sagt das mit **Für den nächsten Start einschalten**: Der Profiler kommt an die erste Stelle der Mod-Liste (und des zuletzt gespielten Spielstands) und misst ab dem nächsten Spielstart.
- **Lua-Fehler in dieser Sitzung**: wie viele, und welchen Mods das Spiel die Schuld gab, so wie der Studio Link sie im Spiel zählt; darunter die neuesten Fehlerkarten, markiert mit **neu**, wenn sie kamen, während du zugesehen hast.
- Eine letzte Zeile sagt, ob das Overlay des Links im Spiel zu sehen ist und was sein eigener Code kostet.

FPS, Java-Speicher und Fehler kommen vom Studio Link (Version 1.20 oder neuer), sobald du in einem Spiel bist. Das Feld verschwindet, wenn das Spiel sich schließt.

### Das Overlay im Spiel

Der Studio Link kann dieselben Zahlen über dem Spiel zeigen: FPS und Frame-Zeit, Java-Speicher gegenüber seinem Limit, CPU, die 5 Mods mit der meisten Lua-Zeit in den letzten 10 Sekunden (mit Profiler an) und die Lua-Fehler der Sitzung mit den beschuldigten Mods. Öffne es im Pausenmenü mit **Live-Overlay** oder gib ihm eine Taste unter **Einstellungen > Mods > PZ Mod Studio Link** im Spiel (es hat keine, bis du eine wählst, damit es nie eine Taste des Spiels belegt). Zieh es, wohin du willst; es bleibt dort und kommt im nächsten Spiel wieder. Seine letzte Zeile sagt, was es selbst kostet, in Millisekunden pro Sekunde. Das Speicherlimit und die CPU sind nur bekannt, solange PZ Mod Studio offen ist.

## Die Log-Ansicht

Zwei Reiter sind immer da:

- **console.txt (Client)**: das Live-Log des Spiels. Das Studio beginnt am Ende der Datei zu lesen, also siehst du, was ab dem Moment passiert, in dem das Studio offen ist. Beginnt eine neue Spielsitzung, fängt die Datei von vorn an, und das Studio liest sie ab ihrer ersten Zeile. Der Reiter behält die letzten 1.500 Zeilen, auch während du auf einem anderen Bildschirm bist.
- **Dedizierter Server (Live)**: das Log des dedizierten Servers, solange einer läuft, den du unter **Server** gestartet hast. Es zeigt seine letzten 1.500 Zeilen und wird alle 2 Sekunden neu gelesen.

Um andere Logs zu lesen, drück **Log laden**. Das Menü listet die Logs in deinem Zomboid-Ordner (mit ihrer Größe); wähle eins, um es in einem neuen Reiter zu öffnen. **📂 Andere Datei wählen (.txt)...** öffnet eine beliebige Textdatei. Ein geladenes Log zeigt seine letzten 2.000 Zeilen. Diese Reiter lassen sich mit ihrem X schließen; die beiden Live-Reiter nicht.

Unter den Reitern:

- Das Suchfeld behält nur die Zeilen, die enthalten, was du eintippst.
- Die Etiketten **Alle**, **🚨 Fehler**, **🌙 Lua** und **☕ Java** behalten nur Zeilen dieser Art. **⚡ Bridge** erscheint nur mit Entwicklerfunktionen an.
- Die Fußzeile zeigt, wie viele Zeilen von der Gesamtzahl gezeigt werden.

Die kleinen Knöpfe rechts neben den Reitern:

- Der Disketten-Knopf speichert, was der Reiter enthält, als neue Datei in `Zomboid/Logs/`.
- Der Kopieren-Knopf kopiert die sichtbaren Zeilen (nach den Filtern) in die Zwischenablage.
- Der Papierkorb-Knopf leert die Ansicht für diesen Reiter. Die Datei auf der Festplatte rührt er nicht an.

Fehlerzeilen werden rot gezeigt, Warnungen gelb.

## Die Fehlerkarten

Eine Karte erscheint, wenn das Spiel einen Fehler wirft oder wenn es eine Warnung schreibt und eine Mod nennt. Alles andere bleibt im Log links. Karten mit geworfenem Fehler (rot) kommen zuerst, Warnungen (gelb, markiert mit **Warnung · das Spiel lief weiter**) danach. Dasselbe Problem erscheint einmal, egal wie oft das Spiel es wiederholt; die Zahl wie **600×** sagt, wie oft es im Log steht, und die, die sich wiederholen, sind die, die du zuerst beheben solltest.

Über den Karten zählt ein Abzeichen die Exceptions und ein anderes die Warnungen (jede Karte sagt, ob etwas zu tun ist), mit einer Zeile, die das Log zusammenfasst. Erinnert sich das Studio an frühere Sitzungen, sagt jede Karte **neu** oder **gesehen in N Sitzungen seit** einem Datum, und **Nur die neuen** blendet den Rest aus.

Eine Karte kann zeigen:

- **Den Namen der Mod**, wenn das Spiel sie in der Fehlerzeile oder im Stacktrace darunter genannt hat.
- **aufgerufen von ...**: die Mods, deren Code die fehlgeschlagene aufgerufen hat, aus dem eigenen Stack des Spiels gelesen. Der Fehler kann in jeder von beiden liegen.
- **Log-Ausschnitt**: die Zeile selbst.
- Bei einem fehlgeschlagenen `require`, wo dieses Modul jetzt wirklich liegt, wenn das Studio es findet.
- Was als Nächstes zu tun ist, oder einen Hinweis, dass der Build-42-Shim es schon abdeckt.
- Wenn der Fehler in einer Kopie liegt, die dein Studio-Paket enthält: einen Hinweis, wenn die Mod diese Datei nach dem Erstellen der Kopie geändert hat oder wenn keine Mod deiner Liste diese Datei mehr mitbringt.

Beginnt eine neue Spielsitzung, werden die Karten der letzten geleert. **Verwerfen** leert sie von Hand.

## Von einer Karte aus beheben

- **Pfad korrigieren** (oder **Pfad in File.lua korrigieren**): erscheint, wenn eine Mod ein Modul über einen alten Pfad anfordert und das Studio den neuen kennt. Es schreibt das `require` in jeder Datei der aktiven Mods um, die es anfordert, und veröffentlicht sie in dein Studio-Paket. Die Mod wird nicht verändert und bleibt an. Eine grüne Zeile sagt, was behoben wurde; starte das Spiel neu.
- **File.lua patchen** oder **Eine Datei davon patchen**: öffnet die Dateien dieser Mod im Mod-Merger, damit du eine korrigieren kannst. Deine Kopie kommt in ein Paket, das zuletzt lädt, also liest das Spiel sie statt der der Mod. Warnungen ohne bekannte Lösung bekommen diesen Knopf nicht.
- **Kopie von File.lua entfernen**: erscheint, wenn der Fehler in einer Datei liegt, die dein Studio-Paket schon gepatcht hat. Es nimmt diese Kopie aus dem Paket (nach Rückfrage), und das Spiel liest wieder die eigene Datei der Mod. Jede Änderung, die du an der Kopie gemacht hast, geht verloren.
- **Deaktivieren**: fragt nach einer Bestätigung, schaltet die Mod aus und bringt dich zur Mod-Liste.
- **Ein Merge-Paket könnte das beheben — Mod-Merger öffnen**: für Fehler, die der Build-42-Shim beantwortet. Der Shim steckt in einem veröffentlichten Merge-Paket; die Karte sagt dir, ob deins schon aktiv oder noch ein Entwurf ist.

## Berichte

- **Bericht speichern** schreibt eine Markdown-Datei mit dem Fehler, deinem Build des Spiels, der Mod und ihrem Code sowie den Log-Zeilen drumherum. Sie ist fertig, um sie einem Mod-Autor oder einem KI-Assistenten zu geben.
- **Bericht kopieren** legt denselben Bericht in die Zwischenablage.
- **🔍 Im Log ansehen** setzt die Datei oder das Stichwort des Fehlers ins Suchfeld links.
- **Zeile kopieren** kopiert nur die Fehlerzeile.

## Die Mod finden, die es auslöst

Für Probleme, die das Log nicht nennt: ein Einfrieren, ein Rezept, das nie erscheint, ein Spielstand, der sich nicht öffnet.

1. Öffne das Feld. Passiert das Problem nicht jedes Mal, hak **Passiert nicht jedes Mal** an. Zeigt das Log etwas Text des Fehlers, tipp ihn ein: Nach jedem Testspiel liest die App das Log und schlägt die Antwort vor. Drück **Liste halbieren**, dann bestätige.
2. Ein Teil deiner Verdächtigen wird angeschaltet; die, die dieser PC schon verdächtigt (schiefgelaufene Sitzungen, Fehler, die sich wiederholen), werden zuerst probiert. Mods, die eine andere Mod benötigt, bleiben in jeder Runde an, damit dadurch nichts kaputtgeht.
3. Drück **Spiel starten**, spiel so, wie du es getan hast, als es passiert ist, und schließ es.
4. Antworte **Passiert noch** oder **Ist weg**. Die Liste wird wieder halbiert. Das Abzeichen zeigt die Runde und wie viele Starts übrig sind (etwa 8 bei hundert Mods).
5. Bleibt eine Mod übrig, spiel noch einmal, um es zu bestätigen.

Am Ende nennt das Feld die Mod, zeigt alle Zeilen, die das Log über sie hat (mit **File.lua patchen**, wenn eine Zeile eine Datei nennt, und **Ihre Zeilen in der Konsole zeigen**), und bietet **Eine Datei davon patchen**, **Deaktivieren** oder **Aktiv lassen** an. Deine ursprüngliche Liste wird zurückgesetzt, egal wie die Suche endet. **Abbrechen und Liste zurück** beendet sie vorzeitig.

Verursacht die letzte Mod es nicht allein, geht die Suche von selbst weiter: Sie lässt diese Mod an und sucht die andere Mod, die es zusammen mit ihr verursacht, und nennt dann das Paar. Findet sie weder eine Mod noch ein Paar, sagt das Feld warum: eine der benötigten Mods, mehr als zwei Mods oder gar keine Mod.

## Gut zu wissen

- Die Suche schreibt deine Mod-Liste selbst. Schalte während sie läuft keine Mods in der Mod-Liste an oder aus, sonst bedeutet die nächste Antwort nichts.
- Eine Suche übersteht das Schließen des Studios; sie wird in `Zomboid/PZModStudio_Bisect.json` gespeichert.
- **Spiel starten** in der Suche startet das Spiel normal, ohne Debug-Modus.
- In ein Paket veröffentlichte Korrekturen wirken beim nächsten Spielstart.
