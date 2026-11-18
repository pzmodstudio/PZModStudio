# App-Einstellungen

In den App-Einstellungen sagst du dem Studio, wo dein Spiel und deine Mods liegen, und wählst, wie sich das Studio verhält: Sprache, Fenster, Warnungen, wie es startet, und ein paar optionale Extras. Jede Änderung wird gespeichert, sobald du sie machst.

## Einfach und Vollständig

Die Karte oben, **Wie viel vom Studio angezeigt wird**, wählt eine von zwei Ansichten. Derselbe Schalter ist immer unten in der linken Leiste, unter **Ansicht**. Ein Wechsel löscht oder deaktiviert nie etwas: Er zeigt oder versteckt nur Bildschirme, und du kannst jederzeit hin und her wechseln.

- **Einfach** zeigt, was du zum Spielen mit Mods brauchst:
  - **Start**: das Profil in Gebrauch, **Spielen**, **Fortsetzen** (dein letztes Spiel mit diesem Profil, sobald du eins hast) und die Prüfung in einer Zeile: grün, wenn nichts zu tun ist, rot mit **Sehen, was zu tun ist**, wenn zuerst etwas repariert werden muss.
  - **Profile**, **Mod-Liste**, **Mods finden**, **Spielstände** und **Diagnose**.
  - Die **Mod-Liste** lässt dich Mods an- und ausschalten, sie ordnen (von Hand oder mit **Auto-Sortierung**), suchen und nach an, aus und mit Problem filtern. Jede Mod wird rot und in einfachen Worten markiert, nur für das, was repariert werden muss, mit ihrem Knopf daneben. Gruppen, Notizen, Voreinstellungen, die anderen Filter und Ladezeiten bleiben in Vollständig.
  - **Diagnose** öffnet direkt bei dem, wo du handeln musst, jedes mit einem Knopf. Optionale Vorschläge (das Spiel funktioniert ohne sie) und was nur gut zu wissen oder schon behoben ist, werden unter der Liste gezählt, nicht gezeigt; **Alles zeigen (Vollständig)** zeigt sie. **N Problem(e) automatisch beheben** zählt und behebt nur die Zeilen der Liste, die du siehst: Die versteckten Vorschläge bleiben in Ruhe.
  - Hier behalten die Einstellungen die Reiter **Ordnerpfade**, **Hinweise** und **Die App**.
- **Vollständig** ist das ganze Studio: jeder Bildschirm in der Seitenleiste (Monitor, Neuigkeiten, Karten, Mod-Merger, Server, Entwickler), jeder Reiter der Diagnose und jeder Reiter der Einstellungen.

Eine neue Installation startet in **Einfach**, mit einer Begrüßung in drei Schritten: wo dein Spiel gefunden wurde, ein Profil erstellen oder wählen, spielen. Wurde das Studio auf diesem PC schon benutzt, startet es in **Vollständig**, wie es immer aussah, und beim ersten Mal bietet eine Zeile oben an, Einfach auszuprobieren: **Einfach ausprobieren** oder **Nein, danke**. Sie erscheint einmal, und nie, wenn du schon eine Ansicht gewählt hast. Die Wahl wird auf diesem PC gemerkt.

Ein Link von einem anderen Bildschirm kann trotzdem einen Bildschirm öffnen, den Einfach versteckt (zum Beispiel die Schritte des Doktors); er erscheint in der Seitenleiste, solange er offen ist.

## Was du siehst

Der Bildschirm hat oben Reiter nebeneinander:

- **Ordnerpfade**: wo das Spiel, der Workshop, dein Zomboid-Ordner und die Mod-Liste sind.
- **Hinweise**: welche Warnungen das Studio dir zeigt.
- **Die App**: wie das Studio startet, sein Fenster und Schließen-Knopf, die Sprache, das Merken, wie das Spiel startet, der Mods-Bildschirm des Spiels, die Suche nach neuen Versionen und die Übersetzung.
- **Erweitert**: die Diagnose der App, die Entwicklerfunktionen und damit die Java-Mods, die du in ZombieBuddy erlaubt hast.
- **Build-42-Korrekturen**: was der Kompatibilitäts-Shim für Build 42 abdeckt, mit einem Schalter pro Mod, die er patcht.
- **MCP-Agent-Server**: nur sichtbar mit **Entwicklerfunktionen** an.

## Ordner

Drück **Pfade automatisch finden**, damit das Studio alles selbst findet. Um einen Ordner von Hand festzulegen, drück **Durchsuchen...** daneben. Ein Hinweis **Gespeichert.** bestätigt die Änderung.

- **Installationsordner von Project Zomboid**: der Ordner mit `ProjectZomboid64.exe`. Er zeigt **Spiel gefunden**, wenn der Ordner stimmt, und **Das Studio findet das Spiel hier nicht**, wenn nicht. Installationen über Steam, GOG und von Hand funktionieren alle.
- **Steam-Workshop-Ordner (optional ohne Steam)**: wohin Steam Workshop-Mods herunterlädt (`workshop/content/108600`). Lass ihn leer, wenn du über GOG spielst oder Mods von Hand in `Zomboid/mods` legst.
- **Zomboid-Benutzerordner (Daten und lokale Mods)**: dein Ordner `C:\Users\<User>\Zomboid`, mit Logs, Spielständen, Profilen und lokalen Mods. Er ist Pflicht.
- **Mod-Listendatei (Zomboid/mods/default.txt)**: die Datei, die das Spiel liest, um zu wissen, welche Mods an sind und in welcher Reihenfolge. Sie wird automatisch gefunden; **Durchsuchen...** wählt hier eine Datei, keinen Ordner.

Ohne gültigen Spielordner bitten dich Bildschirme wie der Mod-Merger, zuerst hierherzukommen.

## Hinweise

Unter **Systemmeldungen und Warnungen** kannst du drei Meldungen stummschalten oder zurückholen:

- **Warnung beim Start wegen nicht gemergter Mod-Konflikte**: die Warnung vor dem Start, wenn sich deine aktiven Mods Dateien teilen, die kein aktives Merge-Paket zusammenführt (oder die es aus älteren Versionen der Mods zusammenführt).
- **Hinweis beim Start zu Mods, die dasselbe definieren**: der Hinweis vor dem Start, wenn zwei aktive Mods denselben Gegenstand, dasselbe Rezept oder denselben Text unterschiedlich definieren und kein Paket entscheidet, was bleibt. Nichts geht kaputt: Die zuletzt ladende gewinnt.
- **Erklärung zur Auto-Sortierung der Abhängigkeiten**: die Erklärung, die beim Sortieren der Abhängigkeiten in der Mod-Liste erscheint.

Jede hat **Warnung ausblenden** / **Warnung wieder anzeigen** (oder **Hinweis ausblenden** / **Hinweis wieder anzeigen**).

## Wie das Studio startet und sein Fenster

- **Was soll sich beim Start des Studios öffnen?**: **Profile-Bildschirm (Start)** (empfohlen) zeigt die Profilverwaltung; **Letztes Profil direkt öffnen** lädt das zuletzt aktive Profil und öffnet die Mod-Liste.
- **Fenster so öffnen, wie ich es verlassen habe**: gleiche Größe, gleicher Ort und Monitor, maximiert oder im Vollbild, wenn es so war. Ist dieser Monitor weg, öffnet es sich auf dem Hauptmonitor.
- **Die Schließen-Schaltfläche (X)**: **Schließt das Studio** beendet es (das Spiel läuft weiter, wenn es offen ist). **Behält es im Infobereich neben der Uhr** versteckt das Fenster, und das Studio beobachtet weiter das Spiel (Backups beim Verlassen, das Profil im Mods-Bildschirm des Spiels). Klick auf das Symbol im Infobereich, um es wieder zu öffnen; sein Menü hat **Beenden**.

## Sprache

Wähle unter **Sprache** die Sprache der App: English, Español, Português (Brasil), Français, Deutsch, Русский oder 简体中文. Eine Zeile, die in einer Sprache fehlt, erscheint auf Englisch. Die eigenen Meldungen des Spiels (Fehler, Warnungen, die Log-Zeilen im Monitor) bleiben so, wie das Spiel sie schreibt.

## Spielstart und der Mods-Bildschirm des Spiels

- **Merken, wie das Spiel startet**: an, startet der Start-Knopf in dem Modus, den du zuletzt genutzt hast (Debug, Fenstermodus, Normal...). Aus, startet er immer im Debug-Modus (Vollbild), dem empfohlenen.
- **Der Mods-Bildschirm des Spiels, wenn ein anderer Mod-Manager an ist**: Nur ein Mod-Manager kann den Mods-Bildschirm des Spiels zeichnen. Ist ein anderer an, wähle **Im Spiel fragen**, **Den von PZ Mod Studio (fragt erneut, wenn der andere eingeschaltet wird)** oder **Den des anderen Managers**. Spielstand-Backups und die Prüfung vor dem Laden eines Spielstands bleiben in jedem Fall bei PZ Mod Studio.

## App-Updates

- **Nach neuen Versionen von PZ Mod Studio suchen**: Beim Start und alle 6 Stunden, solange es offen ist, fragt das Studio GitHub, welches das neueste Release ist. Gibt es ein neueres, sagt das eine Leiste unter der Kopfzeile. **Jetzt aktualisieren** lädt die neue Version herunter, prüft, dass das Projekt sie signiert hat (die Signatur von `SHA256SUMS.txt` und der SHA-256 des Zips), setzt sie an die Stelle deiner und startet das Studio neu; passt etwas nicht, ändert sich nichts. **Herunterladen** öffnet die Release-Seite in deinem Browser, und **Nicht jetzt** blendet den Hinweis bis zur nächsten Version aus. Ist das Studio in einem Ordner installiert, in den es nicht schreiben kann (Program Files, für alle Benutzer), aktualisiere es mit dem Installer von der Seite. Nichts wird installiert, ohne dass du fragst, und nichts wird gesendet. **Jetzt prüfen** fragt sofort. Antwortet GitHub nicht, fragt das Studio stattdessen seine itch.io-Seite; dann bietet der Hinweis nur die Download-Seite an, nicht **Jetzt aktualisieren**. Aus, wird GitHub nicht gefragt.

## Frühere Versionen von Mods

- **Frühere Versionen von Mods behalten** (anfangs an): Steam behält keine alten Versionen einer Workshop-Mod. Das Studio behält eine Kopie der installierten Version jeder Workshop-Mod, die ein Profil nutzt, in `Zomboid/PZModStudio_Versions`, bevor Steam sie ersetzt; **Versionen…** in der Mod-Liste holt eine zurück. **Bis zu** legt fest, wie viel Platz sie belegen darf (anfangs 10 GB; ein Limit, das du gewählt hast, bleibt, wie du es gewählt hast); jede Datei wird einmal gespeichert, also kostet eine Version nach einem Update nur das, was sich geändert hat. Über dem Limit fallen zuerst die ältesten früheren Versionen weg, dann Kopien von Mods, die nicht im aktiven Profil sind; von Hand behaltene Versionen bleiben. Egal wie das Limit ist, das Studio füllt nie das Laufwerk: Eine Runde lässt immer 15 % der Laufwerksgröße frei, und wenn genau das den Speicher bremst, sagt die Karte **Begrenzt durch freien Platz auf D:** (mit dem Limit, das gerade gilt). **Ordner** zeigt, wo die Kopien liegen und wie viel Platz sie belegen; **Ordner wählen…** verschiebt sie, zum Beispiel auf ein anderes Laufwerk: Jede Datei wird kopiert und gegen das Original geprüft, und erst dann wird der alte Ordner entfernt (auf demselben Laufwerk wird er nur umbenannt). Ein Ordner, der schon andere Dateien enthält, bekommt einen Ordner `PZModStudio_Versions` darin. Das Verschieben wird abgelehnt, solange das Spiel oder eine Runde läuft, und für das Stammverzeichnis eines Laufwerks, den Workshop-Ordner von Steam, den Ordner des Spiels, `Zomboid/mods` und `Zomboid/Saves`. Die Zeile darunter sagt, wie viel belegt ist und wie viele deiner Workshop-Mods abgedeckt sind; Mods, die aus Platzmangel ohne Kopie geblieben sind, werden genannt. **Jetzt speichern** startet sofort eine Runde (sie läuft auch von selbst kurz nach dem Start des Studios und immer, wenn Steam etwas aktualisiert). Solange das Spiel läuft, passiert nichts, und der Workshop-Ordner wird nur gelesen.

## Diagnose

**Diagnose** sammelt, was ein Fehlerbericht braucht: die Versionen des Studios und von Windows, die Ordner, den Build des Spiels, das aktive Profil, den Studio Link, ZombieBuddy, ob Steam Anfragen begrenzt, und das Ende des Studio-Logs. **Diagnose kopieren** kopiert sie; dein Windows-Benutzername und dein PC-Name werden durch `<user>` und `<pc>` ersetzt. Sieh es dir an, bevor du es irgendwo einfügst. **Log-Ordner öffnen** öffnet den Ordner mit den eigenen Logs des Studios.

## Übersetzung

Mods finden hat einen Übersetzen-Knopf auf der Seite einer Mod und einen für die Zusammenfassungen einer Ergebnisseite; die Übersetzungsentwürfe im Bildschirm Entwickler nutzen denselben Dienst. Ohne Schlüssel wird der kostenlose Dienst Google Translate genutzt (er ist inoffiziell und kann eines Tages aufhören). Mit einem DeepL-API-Schlüssel (kostenlos bis 500.000 Zeichen im Monat) wird zuerst DeepL genutzt: Drück **DeepL-Schlüssel holen**, füge ihn ein und drück **Prüfen und speichern**. Der Schlüssel wird nur auf diesem PC in der Windows-Anmeldeinformationsverwaltung gespeichert; **Entfernen** löscht ihn. Schon gemachte Übersetzungen bleiben auf diesem PC, damit ein Text nicht noch einmal gesendet wird; **Löschen** leert sie.

## Entwicklerfunktionen

**Entwicklerfunktionen** zeigt den Bildschirm Entwickler in der Seitenleiste, den Reiter **MCP-Agent-Server** (mit dem ein KI-Assistent das Studio steuern kann) und die Begleit-Mod, die Befehle im Spiel ausführt. Als Spieler brauchst du das nicht; lass es aus, außer du entwickelst damit. Die Wahl wird auf diesem PC gemerkt.

## Build-42-Korrekturen

Dieser Reiter erklärt den Kompatibilitäts-Shim für Build 42: eine Datei, die das Studio in jedes Merge-Paket schreibt, das es baut, und die Aufrufe patcht, die Mods für Build 41 an Dinge machen, die Build 42 umbenannt oder entfernt hat. Die Teile 1 bis 6 gelten immer, solange ein Merge-Paket aktiv ist. Teil 7 patcht bestimmte Mods, jede mit einem Schalter **AN** / **AUS**. Ein Schalter wird sofort in jedes Studio-Paket auf der Festplatte geschrieben; es ist kein Veröffentlichen nötig, und das Spiel liest ihn beim nächsten Start.

## MCP-Agent-Server

Nur mit **Entwicklerfunktionen** an. Wähle deinen KI-Client, drück **MCP-JSON kopieren** und füge es in die Konfigurationsdatei dieses Clients ein; Schritt-für-Schritt-Hinweise für jeden Client stehen darunter. Starte `pz-mcp-server.exe` nicht per Doppelklick: Ein leeres schwarzes Fenster ist normal, und dein KI-Client startet und stoppt ihn von selbst.

## Gut zu wissen

- Die Wahl von Ordnern, Sprache, Fenster und Warnungen gilt sofort; nichts braucht einen Neustart außer den Shim-Schaltern, die das Spiel beim nächsten Start liest.

## Unter Linux

Die Linux-Version (AppImage oder .deb) findet Steam in den üblichen Ordnern (`~/.local/share/Steam`, `~/.steam`, dem des Flatpaks) und den Zomboid-Ordner in deinem Home-Ordner. Manches geht nur unter Windows: die App an Ort und Stelle aktualisieren (unter Linux öffnet **Herunterladen** die Release-Seite), das Spielfenster für einen KI-Assistenten verschieben und aufnehmen (parken, Screenshots, Klicks, Tasten), das Speichermangel-Protokoll von Windows, Abonnieren über den Steam-Client und SteamCMD herunterladen (installiere es aus deiner Distribution). Gespeicherte Schlüssel und Passwörter liegen in einer privaten Datei, `~/.config/PZModStudio/secrets.json`, statt in einem Anmeldespeicher. Für einen KI-Client ist der Konsolenserver `/usr/bin/pz-mcp-server` mit dem .deb; das AppImage stellt MCP selbst mit `--mcp` bereit.
