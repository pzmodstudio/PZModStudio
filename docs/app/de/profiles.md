# Profile

Ein Profil ist eine benannte Mod-Liste: welche Mods dazugehören, welche davon an sind und in welcher Ladereihenfolge. Hier erstellst du Profile, wechselst mit einem Klick zwischen ihnen, teilst und löschst sie. Die meisten anderen Bildschirme bleiben gesperrt, bis ein Profil aktiv ist.

## Was du siehst

- Oben, wie viele Profile du gespeichert hast, und der Knopf **Neues Profil**, der ein Fenster öffnet, das dich in drei Schritten durch das Erstellen führt.
- Eine Karte pro gespeichertem Profil, das aktive zuerst. Das aktive hat einen grünen Rahmen, das Etikett **Aktiv** und den Knopf **Mods ansehen**, der die Mod-Liste öffnet; die anderen zeigen **Inaktiv** und haben **Aktivieren (1 Klick)**.
- Jede Karte zeigt ihre Beschreibung, wie viele Mods an sind (**Aktive Mods**), die **Gesamtzahl** der Mods in ihrer Reihenfolge und den Build: **Build 41/42**, wenn du ihn von Hand festgelegt hast, oder **Gespielt auf ...** mit dem Build, auf dem es zuletzt aus dem Studio gespielt wurde.

## Ein Profil erstellen

Drück **Neues Profil**: Ein Fenster mit drei Schritten öffnet sich (**Name**, **Welche Mods**, **Prüfen**), jeder mit einer Erklärung oben. **Weiter** und **Zurück** wechseln zwischen ihnen; Esc oder das X schließt es, ohne etwas zu erstellen.

1. Gib einen Namen unter **Profilname** ein (Pflicht) und, wenn du willst, eine **Beschreibung (optional)**.
2. Unter **Welche Mods** wählst du, womit das Profil startet:
   - **Was jetzt an ist**: die Mods, die gerade an sind.
   - **Alles Installierte**: jede Mod, die das Studio gefunden hat.
   - **Selbst auswählen**: eine Liste mit Suchfeld und den Knöpfen **Alle** / **Keine**; hak die Mods an, die du willst.
   - **Noch nichts**: ein leeres Profil. Das Studio fragt nach einer Bestätigung, weil der Wechsel darauf jede Mod ausschaltet. Solange ein Profil aktiv ist, bringt dich ein Knopf **Mehr Mods unter „Mods finden“** dorthin, um zuerst zu abonnieren.
3. Unter **Prüfen** siehst du den Namen und wie viele Mods angeschaltet starten. Drück **Erstellen und einschalten**.

Das neue Profil wird sofort erstellt und aktiviert und behält die Ladereihenfolge, die deine Liste schon hat. Hat das Studio noch keine Mods gefunden (Spiel- oder Zomboid-Ordner in den App-Einstellungen nicht gesetzt), lehnt es ab, damit ein Profil nicht aus Versehen alle deine Mods ausschaltet.

## Profile wechseln

Drück **Aktivieren (1 Klick)** auf einer Karte. Das Studio dann:

- schreibt die Mods dieses Profils in die Liste, die das Spiel liest (`Zomboid/mods/default.txt`);
- markiert es als aktiv und öffnet die Mod-Liste mit seiner Reihenfolge;
- installiert die Mod PZ Mod Studio Link und lässt sie in jedem Profil an, damit der Mods-Bildschirm des Spiels das Profil zeigt.

Braucht eine Mod des Profils einen Java-Agenten in der Startdatei des Spiels (zum Beispiel ZombieBuddy), zeigt ein Fenster zuerst, was aus dieser Datei herausgenommen oder wieder hineingeschrieben wird. Drück **Einschalten ...**, um weiterzumachen. Läuft das Spiel, wartet dieser Teil, bis das Spiel geschlossen ist. Vorher wird eine Kopie der Startdatei angelegt.

## Profil abtrennen

**Profil abtrennen** hält für dieses Profil getrennt:

- seine Einzelspieler-**Spielstände**;
- die **Optionen des Spiels**: Grafik, Ton und Tasten;
- die **Einstellungen der Mods**: was jede Mod im Zomboid-Ordner speichert (ihre Optionen, ihre Einstellungen, ihre Daten).

Mit diesem Profil nutzt das Spiel nur seine eigenen, mit einem anderen Profil dessen eigene (oder die gemeinsamen, wenn das andere nicht abgetrennt ist). Beim Abtrennen wählst du, welche gemeinsamen Spielstände es mitnimmt; der Rest bleibt gemeinsam. Die Optionen und die Einstellungen der Mods starten als **Kopie** der jetzigen, sodass das Profil genauso startet wie vorher und die anderen Profile ihre behalten.

Die **Mods selbst sind für jedes Profil dieselben**: Sie werden nicht kopiert oder verschoben, und es gibt nichts neu zu installieren. Eine Mod (oder ein Teil einer Mod), die in einem Profil an und in einem anderen aus ist, funktioniert wie immer. Die Spielstände von Servern (`Saves/Multiplayer`), die Logs des Spiels, Server und die Dateien der App und des Links werden auch nicht getrennt.

Große Daten, die keine Einstellung sind, werden beim Abtrennen nicht kopiert: Eine einzelne Datei über 20 MB oder ein Ordner über 50 MB (der Cache einer Mod, Aufnahmen oder Dumps) bleibt **gemeinsam**, ab dann für jedes Profil gleich, damit Abtrennen nie die Festplatte füllt. Die eigenen Optionen und Tasten des Spiels werden immer kopiert, egal wie groß. Was gemeinsam geblieben ist, steht in `Zomboid/PZModStudio_ProfileSavesShared.json`.

Beim Profilwechsel legt die App alles durch Umbenennen an seinen Platz, nichts wird kopiert. Das tut sie nur, während Project Zomboid geschlossen ist: das Spiel und auch jeder Project-Zomboid-Server auf diesem PC (der Server des Studios aus dem Bildschirm Server oder einer, der von Hand gestartet wurde). Läuft einer, sagt die App welcher und bittet dich, ihn zuerst zu schließen. Was zu den anderen Profilen gehört, wartet in `Zomboid/PZModStudio_ProfileSaves`. Geht der PC mitten in einem Wechsel aus, beendet die App ihn beim nächsten Öffnen.

Jeder Spielstand und jede Einstellung, die die App verschiebt, kopiert oder in den Backups aufbewahrt, steht im Log des Studios (`Zomboid/PZModStudio_Logs/studio.log`, Zeilen mit `[Profiles]`), von wo nach wo, sodass du nachsehen kannst, wohin ein scheinbar fehlender Spielstand gegangen ist.

**Nicht mehr abtrennen** legt seine Spielstände zu den gemeinsamen und holt die gemeinsamen Optionen und Mod-Einstellungen zurück; die eigenen des Profils werden in `Zomboid/PZModStudio_Backups/ProfileSettings` aufbewahrt, nichts wird überschrieben. Das Löschen des Profils tut dasselbe, sodass nie ein Spielstand oder eine Einstellung verloren geht.

## Bearbeiten, teilen und löschen

- Der Stift-Knopf (**Dieses Profil umbenennen**) öffnet **Mod-Profil bearbeiten**: Ändere den Namen, die Beschreibung und den **Spiel-Build**. Lass den Build auf **Wie zuletzt gespielt** oder setze **Build 41** / **Build 42**. Ist ein Build gesetzt, fragt das Studio, bevor es dieses Profil auf einem anderen Build startet, auch bevor es gespielt wurde. Drück **Änderungen speichern**.
- Der Teilen-Knopf speichert das Profil in eine `.pzpack`-Datei, die du jedem schicken kannst. Sie enthält nur die Mods, die an sind, in Reihenfolge, mit ihren Workshop-IDs. Wer sie öffnet, kann abonnieren, was ihm fehlt.
- Der Papierkorb-Knopf löscht das Profil nach einer Bestätigung. Werden einige seiner Workshop-Artikel von keinem anderen Profil genutzt, ist die Bestätigung ein Fenster, das sie auflistet (**Nur dieses Profil nutzt**), angehakt, mit **Alle ankreuzen** / **Keine ankreuzen**. **Löschen und deabonnieren** löscht das Profil und deabonniert die angehakten Artikel (Steam muss laufen); **Nur Profil löschen** behält sie. Artikel, die ein anderes Profil nutzt, und lokale Mods werden nie angerührt. Nutzt das Profil Patch-Pakete aus dem Mod-Merger (`Z_PZModStudio_...`), fragt eine zweite Frage, ob sie auch gelöscht werden sollen; **Abbrechen** behält sie.

## Gut zu wissen

- Ein gelöschtes Profil ist nicht verloren: Seine Datei wird nach `Zomboid/PZModStudio_Backups/profiles` verschoben.
- Wird das aktive Profil gelöscht, ist kein Profil mehr aktiv, und der Mods-Bildschirm des Spiels zeigt kein Profil mehr.
- Änderungen in der Mod-Liste (Mods schalten, umsortieren) werden sofort im aktiven Profil gespeichert.
- Im eigenen Mods-Bildschirm des Spiels übernommene Änderungen kommen auch ins aktive Profil zurück.
- Eine angeschaltete Mod wird zu einer Mod des Profils. Mods, die zu keinem Profil gehören, bietet die Mod-Liste mit **Zum Profil hinzufügen** an.
- Spielstände wechseln nie zwischen den großen Builds (41 und 42), deshalb merkt sich ein Profil seinen Build.
