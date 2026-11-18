# Spielstände

Deine Spielstände aus Project Zomboid und ihre Backups. Das Studio sichert einen Spielstand von selbst in den Momenten, in denen er am meisten gefährdet ist; hier siehst du diese Backups, machst selbst eins, behältst eins für immer oder spielst eins zurück.

## Die Spielstände jedes Profils

Jedes Profil hat seine eigenen Spielstände. Ein Spielstand gehört zu dem Profil, das aktiv war, als du ihn zuletzt bei offenem Studio gespielt hast; ein ohne Studio gespielter Spielstand gehört nur dann zu einem Profil, wenn die Mods dieses Profils genau die des Spielstands sind. Ein Spielstand wird nie in ein anderes Profil verschoben.

- Dieser Bildschirm zeigt die Spielstände des aktiven Profils. **Alle zeigen (N weitere)** fügt die Spielstände der anderen Profile und die ohne Profil hinzu, jeder mit einem Etikett **Profil: <Name>** oder **Kein Profil**. **Nur die von <Name>** geht zurück zu den Spielständen des aktiven Profils.
- Der Spielstand eines Servers erscheint als seine Adresse (`ip:port`) mit einem Etikett **Online** und gehört zu dem Profil, mit dem du diesem Server unter **Server** beigetreten bist (der Link läuft nicht auf einem Server, also richtet sich das Studio nach seiner eigenen Serverliste).
- Im Spiel, mit der Mod Studio Link: Der Bildschirm **Spielstand laden** listet die Spielstände des aktiven Profils, mit **Alle zeigen** neben dem Suchfeld, und **Fortsetzen** lädt den neuesten Spielstand des aktiven Profils (er ist ausgeblendet, wenn das Profil keinen hat). Ohne das Studio listet das Spiel wie gewohnt jeden Spielstand.

## Was du siehst

- Oben öffnet **Backup-Ordner öffnen** den Ordner, in den die Backups gehen (`Zomboid/PZModStudio_SaveBackups`), und der runde Pfeil aktualisiert den Bildschirm.
- Eine Leiste mit dem Platz auf der Festplatte: wie viel frei ist, die Größe der Festplatte und wie viel die Backups belegen. Sie wird gelb mit **Wenig Platz übrig**, wenn dein größter Spielstand vielleicht nicht noch zweimal hineinpasst.
- Eine Zeile pro Spielstand: der Name seiner Welt, dann Spielmodus, Größe, Anzahl der Dateien, Anzahl der Mods und wann er sich zuletzt geändert hat. Ein Etikett **erstellt auf <Build>** nennt den Build des Spiels, der den Spielstand geschrieben hat; ein neuerer Build aktualisiert ihn, wenn du ihn lädst.
- Haben sich die Mods des Spielstands seit seiner letzten Sitzung geändert, sagt das eine gelbe Zeile: **Seit seiner letzten Sitzung (...)**, mit wie vielen Mods hinzugefügt, entfernt oder aktualisiert wurden oder dass sich die Reihenfolge geändert hat. Fahr mit der Maus darüber, um die Namen der Mods zu sehen.
- Nennt die Mod-Liste des Spielstands Mods, die nicht installiert sind, sagt das eine rote Zeile: **Lädt nicht: N Mods seiner Liste sind nicht installiert**, mit einem Knopf pro Mod. Klick auf einen, um ihn unter **Mods finden** zu öffnen (den Workshop-Artikel, wenn die Liste ihn nennt, sonst eine Suche nach seiner ID). Spielstände von Servern haben keine eigene Liste: Das Spiel lädt die Mods des Servers beim Beitreten herunter.
- Bringen die eigenen Mods des Spielstands Karten mit, die der Spielstand nicht lädt, sagt das eine gelbe Zeile: **Lädt N Karten seiner Mods nicht**, mit einem Etikett pro Kartenordner. Ein Spielstand behält die Liste der Karten, mit denen er erstellt wurde, und das Spiel fügt ihr nie etwas hinzu, also lädt eine später angeschaltete Karten-Mod ohne ihre Karte (leeres Land, wo ihre Zellen sein sollten). Bei geschlossenem Spiel sichert **Karten zum Spielstand hinzufügen** den Spielstand, setzt diese Karten in seine Liste und verschiebt das schon erzeugte leere Land in ihren Zellen in den Backup-Ordner, nach einer Bestätigung.
- Hat das aktive Profil Mods aktiv, die der eigenen Liste des Spielstands fehlen, sagt es eine grüne Zeile: **N in <Profil> aktive Mods sind nicht in diesem Spielstand**, mit einem Chip pro Mod (ein Globus markiert Mods mit Kartenbereichen). Ein Spielstand lädt mit seiner eigenen Liste, nicht mit dem Profil: Eine Mod, die nach seinem Erstellen aktiviert wurde, lädt nicht mit ihm. Bei geschlossenem Spiel sichert **Diesem Spielstand hinzufügen** den Spielstand und setzt sie nach einer Bestätigung dort in seine Liste, wo die Ladereihenfolge des Profils sie einordnet; ihre Karten kommen hinzu wie bei **Karten zum Spielstand hinzufügen**. Spielstände eines anderen Profils bleiben unberührt.
- **Charakter und Fahrzeuge** öffnet, was der Spielstand enthält, aus seinen eigenen Dateien gelesen: jeden Charakter mit Name, Position und ob er tot ist, die Gegenstände, die er trägt, und jedes Fahrzeug mit seiner Position, wie weit es vom Charakter entfernt ist und wie viele Gegenstände darin sind. Die Anzahl der Gegenstände ist ungefähr.
- Rechts in jeder Zeile sein Zustand:
  - **Gesichert, wie er ist**: Das neueste Backup stimmt mit dem Spielstand überein.
  - **Geändert seit dem Backup vom ...**: Der Spielstand hat sich nach seinem letzten Backup geändert.
  - **Kein Backup**: Es gibt noch keins.
  - **Wird gesichert...**, während ein Backup läuft.

## Wann das Studio ein Backup macht

- **Als du ihn verlassen hast**: wenn du ins Hauptmenü zurückgehst oder das Spiel sich schließt, während du im Spielstand bist.
- **Bevor er mit geänderten Mods geladen wurde**: Die Mod Studio Link fordert es an, wenn du einen Spielstand lädst, dessen Mods nicht die seiner letzten Sitzung sind oder seitdem aktualisiert wurden.
- **Bevor seine Mod-Liste geändert wurde**: wenn die Mod-Liste des Spielstands im Spiel gleich geändert wird, oder vor **Karten zum Spielstand hinzufügen**.
- **Vor einer Wiederherstellung**: der Spielstand, wie er war, direkt bevor du ein älteres Backup zurückspielst.

Ein Spielstand, der sich seit seinem neuesten Backup nicht geändert hat, wird nicht noch einmal gezippt.

## Selbst ein Backup machen

Drück **Jetzt sichern** in der Zeile. Das Backup ist ein Zip, das nach dem Schreiben wieder eingelesen wird; geprüfte Backups zeigen **geprüft**. Hat sich der Spielstand nicht geändert, wird kein neues Zip gemacht: Stattdessen wird das neueste Backup als behalten markiert. Der Knopf ist ausgegraut, wenn auf der Festplatte kein Platz für den Spielstand plus etwas Reserve ist.

## Die Backups eines Spielstands

Klick auf den Pfeil links in einer Zeile, um ihre Backups zu öffnen. Jedes zeigt, wann es gemacht wurde, warum (**von dir erstellt**, **beim Verlassen**, ...), die Größe des Zips, die Größe des Spielstands, Dateien und Mods. Für jedes Backup:

- Der Pin-Knopf behält es: Ein behaltenes Backup wird nie entfernt, um Platz zu schaffen. Drück ihn noch einmal, um es wieder zu einem automatischen Backup zu machen.
- **Wiederherstellen** spielt dieses Backup als Spielstand zurück, nach einer Bestätigung.
- Der Papierkorb-Knopf (**Dieses Backup entfernen**) löscht es, nach einer Bestätigung. Das lässt sich nicht rückgängig machen.

## Einen Spielstand wiederherstellen

1. Schließ Project Zomboid. Eine Wiederherstellung wird abgelehnt, solange das Spiel läuft.
2. Öffne die Backups des Spielstands und drück **Wiederherstellen** beim gewünschten.
3. Bestätige. Das Studio prüft das Backup, sichert den Spielstand, wie er jetzt ist (damit du es rückgängig machen kannst), entpackt das Backup neben den Spielstand und tauscht sie erst dann aus.

## Gut zu wissen

- Die neuesten 5 automatischen Backups jedes Spielstands werden behalten; ältere fallen weg, wenn ein neues gemacht wird. Backups, die du machst oder anpinnst, bleiben, bis du sie entfernst.
- Automatische Backups brauchen PZ Mod Studio offen, während du spielst, und die Mod Studio Link im Spiel an (sie sagt der App, wann du einen Spielstand betrittst und verlässt).
- Der Spielstand eines Servers wird gesichert, wenn das Spiel sich bei offenem Studio schließt, falls er sich in dieser Sitzung geändert hat.
- Bei geschlossener App warnt die Link-Mod vor dem Laden, dass der Spielstand nicht gesichert werden kann, und lässt dich ohne Backup weitermachen.
- Die Link-Mod kann einen Spielstand auch auf einer Kopie spielen (`<world>_copy`), die die App macht; das Original bleibt, wie es ist.
- Wenn du einen Spielstand im Spiel lädst, fragt der Studio Link vorher dasselbe: **Hinzufügen und laden** (die App sichert den Spielstand und fügt sie dann hinzu), **Ohne sie laden** (der Spielstand bleibt, wie er ist) oder **Zurück**. Zum Hinzufügen muss PZ Mod Studio geöffnet sein.
