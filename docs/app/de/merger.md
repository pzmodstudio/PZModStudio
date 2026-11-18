# Mod-Merger

Der Mod-Merger repariert Dateien, die zwei oder mehr deiner aktiven Mods beide ändern, und lässt dich einen Fehler in der eigenen Eine Mod-Datei patchen. Deine Entscheidungen kommen in ein Paket: eine kleine eigene Mod, die zuletzt lädt, sodass das Spiel deine Version dieser Dateien liest.

## Was ein Paket ist

- Ein Paket ist ein Mod-Ordner in `Zomboid/mods`, dessen Name mit `Z_PZModStudio_` beginnt (das Standardpaket heißt `Z_PZModStudio_MergedPatch`).
- Beim Veröffentlichen schreibt das Studio deine zusammengeführten Dateien hinein (für Build 41 und in seinen Ordner `42` für Build 42), fügt eine kleine Kompatibilitätsdatei hinzu und setzt das Paket **zuletzt** in die Mod-Liste des Spiels. Die Mod, die zuletzt lädt, ist die, deren Kopie einer Datei das Spiel nutzt, also gewinnt deine zusammengeführte Kopie.
- Ein Paket hat zwei Zustände. **🟢 GEPACKT / GESCHLOSSEN** bedeutet, dass es veröffentlicht ist und das Spiel es liest. **🟡 OFFEN (Entwurf)** bedeutet, dass du es bearbeitest; solange es ein Entwurf ist, wird es aus der Mod-Liste des Spiels genommen, also liest das Spiel es erst, wenn du wieder veröffentlichst.
- Jedes Paket gehört zu den Profilen, die es unter ihren Mods haben. Ein Paket, das du bei aktivem Profil erstellst, wird diesem Profil hinzugefügt.
- Ein Paket merkt sich das Profil, das beim letzten Veröffentlichen aktiv war (**erstellt mit** auf seiner Karte, in Gelb, wenn jetzt ein anderes Profil aktiv ist): Seine Zusammenführungen bereinigen die Liste dieses Profils. Bei einem Profil ohne eigenes Paket erstellt **Paket für "Name"** in der oberen Leiste eins, das nach ihm benannt ist.

## Was du siehst

Ist kein Paket offen, siehst du die Liste **Merge-Pakete**. Jede Karte zeigt den Paketnamen, seine Dateien, seinen Zustand, die Profile, zu denen es gehört, und diese Knöpfe:

- **🔍 Paket öffnen** öffnet es im Arbeitsbereich und durchsucht deine aktiven Mods nach Konflikten.
- **Sichtbar in der Mod-Liste** / **Ausgeblendet in der Mod-Liste** schaltet das Paket in der Mod-Liste des Spiels (`default.txt`) an oder aus.
- Der Stift bearbeitet Name und Beschreibung; der Papierkorb löscht es von der Festplatte (du wirst vorher gefragt). Profile, die es enthielten, behalten nach dem Umbenennen den neuen Namen.
- Das Download-Symbol speichert es als `.pzmerge`-Datei zum Teilen; das Ordnersymbol öffnet seinen Ordner im Windows Explorer.
- **Pakete anderer Profile** zeigt Pakete, die zu anderen Profilen gehören; **Nur "Name"** geht zurück zu denen des aktiven Profils.

Die obere Leiste hat **Paket laden (.pzmerge)**, um ein Paket zu importieren, das dir jemand geschickt hat, und **Neues Paket**, um eins zu erstellen (ein Name, eine optionale Beschreibung, dann **Paket erstellen**). Hat das aktive Profil keine Merge-Pakete, sagt der Bildschirm das und erklärt, wann eins nötig ist, mit einem Knopf, um die Pakete anderer Profile zu sehen. Die eigenen Mods des Studios (die Live IPC Bridge und der Studio Link) sind keine Merge-Pakete: Sie stehen darunter, eingeklappt unter **Studio-Mods**, und du kannst ihre Dateien sehen, aber sie nicht bearbeiten, umbenennen oder löschen.

## In einem Paket arbeiten

Die obere Leiste zeigt **← Zurück zur Liste**, eine Auswahl **Paket:**, um zu einem anderen Paket zu springen, den Zustand und die Sichtbarkeit sowie diese Knöpfe:

- **Eine Mod-Datei patchen**: öffnet die eigene Datei einer Mod, um sie zu reparieren (siehe unten).
- **⚡ Alles zusammenführen und veröffentlichen**: führt jede kollidierende Datei mit der automatischen Engine zusammen und veröffentlicht das Paket sofort: Es wird nach `Zomboid/mods` geschrieben und ans Ende deiner Mod-Liste gesetzt, und das Spiel liest es beim nächsten Start. Sieh dir die Dateien danach an.
- **↩️ Alle Merges lösen**: verwirft jede gespeicherte Zusammenführung in diesem Paket. Eine Kopie bleibt in `Zomboid/PZModStudio_Backups`, und die schon veröffentlichten Dateien bleiben, bis du wieder veröffentlichst.
- **Alle** / **Ohne Duplikate**: blendet Dateien aus, bei denen die Mods identische Kopien mitbringen, sodass nur echte Kollisionen aufgelistet werden.
- Symbole für Ordner, Export und neues Durchsuchen.
- **📦 Packen & veröffentlichen**: der letzte Schritt. Er schreibt die aktuelle Entscheidung jeder Datei ins Paket und setzt das Paket ans Ende der Mod-Liste des Spiels.

Links listet **VFS-Konflikte** jede kollidierende Datei mit ihrer Art (Lua, **DEFINITION** für Gegenstands- und Rezeptskripte, **TEXTE** für Übersetzungen) und ihrem Zustand: **Prüfung nötig**, **Automatisch zusammengeführt** oder **Geklärt**.

## Entscheiden, was das Spiel für eine Datei liest

1. Klick auf eine Datei in der Liste. Die obere Hälfte zeigt die Vanilla-Datei, die Kopie jeder Mod und eine Zusammenfassung dessen, was jede Mod gegenüber dem Grundspiel ändert; echte Konflikte (dieselben Zeilen unterschiedlich geändert) sind markiert. Klick auf eine Änderung, um jedes Feld dorthin zu scrollen.
2. Unter **Was soll das Spiel lesen?** wählst du eine Antwort:
   - **Beide behalten** (für Lua und andere Dateien): die Änderungen aller Mods zusammen. Wo zwei dieselben Zeilen ändern, gewinnt die, die zuletzt lädt, und die andere bleibt in einem Kommentar erhalten.
   - **Zusammenführen** (für Definitionen und Übersetzungen): Was beide Werte aufnehmen kann, behält beide; wo nur einer passt, bleibt, was das Spiel heute tut, und der andere landet in einem Kommentar.
   - **Nur (Name der Mod)**: die Version dieser Mod. Bei einer Definition gelten die Einstellungen der anderen Mods, die diese nicht anfasst, weiterhin.
   - **Vanilla**: die eigene Datei des Spiels, als hätte keine Mod sie angefasst (nicht angeboten, wenn das Grundspiel keine solche Datei hat).
3. Oder bearbeite den Code im Editor darunter; dann zeigt er **Deine eigene Änderung**. **Anhängen** und **Entfernen** im Feld einer Mod fügen deren ganze Kopie hinzu oder nehmen sie heraus, und **Basis laden** setzt die Vanilla-Datei in den Editor.
4. Drück **✓ Merge dieser Datei speichern**. Die Datei wird **Geklärt**.
5. Ist jede Datei erledigt, drück **📦 Packen & veröffentlichen**.

**AST-Merge wiederherstellen** setzt die automatische Zusammenführung zurück in den Editor, und **Merge dieser Datei lösen** vergisst die gespeicherte Entscheidung für diese Datei. Zieh die Leiste zwischen den Feldern, um ihre Größe zu ändern.

## Einen Fehler in einer Mod patchen

Nimm **Eine Mod-Datei patchen**, wenn eine Mod einen Fehler hat und es nichts zusammenzuführen gibt. Wähle die Mod, dann die Lua- oder Skriptdatei, bearbeite sie und drück **✓ Merge dieser Datei speichern**. Deine Kopie kommt ins Paket, und das Spiel liest sie statt der der Mod, nur für diese Datei; die Mod bleibt an und der Rest von ihr unberührt. Eine Kopie ohne Änderungen zu speichern wird abgelehnt, weil das die Datei nur auf dem heutigen Stand einfrieren würde.

Der Monitor und die Diagnose können dich direkt hierher schicken: Das passende Paket öffnet sich, und die Datei, die der Fehler genannt hat, öffnet sich im Editor, manchmal mit schon angewendeter Lösung.

## Gepackte Pakete und Veröffentlichen an Ort und Stelle

Ein gepacktes Paket ist schreibgeschützt: Seine Dateien bleiben hinter einer Zusammenfassung verborgen, bis du **🔓 Merge lösen / zum Bearbeiten öffnen** (oder **Zum Bearbeiten entpacken**) drückst und bestätigst. Entpacken nimmt die veröffentlichten Dateien aus dem Spiel, bis du wieder veröffentlichst.

Die eigene Datei einer Mod, die zum Patchen geöffnet wurde, ist die Ausnahme: In einem gepackten Paket wird beim Speichern **an Ort und Stelle veröffentlicht**. Alles, was das Paket enthält, wird mit deiner Datei daneben neu veröffentlicht, das Paket bleibt gepackt, und das Spiel liest die neue Datei beim nächsten Start. Schicken dich die Diagnose oder der Monitor zu einer Datei eines gepackten Pakets, wird diese Datei gezeigt, ohne es zu entpacken.

## Pakete teilen

**Paket laden (.pzmerge)** importiert eine Paketdatei. Wurde es mit Mods erstellt, die du nicht hast, mit anderen Kopien davon oder auf einem anderen Build des Spiels, listet ein Fenster **Bevor du veröffentlichst** sie auf, mit **Alle abonnieren** für die fehlenden Workshop-Mods. Am sichersten ist es, wenn ihr beide die neuesten Kopien habt, dann neu durchsucht und vor dem Veröffentlichen noch einmal zusammenführt.

## Gut zu wissen

- Das Spiel liest ein Paket beim Start: Starte das Spiel nach dem Veröffentlichen neu, wenn es läuft.
- Wird der Spielordner nicht gefunden, bittet der Bildschirm dich, zuerst die Pfade in den **App-Einstellungen** festzulegen.
- Verlässt du eine Datei mit ungespeicherten Änderungen, wirst du gefragt, ob du sie zurücklassen willst.
- Ein Paket, das du löschst, wird von der Festplatte entfernt; die Mods, die es zusammengeführt hat, werden nicht angerührt.
