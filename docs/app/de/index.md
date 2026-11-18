# PZ Mod Studio

PZ Mod Studio ist ein kostenloser Mod-Manager für Project Zomboid unter Windows und Linux. Er hält deine Mods in Profilen, sichert deine Spielstände, prüft deine Mod-Liste so, wie das Spiel sie liest, und hilft dir zu reparieren, was kaputtgeht. Drück auf jedem Bildschirm "?" für seine eigene Hilfeseite.

Beim ersten Öffnen des Studios erscheint eine **Willkommen**-Tour: eine Karte unten rechts, die dich durch jeden Bildschirm führt, mit dem echten Bildschirm dahinter und seinem hervorgehobenen Knopf in der Seitenleiste, dazu das, was das Studio schon über deine Installation weiß (die gefundenen Ordner, deine Profile, die Probleme in deiner Liste). Weiter geht es mit **Weiter** oder den Pfeiltasten; das X schließt sie. Um sie noch einmal zu sehen, öffne die Hilfe ("?") und klick auf **Willkommenstour erneut zeigen**.

## Die Bildschirme

Die Seitenleiste listet die Bildschirme unter **Studio-Module**. Bildschirme mit einem Schloss brauchen zuerst ein aktives Profil: Erstelle oder aktiviere eins unter Profile.

- **Profile**: benannte Mod-Listen; erstelle sie, wechsle mit einem Klick zwischen ihnen, teile sie als `.pzpack`-Dateien.
- **Spielstände**: deine Spielstände und ihre Backups; mach eins, behalte eins oder stell eins wieder her.
- **Monitor**: starte das Spiel und lies sein Log, mit einer Karte pro Fehler und der Mod, von der er kam.
- **Mod-Liste**: schalte Mods an und aus und lege die Ladereihenfolge fest, mit Filtern, Gruppen, Notizen und Rückgängig.
- **Neuigkeiten**: deine Workshop-Mods, die seit deinem letzten Spiel aktualisiert wurden, mit ihren Änderungshinweisen und einer Warnung, wenn ein Update Spielstände kaputt machen kann.
- **Diagnose**: was die App über deine Mods weiß (so geprüft, wie das Spiel sie liest: fehlende oder deaktivierte Voraussetzungen, falsche Reihenfolge, inkompatible Mods, Mods, die dein Build nicht laden kann, mit Lösungen), über das Spiel (Arbeitsspeicher, CPU, Start, Frames, Festplatte) und über den Server auf deinem PC, mit dem Doktor auf der Startseite.
- **Mods finden**: durchsuche den Steam Workshop, abonniere, führe eigene Listen und installiere Mods, die du woanders heruntergeladen hast.
- **Karten**: die Zellen jeder Karten-Mod über der Welt des Spiels, und wo zwei Karten-Mods um dieselben Zellen streiten.
- **Mod-Merger**: wenn zwei Mods dieselbe Datei ändern, kombiniere sie zu einem Patch-Paket, das zuletzt lädt.
- **Entwickler**: für Mod-Macher: prüfe die Lua- und Skriptdateien einer Mod, bevor das Spiel es tut, und starte das Spiel nur mit dieser Mod. Erscheint nur, wenn **Entwicklerfunktionen** in den App-Einstellungen an ist.
- **Server**: hoste und verwalte einen dedizierten Server und halte seine Mods im Einklang mit deinen.
- **App-Einstellungen**: wo dein Spiel und deine Mods liegen, welche Warnungen du siehst, und weitere Optionen.

## Das Spiel starten

Der grüne Knopf oben rechts startet Project Zomboid mit deiner Mod-Liste. Der Pfeil daneben öffnet das Menü:

- **Startmodi**: mit Debugging (`-debug`, der Monitor verfolgt das Log live) oder das normale Spiel, jeweils im Vollbild oder im Fenstermodus.
- **Wohin**: **Hauptmenü**, wie immer, oder **Eigenes Ziel…**, das **Eigener Start** öffnet, um direkt irgendwohin zu gehen:
  - **Ein Spielstand**: deine Einzelspieler-Spielstände, die des aktiven Profils zuerst. Das Spiel öffnet sich und im Hauptmenü lädt die Mod Studio Link diesen Spielstand, mit denselben Hinweisen und demselben Backup wie bei **Fortsetzen**. Danach kommt der schwarze Bildschirm "Zum Starten klicken" des Spiels.
  - **Ein Server**: deine gespeicherten Server. Tritt bei wie **Beitreten** unter Server, mit dem gespeicherten Passwort und deinem Benutzernamen; das Spiel lädt die Mods des Servers.
  - **Wie** wählt den Startmodus. **Wählen** legt das Ziel für den Knopf fest; **Wählen und starten** startet das Spiel auch gleich. **Zurück zum Hauptmenü** setzt es zurück.

Ist ein Ziel gewählt, nennt der Knopf es (**Spielen: (Spielstand)** oder **Beitreten: (Server)**) und merkt es sich, wie den Modus, solange **Merken, wie das Spiel startet** in den Einstellungen an ist.

## Die Mod Studio Link im Spiel

PZ Mod Studio Link (`Z_StudioLink`) ist eine kleine Mod für Build 42, die das Studio installiert und in jedem Profil an lässt. Sie ist die Seite der App im Spiel. Einiges davon funktioniert allein; der Rest braucht die App.

### Ihr eigener Mods-Bildschirm

Der Knopf **MODS** im Hauptmenü öffnet den Mods-Bildschirm des Links statt dem des Spiels. Allein bietet er:

- jede installierte Mod mit einem Suchfeld (Name, Mod-ID, Workshop-ID, Autor) und Filtern (**Alle**, **An**, **Aus**, **Favoriten**, **Aktualisiert**, **Probleme**, **Karten**);
- die Details jeder Mod: was sie benötigt und welche Mods sie brauchen, ihre Builds, ihre Workshop-Seite;
- Mods an- und ausschalten, zusammen mit dem, was sie benötigen, mehrere auf einmal mit Strg oder Umschalt;
- die Ladereihenfolge: danach sortiert, nach oben und unten verschoben oder gezogen, nie über das, was eine Mod benötigt;
- Gruppen nach Autor oder Kategorie, die Optionen jeder Mod (**Mod-Optionen**), die **Kartenreihenfolge** und **Rückgängig** / **Wiederholen**;
- **Geteilte Dateien**: Wenn zwei aktive Mods dieselbe Lua-Datei mitbringen, führt das Spiel nur eine Kopie aus, die der Mod, die später lädt. Die Mod, deren Kopie genutzt wird, ist mit **Überschreibt** markiert, die, deren Kopie nicht genutzt wird, mit **Überschrieben**; der Übersichtsknopf **N teilen Dateien** listet sie auf (sein Tooltip erklärt die Markierungen), und die Details jeder Mod nennen die Dateien und die andere Mod. Geprüft wird gegen die Mods, die das Spiel geladen hat, ein paar Sekunden nachdem das Hauptmenü erscheint; nur Lua-Dateien werden angesehen;
- **Mods neu laden** lässt das Spiel seine Mod-Ordner neu einlesen, sodass ein Workshop-Artikel, den du bei laufendem Spiel abonniert hast (oder der sich aktualisiert hat), ohne Neustart erscheint; das Spiel lädt sein Lua neu, wie nach **Übernehmen**;
- **Server-Zeilen** kopiert die aktiven Mods als Zeilen `Mods=`, `WorkshopItems=` und `Map=` für einen dedizierten Server, in Ladereihenfolge (jede Mod nach dem, was sie benötigt, die Karten in der Kartenreihenfolge), zum Einfügen über diese Zeilen der `.ini` des Servers in `Zomboid/Server`; Mods ohne Workshop-Artikel werden in einer Kommentarzeile genannt, zum Kopieren in den eigenen Mods-Ordner des Servers;
- **Übernehmen** speichert in die Liste, die das Spiel liest, wie es der eigene Bildschirm des Spiels tut. **Bildschirm des Spiels** öffnet den Originalbildschirm.

Mit der App zeigt derselbe Bildschirm außerdem nur die Mods des Profils (**Mods des Profils** / **Alle installierten**), die Warnungen der Diagnose an jeder Mod, Mods, die seit deiner letzten Sitzung aktualisiert wurden (in ihren Details die neuesten Änderungshinweise ihres Autors und ob das Update einen Spielstand kaputt machen kann), deine Notizen und Gruppen und einen Knopf, um neue Mods hinzuzufügen, die in keinem Profil sind. **Mit PZ Mod Studio abgleichen** holt, was die App geändert hat, und **PZ Mod Studio öffnen** holt die App nach vorne.

Ist ein anderer Mod-Manager an, der den Mods-Bildschirm selbst übernimmt, fragt der Link, welchen Bildschirm du willst, und überlässt ihn dem anderen, wenn du das sagst.

### Die Karte im Hauptmenü

Wenn die App auf diesem PC gelaufen ist, zeigt eine Karte im Hauptmenü, ob die App offen oder geschlossen ist, das aktive Profil und seine Anzahl an Mods, die Fehler und Warnungen der Diagnose, und **Fortsetzen: (Spielstand)** mit dem, was sich seit seiner letzten Sitzung an seinen Mods geändert hat. Wurde die App nie installiert, lädt dich die Karte stattdessen ein, sie zu holen, mit **Herunterladen** (kopiert die Adresse der Download-Seite) und **Ausblenden**.

### Der Fehlerhinweis

Wenn das Spiel einen Lua-Fehler aufzeichnet, nennt ein Hinweis die Mod, von der er kam (und welche Mod sie aufgerufen hat), mit der letzten Meldung. **Bericht kopieren** legt den Fehler in die Zwischenablage, um ihn an den Autor der Mod zu schicken, **In Mods zeigen** öffnet den Mods-Bildschirm bei dieser Mod, und **Im Studio ansehen** öffnet den Monitor bei ihrer Karte. Der Hinweis bleibt still, wenn eine andere Mod an ist, die diese Fehler schon zeigt. **Alle (N)** und **Lua-Fehler (N)** im Pausenmenü öffnen jeden Fehler der Sitzung, jede Art einmal mit ihrer Anzahl, ihrer Mod und Datei:Zeile: Ein Klick kopiert einen, **Alle kopieren** kopiert alle mit dem Build des Spiels, dem Modus und wie viele Mods an sind. Ein Bericht nennt jede Mod mit ihrer ID, Version und ihrem Workshop-Artikel.

Jeder Fehler landet außerdem innerhalb einer Sekunde in `Zomboid/Lua/PZModStudio_LinkErrorLog.txt`, sodass ein Absturz, ein Verbindungsabbruch oder die Rückkehr ins Menü ihn nicht verliert: jede Art einmal vollständig (Mod, Datei:Zeile, Meldung, Stack) und ihre Wiederholungen gezählt, unter einer Überschrift mit Datum, Build und ob es das Menü oder welcher Spielstand war, mit einer Zeile, wo das Spiel ins Hauptmenü zurückging oder der Server es getrennt hat. Die Datei behält etwa die letzten 3000 Zeilen. **Fehlerlog** in der Liste öffnet ihren Ordner und kopiert ihren Pfad.

Für Mod-Autoren: `PZModStudioLink.registerReport("YourModId", fn)` (oder `PZModStudioLinkReports["YourModId"] = fn` aus Lua, das vor dem des Links läuft) fügt den Berichten deine eigenen Infos hinzu. `fn(err)` gibt einen String, eine Liste von Zeilen oder eine Tabelle von Name = Wert-Paaren zurück; es läuft nur, wenn ein Bericht erstellt wird, innerhalb von `pcall`. Sein Text kommt in **Alle kopieren**, und bei einem Fehler deiner Mod in **Bericht kopieren** und ins Fehlerlog. Die Debug-Berichte anderer Mods, die Mods schon für Fehler-Tools registrieren, landen an denselben Stellen unter dem Namen, den jede Mod ihnen gegeben hat.

Den roten **ERROR**-Zähler des Spiels unten rechts zeichnet das Java des Spiels, an das Lua nicht herankommt: Der Link kann ihn nicht ausblenden.

### Einem Server beitreten

Wenn das Spiel die Verbindung trennt, weil eine Workshop-Mod von der des Servers abweicht, nennt der Link die Mod (das Spiel gibt nur eine Nummer an), sagt, was in diesem Fall zu tun ist (der Server aktualisiert beim Neustart, oder Steam muss die neue Version herunterladen) und bietet **Workshop-Links kopieren** an. Weicht der Build des Spiels von dem des Servers ab, sagt er das mit beiden Builds.

Beim Hosten (**SERVER ERSTELLEN**, **Starten**) lädt das Spiel die eigene Mod-Liste dieser Servereinstellungen, nicht die des Mods-Bildschirms. Fehlen dieser Liste Mods, die du an hast, oder nennt sie Mods, die dieser PC nicht hat, fragt der Link zuerst: **Ja** hostet trotzdem, **Nein** geht zurück (unter **Servereinstellungen** änderst du seine Mods).

### Funktionen, die zwei Mods ändern

Wenn die Diagnose sieht, dass zwei Mods dieselbe Funktion neu definieren, fragt der Link das Spiel, ein paar Sekunden nach dem Hauptmenü und nachdem ein Spielstand geladen ist, aus welcher Datei die Funktion stammt, die geblieben ist. Die Diagnose zeigt das als **Im laufenden Spiel gesehen**.

### Mit und ohne die App

- Ohne die App: der Mods-Bildschirm des Links (mit **Server-Zeilen**), der Fehlerhinweis und sein Fehlerlog (ohne **Im Studio ansehen**) und die Einladungskarte.
- Mit aktivem Profil, auch wenn die App geschlossen ist: Der Mods-Bildschirm zeigt das Profil, wie die App es zuletzt hinterlassen hat; auf dem Ladebildschirm ist jeder Spielstand danach markiert, ob seine Mods zum Profil passen; ein Spielstand, der Mods außerhalb des Profils nutzt, lädt nicht; ein Spielstand mit leerer Mod-Liste fragt zuerst.
- Nur während die App offen ist: Backups eines Spielstands, bevor er mit geänderten Mods lädt und wenn du ihn verlässt, Spielen auf einer Kopie, neue Mods zum Profil hinzufügen, **Abgleichen**, **PZ Mod Studio öffnen** und **Im Studio ansehen**.
- Notizen, Gruppen, Profile und die Diagnose entstehen in der App; das Spiel zeigt sie nur.
