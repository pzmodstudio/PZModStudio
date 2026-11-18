# Diagnose

Die Diagnose bündelt, was die App über deine Mods, das Spiel und den Server weiß. Oben vier Knöpfe: **Übersicht**, **Mods**, **Spiel** und **Server**. Sie öffnet immer mit der Übersicht; jeder Teil merkt sich den Reiter, den du zuletzt angesehen hast.

## Übersicht und der Doktor

- Drei Karten, eine pro Teil, jede sagt in einer Zeile, wie es steht. **Mods**: etwas, das das Spiel kaputt macht, etwas, das sich zu beheben lohnt, oder alles in Ordnung. **Spiel**: offen oder geschlossen, wie viel Arbeitsspeicher es nutzt, der letzte Start, das Speicherlimit und ob einer Sitzung der Speicher ausgegangen ist. **Server**: ob einer auf diesem PC läuft und wie es ihm geht. Klick auf eine Karte, um diesen Teil zu öffnen.
- Darunter **der Doktor**: Er fragt, was los ist (**Das Spiel stürzt ab oder startet nicht**, **Eine Mod funktioniert nicht**, **Es ist langsam**, **Etwas anderes**), und führt dich Schritt für Schritt mit dem, was die App schon weiß: was die Mod-Prüfung gefunden hat, die Fehler des Monitors und die Male, als dem Spiel der Speicher ausging. Schickt dich ein Schritt woandershin, findest du beim Zurückkommen denselben Schritt. **Von vorn** geht zurück zur Frage.
- Der Knopf **Doktor** ganz oben öffnet diese Seite von jedem Bildschirm aus.

## Mods

Prüft deine aktiven Mods so, wie das Spiel sie liest, und listet jedes gefundene Problem mit einem Knopf zum Beheben. Sie schaut darauf, was dein Build des Spiels laden kann, fehlende oder ausgeschaltete Voraussetzungen, die Ladereihenfolge, inkompatible Mods, Dateien, die zwei Mods beide ändern, und Mods, die beim letzten Spielen die Konsole vollgeschrieben haben.

- Oben: dein **Spiel-Build** und wie viele Mods aktiv sind.
- **Studio-Hinweise** (zum Beispiel **Studio-Hinweise: 22 Mods**): die Laderegeln, inkompatiblen Paare und bekannten Probleme, die die App mitbringt. Hast du Warnungen mit **Passiert nicht mehr** markiert, sagt ein Knopf daneben wie viele und zeigt sie alle wieder.
- **Erneut prüfen** führt die Prüfung noch einmal aus. Sie läuft auch von selbst, wenn sich deine Liste ändert.
- Fünf Reiter; die ersten drei sagen, wie viele Zeilen nach etwas verlangen:
  - **Was jetzt zu tun ist**: was das Spiel kaputt macht oder machen wird (fehlende oder ausgeschaltete Voraussetzungen, Ladereihenfolge, Mods, die dein Build nicht laden kann, inkompatible Paare). **N Problem(e) automatisch beheben** ist hier. Darunter eingeklappt: was das Studio schon behoben hat und was nur gut zu wissen ist.
  - **Konflikte**: zwei Mods, die dieselbe Datei, denselben Gegenstand, dasselbe Rezept oder denselben Text liefern; das Spiel nutzt einen, und welchen, entscheidest du. **N Dateien automatisch zusammenführen** ist hier.
  - **Sicherheit**: was eine Mod über das Modden hinaus tun könnte (unten).
  - **Prüfliste**: deine Liste Mod für Mod, jede unter einem Urteil: **Kaputt**, **Wählen**, **Unnötig**, **Prüfen** oder **Behalten**. Klick auf eine Mod, um zu sehen warum, und **Los**, um zur Warnung zu gehen.
  - **Zeit pro Mod**: welche Mods das Spiel beim Spielen bremsen (unten).
- Jede Zeile zeigt die Mod, was falsch ist, einen kurzen Code (wie `missing_dependency`) und rechts ihre Lösungsknöpfe. **Für einen Assistenten kopieren** kopiert die Zeile mit einer Frage, zum Einfügen in einen beliebigen Chat-Assistenten; aus dem Studio wird nichts gesendet. Ein Problem-Etikett in der Mod-Liste öffnet den Reiter, der es enthält.

Ist nichts falsch, siehst du **Keine Probleme in deinen N aktiven Mods gefunden.**

### Was mit jeder Zeile zu tun ist

Jede Zeile sagt eins von vier Dingen, genau wie in der Mod-Liste:

- **Du musst handeln**: Das Spiel geht kaputt oder lädt die Mod nicht, bis du etwas tust. Fang hier an.
- **Lohnt sich**: Es funktioniert, aber es lohnt sich, es zu beheben (falsche Reihenfolge, ein noch nicht heruntergeladenes Update, eine Mod, die die Konsole vollschreibt). Der Knopf sagt, was er ändert.
- **Gut zu wissen**: nichts zu tun, nur damit du es weißt (zwei Mods definieren dasselbe, Mods, die seit deinem letzten Spiel aktualisiert wurden).
- **Vom Studio schon behoben**: Lass es in Ruhe, so funktioniert es (eine kompatible Kopie, eine Datei in deinem Paket). Jedes lässt sich bei Bedarf rückgängig machen.

Jeder Abschnitt lässt sich mit einem Klick auf seinen Titel einklappen.

### Die Probleme, die sie findet

- `missing_mod` (Nicht installiert): Eine Mod deiner Liste ist nicht auf der Festplatte. Hat eine installierte Mod fast dieselbe ID, bietet sie an, stattdessen diese anzuschalten.
- `not_for_build`, `outside_version_range`, `invalid_mod_info`: Das Spiel lädt diese Mod auf deinem Build nicht, oder ihre `mod.info` ist kaputt.
- `missing_dependency`: Die Mod benötigt eine andere, die nicht installiert ist.
- `dependency_not_enabled`: Die benötigte Mod ist installiert, aber aus.
- `dependency_after`, `load_after_order`, `load_before_order`: Zwei Mods stehen in der falschen Reihenfolge.
- `dependency_unavailable`: Eine benötigte Mod kann selbst nicht laden.
- `incompatible`: Die Mod sagt, dass sie nicht mit einer anderen Mod funktioniert, die du an hast.
- `duplicate_entry`: Dieselbe Mod steht zweimal in der Liste.
- `two_mod_managers`: Ein anderer Mod-Manager, der den Mods-Bildschirm des Spiels zeichnet, ist neben Studio Link an.
- `workshop_update_pending`: Steam hat eine neuere Version, die noch nicht heruntergeladen ist. Server lehnen Spieler ab, deren Mod-Versionen abweichen.
- `file_conflict`: Zwei Mods bringen dieselbe Datei mit, und das Spiel nutzt nur die letzte.
- `function_overridden` (**Schneidet eine andere Mod ab**): Eine Mod ersetzt eine Funktion des Spiels oder einer anderen Mod, nachdem eine andere Mod sie geändert hat, also läuft diese Änderung nie. Werden viele Funktionen einer Mod von einer anderen abgeschnitten (zwei Versionen derselben Mod gleichzeitig an), ist es eine Zeile. Mit Studio Link an wird das im laufenden Spiel geprüft: Die Zeile sagt **Im laufenden Spiel gesehen**, oder `function_seen_other` (**Andere Version im Spiel**) erscheint, wenn das Spiel die Version einer anderen Mod hat.
- `java_link_broken` (**Java älter als dieser Build**): Das Java der Mod nutzt Klassen, Methoden oder Felder, die das installierte Spiel nicht mehr hat (ein Update hat sie geändert). Dieser Code schlägt fehl, wenn er läuft. Zeigt der Monitor, dass er fehlschlägt, such nach einem Update der Mod oder schalte sie aus; funktioniert es, prüft die Mod vorher selbst.
- `tiledef_clash`: Zwei Mods legen ihre Tiles unter derselben `tiledef`-Nummer mit verschiedenen Sheets ab, sodass manche Tiles als die der anderen Mod erscheinen oder fehlen. Lass eine an oder bitte einen Autor, die Nummer zu ändern. Zeilen `file_conflict` sagen außerdem, ob das Log der letzten Sitzung (mit genau dieser Liste) bestätigt, welche Kopie das Spiel genutzt hat.
- `game_folder_changed` (**Spielordner hat sich geändert**): Während das Spiel lief, haben sich Dateien in seinem eigenen Ordner geändert, in den das Spiel nie schreibt. Die App vergleicht den Ordner des Spiels, den Zomboid-Ordner (ohne Spielstände oder Logs) und den von ZombieBuddy beim Öffnen und Schließen des Spiels, nur solange die App offen ist.
- `script_definition_conflict` und `translation_conflict`: Zwei Mods definieren denselben Gegenstand, dasselbe Rezept oder denselben Namen unterschiedlich; die zuletzt ladende gewinnt. Zeilen, die auf `_stale` enden, bedeuten, dass du es schon zusammengeführt hast, aber eine Mod sich seitdem geändert hat; `_settled` bedeutet, dass dein Paket es schon regelt.
- `floods_console` (in der Mod-Liste als **Flutet die Konsole**): In deiner letzten Sitzung hat diese Mod mehr als 1.000 Fehlerzeilen in die Konsole geschrieben. Das bremst das Spiel und versteckt die Fehler anderer Mods. Sieh sie dir im Monitor an oder schalte die Mod aus.
- `user_rule_order`: Eine deiner eigenen Laderegeln ist gebrochen.
- `community_rule_order`, `community_incompatible`, `community_known_issue`, `community_successor`: was die Regeln und der Mod-Index des Studios über deine Mods sagen. Ein bekanntes Problem hat **Passiert nicht mehr**: Es wird auf diesem PC nicht mehr gezeigt.
- `learned_issue`: ein Fehler, den eine Mod in 3 oder mehr deiner Sitzungen mit derselben Version hatte (siehe "Was dieser PC lernt").
- `package_built_for_other_list`: Dein Studio-Paket wurde mit einer anderen Mod-Liste als der dieses Profils gebaut.
- `package_file_stale`: Dein Paket trägt eine alte Kopie einer Datei einer Mod, und die Mod hat diese Datei seitdem geändert. Das Spiel liest die alte Kopie.
- `package_file_differs`, `package_file_orphan`: Dein Paket trägt eine Datei, die nur eine Mod der Liste mitbringt oder keine Mod der Liste mitbringt. Nimm sie heraus, wenn du sie nicht mehr brauchst.
- `security_executable`, `security_script`, `security_java`, `security_lua`: was eine Mod über das Modden hinaus tun könnte, im Reiter **Sicherheit** (unten).
- `compatible_copy`, `shadowed_copy`, `updated_since_last_play`: Hinweise dazu, welche Kopie einer Mod genutzt wird und was sich geändert hat.

Zeilen zur selben Definition lassen sich mit **Nach Datei** öffnen, um die Datei jeder Mod zu sehen. Klick dort auf eine Definition, um sie im Merger zu öffnen.

### Die Lösungsknöpfe

- **X aktivieren** oder **X deaktivieren**: schaltet eine Mod an oder aus. Ausschalten fragt vorher; die Mod bleibt installiert.
- **X vor Y verschieben**: ändert die Ladereihenfolge. In einer Zeile, bei der du entscheidest (zwei Mods definieren dasselbe), heißt er **Stattdessen Version von Y nutzen**.
- **X aus der Liste entfernen**: nimmt einen Eintrag aus der Liste.
- **Im Workshop nach "X" suchen** und **Workshop-Seite öffnen**: öffnen den Steam Workshop in deinem Browser.
- **Kompatible Kopie erstellen**: erstellt in `Zomboid/mods` eine angepasste Kopie einer Workshop-Mod für deinen Build. Das Ergebnis zeigt, was geändert wurde und was noch kaputtgehen kann. Starte das Spiel aus dem Studio, damit es die Kopie nutzt. **Zurück zum Original** entfernt die Kopie.
- **Deine Version speichern**: speichert die Mod, wie sie heute ist, bevor ein Workshop-Update ankommt. Du kannst sie in der Mod-Liste zurückholen (Rechtsklick, Gespeicherte Versionen).
- **Automatisch zusammenführen** und **Im Merger öffnen**: regeln einen Datei- oder Definitionskonflikt in deinem Patch-Paket, das zuletzt lädt.
- **Aus dem Paket nehmen**: nimmt nach Rückfrage eine Datei aus deinem Patch-Paket; das Spiel liest beim nächsten Start wieder die eigene Datei der Mod.
- **X-Patch des Shims einschalten**: schaltet einen Patch des Build-42-Shims ein, den ein bekanntes Problem braucht.

Eine grüne Zeile bestätigt jede Änderung, mit **In deiner Mod-Liste gespeichert.**

### Viele auf einmal beheben

- **N Problem(e) automatisch beheben** schaltet fehlende Voraussetzungen an und verschiebt Mods nach vorn, wo nötig. Es schaltet nie etwas aus und überspringt die Zeilen, bei denen du entscheidest. Es fragt vorher, und die Mod-Liste kann es rückgängig machen.
- **N Dateien automatisch zusammenführen** führt jede kollidierende Datei ohne überlappende Änderungen in dein Patch-Paket zusammen. Dateien, in denen beide Mods dieselben Zeilen ändern, werden aufgelistet, damit du im Merger entscheidest.

### Die Regeln und der Index des Studios

Das Studio bringt Laderegeln mit, Paare von Mods, die nicht zusammenpassen, und einen Index bekannter Probleme mit ihrer Lösung. Sie kommen mit jeder Version der App; nichts wird heruntergeladen und nichts gesendet. Sie erscheinen als Zeilen `community_*`. Ein bekanntes Problem wird von selbst weggelassen, wenn die eigenen Dateien der Mod zeigen, dass es weg ist (ein Update hat die betroffene Datei repariert), oder wenn sich die Mod nach dem Auftreten des Problems geändert hat und seine Log-Zeile in deiner Sitzung nicht mehr erscheint. **Passiert nicht mehr** lässt eins von Hand weg.

### Was dieser PC lernt

Neben dem mitgelieferten Index lernt jede App von ihrem eigenen PC. Im Hintergrund liest sie die Sitzungslogs von Spiel und Server, die das Spiel aufbewahrt (`Zomboid/Logs`), und notiert, welche Mods an waren, in welcher Version, und welche Fehler das Spiel ihnen zugeschrieben hat (dieselben, die der Monitor zeigt). Sie speichert das in `Zomboid/PZModStudio_Learned.json` und sendet nichts.

Hat eine Mod denselben Fehler in 3 oder mehr Sitzungen, mit derselben Version und auf dem installierten Build, erscheint er in der Diagnose als **Auf diesem PC gelernt** (`learned_issue`), mit der Anzahl als Beleg: "in 8 von 14 deiner Sitzungen". Spricht der Index der App schon von diesem Problem, erscheint nur die Zeile des Index. Ein Java-Patch, den das Log mehreren Mods gleichzeitig zuschreibt, wird nicht gelernt, weil nicht klar ist, wessen er ist. Wird die Mod aktualisiert, fängt die Zählung bei null an; liefen ihre letzten 5 Sitzungen ohne ihn (behoben, oder eine Lösung, die gewirkt hat), wird er nicht mehr gezeigt. **Passiert nicht mehr** nimmt ihn von Hand heraus. Die Startseite der Diagnose sagt, aus wie vielen Sitzungen sie gelernt hat.


### Der Doktor und das Gedächtnis

Die Startseite der Diagnose hat **Frag den Doktor**, einen Chat, der ohne KI antwortet: Er erkennt an deinen Worten, was du fragst, und baut die Antwort aus dem, was dieser PC gelernt hat, und aus den Werkzeugen der App (dieselben wie die des MCP-Servers). Jede Antwort sagt, von welchem Werkzeug sie kommt. Frag ihn, was er weiß, warum sich das Spiel geschlossen hat, was sich geändert hat, als ein Fehler anfing, welche Mods scheitern, ob das Laden langsamer ist, ob Lösungen gewirkt haben, was das letzte Update des Spiels bewirkt hat, oder nenn eine Mod, um ihre Geschichte zu sehen.

Er verzeiht Tippfehler und andere Formulierungen ("es ist abgestürzt", "es hängt", "was hat sich geändret"), liest Daten ("was hat sich seit gestern geändert?", "diese Woche", "seit 12.11."), "welche lädt am längsten?" oder "welche Mod macht die meisten Fehler?" und folgt dem Gespräch: Nach einer Mod kannst du "und seit wann?", "schalt sie aus", "notier es" oder "zeig sie" sagen. Ist er unsicher, fragt er "welche davon?" mit Knöpfen; versteht er etwas nicht und zeigst du ihm mit einem Knopf, was du meintest, merkt er sich diese Formulierung auf diesem PC. Jede Antwort beginnt mit dem Fazit, dann ein Rat mit seinem Beleg (zum Beispiel, dass fast alle Abstürze daher kamen, dass dem PC mit Spiel und offenem Server der Speicher ausging), die Details eingeklappt, und Knöpfe, um zur Mod-Liste oder zum betroffenen Teil der Diagnose zu gehen.

Außerdem:

- **Welche Mod ist schuld?** Er vergleicht die Sitzungen mit und ohne jede Mod: "mit X liefen 3 von 7 schlecht; ohne sie 24 von 196", und sagt, wie sicher er ist (ziemlich sicher, wahrscheinlich, könnte Zufall sein). Eine Mod, die in fast jeder Sitzung ist, lässt sich nicht vergleichen und wird nicht genannt.
- **Per Halbierung suchen** aus dem Chat: Er schaltet die Hälfte deiner Mods aus (was die anderen brauchen, bleibt an), lässt dich die Probe spielen, und du antwortest "passiert noch" oder "passiert nicht mehr", bis eine übrig ist. Deine Liste wird aufbewahrt und kommt am Ende zurück.
- **Der Bericht**: Öffnest du den Chat nach dem Spielen, erzählt er, wie deine letzte Sitzung lief (gespielte Zeit, wie sie endete, neue Fehler, Lösungen, die gewirkt haben). "Was ist diese Woche passiert?" macht dasselbe für einen Zeitraum.
- **Fragen zur App** ("wie erstelle ich ein Profil?", "was ist eine kompatible Kopie?"): Er antwortet mit dem Absatz dieser Anleitung und einem Knopf zu diesem Bildschirm.
- **Gruppen, Profile und der Workshop**: "VP Base" sagt, wie es dieser Gruppe geht, "Profil Knox Lives", ob seine letzten Sitzungen gut liefen; "gibt es eine neue Version von X?" und "was sagen die Kommentare zu X?" lesen ihre Seite.
- **Mehrere Fragen auf einmal** ("was hat sich geändert und welche Mods scheitern?"), **Spitznamen**, die er lernt, wenn du unter mehreren Mods wählst, **"mach das rückgängig"** für seine letzte Änderung, und **Gedächtnis**: Beim nächsten Mal erinnert er dich daran, über welche Mod ihr gesprochen habt und wie es ihr jetzt geht.

**Mit dem Spiel (Studio Link):**

- Im Hauptmenü des Spiels zeigt die Karte von PZ Mod Studio (oben rechts), was der Doktor zuerst sagen würde: ob die letzten Sitzungen mit dieser Liste schlecht liefen, ob die letzte abgestürzt ist, welche Fehler sich wiederholen. Die App schreibt das bei jeder Prüfung, also sagt die Karte das Letzte, was sie wusste, auch wenn die App geschlossen ist.
- Der Link führt ein Tagebuch darüber, welchen Spielstand du geladen hast und ob du ins Menü zurückgegangen bist (statt das Fenster zu schließen). So sagt jede Sitzung "Spielstand X · ins Menü zurückgegangen", auch die, die bei geschlossener App gespielt wurden.
- Im Pausenmenü (Esc) gibt es **Moment markieren**: Passiert etwas Seltsames, drück es. Frag dann den Doktor "was ist passiert, als ich markiert habe?", und er sagt dir, was das Log des Spiels in dieser Minute geschrieben hat (anderthalb Minuten davor bis eine halbe Minute danach).

Er kann auch Dinge ändern, immer mit deinem **Machen**: Gruppen des aktiven Profils erstellen, umbenennen oder löschen und Mods hineinlegen oder herausnehmen ("Gruppe A, B in Karten", "nimm A aus der Gruppe Karten", "benenne Gruppe Karten in Welt um", "lösch Gruppe Karten"), Gruppen für deine aktiven Mods ohne Gruppe vorschlagen (die, die einen Namen teilen oder dasselbe benötigen), und **Studio-Notizen** schreiben oder entfernen ("Notiz zu A: Text", "entferne die Studio-Notiz von A", "notier, was du gelernt hast"). Studio-Notizen werden getrennt von deinen aufbewahrt und in Violett mit einem eigenen Knopf zum Entfernen gezeigt; deine Notizen schreibt, ändert oder entfernt er nie.

Was das Gedächtnis aus den Sitzungen erzählt:

- **Was sich geändert hat**: Wiederholt sich ein Fehler, wann er anfing und was anders war als in der letzten Sitzung ohne ihn (Mods hinzugefügt, entfernt oder aktualisiert, ihr Platz in der Reihenfolge, der Build des Spiels). Es sagt das als das, was sich damals geändert hat, nicht als Ursache. Es erscheint in der Zeile der Diagnose und in der Geschichte der Mod.
- **Gemerkte Lösungen**: Jede Lösung, die du aus der Diagnose anwendest, wird notiert, und die Sitzungen danach sagen, ob der Fehler der Mod aufgehört hat.
- **Vor dem Spielen**: Haben es die letzten Sitzungen mit fast derselben Liste nicht einmal ins Menü geschafft oder ging ihnen der Speicher aus, sagt die App das, wenn du Spielen drückst, mit dem, was sich von der letzten unterscheidet, die in einen Spielstand kam, und fragt, ob du trotzdem spielen willst.
- **Die Geschichte einer Mod**: in der Mod-Liste der Abschnitt **Auf diesem PC** im rechten Feld.
- **Langsameres Laden** und ein **Update des Spiels**, das mehrere Mods auf einmal scheitern ließ.
- **Echte Abstürze**: Fällt Java um, hinterlässt es einen Bericht (`hs_err_pid…log`) im Ordner des Spiels; das Gedächtnis liest ihn und ordnet ihn seiner Sitzung zu ("es ist umgefallen: dem PC ging der Speicher aus" oder "ein fataler Java-Fehler in dieser Bibliothek"). Das Spiel mit X oder Alt+F4 zu schließen hinterlässt keinen, also zählt das nie als Absturz.

### Sicherheit

Der Reiter **Sicherheit** listet, was eine Mod über das Modden hinaus tun könnte, aus ihren Dateien gelesen: Programmdateien oder Skripte, die sie mitbringt, Java, das Programme starten oder ins Internet gehen kann, und Lua, das aufruft, was die eigenen Skripte des Spiels nie aufrufen. Nichts davon ist ein Urteil: Viele Mods tun diese Dinge aus guten Gründen. Die meisten Zeilen sind Info; eine Warnung markiert ein Muster, für das keine Mod einen guten Grund hat. Die Zeile einer Workshop-Mod hat **Workshop-Seite öffnen**.

### Zeit pro Mod

Unter **Mods** > **Zeit pro Mod**: welche Mods das Spiel beim Spielen bremsen. **Messen** setzt eine kleine Studio-Mod (PZ Mod Studio Profiler) an die erste Stelle deiner Mod-Liste und des zuletzt gespielten Spielstands; starte das Spiel (neu, wenn es offen war) und spiel ein paar Minuten wie immer. Jeder Event-Handler jeder Mod wird vom Spiel gestoppt und pro Mod addiert: der Anteil an der echten Zeit, Millisekunden pro Sekunde und ihr längster einzelner Aufruf (ein langer ist ein Hänger, den du siehst). Öffne eine Zeile für die Handler, die am meisten kosten, mit Datei und Zeile. **Messen beenden** nimmt den Profiler wieder aus der Liste und dem Spielstand; die letzten Zahlen bleiben.

Die Fenster, Leisten und zeitgesteuerten Aktionen einer Mod zählen auch (als Class:render, Class:update); jede Funktion zählt nur ihre eigene Zeit. Der Profiler fügt jedem Handler etwas eigene Zeit hinzu, also stopp ihn, wenn du fertig bist.

## Spiel

Was die App über das Spiel selbst weiß, abgesehen von den Mods. Fünf Reiter: **Live**, **Speicher**, **Start**, **Frames** und **Festplatte**.

### Live

Was das Spiel gerade nutzt: oben der freie Arbeitsspeicher des PCs (die Anzeige in der oberen Leiste führt zu diesem Reiter). Darunter eine Karte pro offenem Spiel: Speicher gegenüber seinem Java-Limit, CPU, auf welchem Bildschirm es ist (Menü, Verbinden, im Spiel), die letzte Zeile seines Logs, wie viele Fehler es hat, und Knöpfe, um seinen Ordner und sein Log zu öffnen. Bei geschlossenem Spiel zeigt sie, was vom letzten Mal bekannt ist: den letzten Start, das Speicherlimit, Sitzungen, denen der Speicher ausging, und die Male, als Windows der Speicher ausging.

### Speicher

Unter **Spiel** > **Speicher**: wie viel Arbeitsspeicher das Java des Spiels nutzen darf (`-Xmx` in `ProjectZomboid64.json`), vom Speicher des PCs, mit einem Vorschlag für deine aktiven Mods und einer Warnung, wenn einer neueren Sitzung der Speicher ausging. Wähle einen Wert (2 GB bis zur Hälfte des PC-Speichers) und **Übernehmen** bei geschlossenem Spiel; er gilt ab dem nächsten Start, und eine Kopie der Datei vor der Änderung kommt nach `PZModStudio_Backups`.

Jedes Profil hat seinen eigenen Wert: Ein Profil anzuschalten gibt dem Spiel seinen Speicher. Ein Profil ohne eigenen Wert nutzt den zuletzt gewählten. Setzt ein Update des Spiels seinen eigenen Wert zurück, stellt das Studio deinen beim nächsten Start des Spiels wieder her.

Ging Windows selbst in den letzten 14 Tagen bei offenem Spiel der Speicher aus, steht das hier auch, mit dem, was hilft (eine größere Auslagerungsdatei, andere große Programme schließen). Das ist nicht der Java-Speicher des Spiels, also behebt es ein höherer Wert oben nicht.

### Start

Unter **Spiel** > **Start**: wie lange das Spiel bis zum Hauptmenü gebraucht hat, Phase für Phase (Dateien der Mods, Skripte, Lua, Modelle und so weiter), aus dem Log, das jeder Start hinterlässt. Jeder Start steht neben deinem üblichen Start mit etwa gleich vielen Mods; eine gelbe Phase hat viel länger gedauert als sonst. Wähle einen früheren Start, um ihn zu sehen.

### Frames und Ruckler

Unter **Spiel** > **Frames**: Frame-Zeiten beim Spielen (FPS, Frames über 25, 50 und 100 ms), geladene Chunks und pro Mod die Handler, die einen Frame am längsten aufgehalten haben, mit Datei und Zeile. Derselbe Profiler wie unter **Mods** > **Zeit pro Mod** zählt sie, also drück dort zuerst **Messen** und spiel ein paar Minuten. **Neu lesen** liest die neuesten Zahlen.

### Festplatte

Oben **Platz freigeben**: **Cache leeren** löscht, was das Studio aufbewahrt, um dasselbe nicht noch einmal zu lesen oder zu erfragen (die Prüfungen jeder Mod, was Steam gesagt hat, Kartenbilder, Übersetzungen); es baut das von selbst wieder auf und rührt nie Mods, Spielstände, Profile oder Notizen an. Nach einem Mehrspieler-Test schickt **Ordner von Spieler 2** den Ordner `Zomboid2` des zweiten Spiels (mit Kopien der Mods des Servers) in den Papierkorb; dort verlinkte Mods in Entwicklung werden nicht angerührt, und der nächste Test erstellt ihn neu. Nicht, solange der Test läuft.

Unter **Spiel** > **Festplatte**: wie viel Platz jede installierte Mod belegt, gezählt nach dem, was zusammengehört (ein Workshop-Artikel mit all seinen Mods, ein Ordner in `Zomboid/mods`). Das Messen dauert ein paar Sekunden.

- **Nichts nutzt sie** (gelb): Kein Profil hat sie, weder an noch aus; kein Spielstand nennt sie in seiner Liste; kein gespeicherter Server hat sie geladen; sie ist nicht in der aktiven Liste des Spiels.
- **Überall aus** (hellblau): Ein Profil behält sie, ausgeschaltet, und kein Spielstand oder Server nutzt sie.
- **Überrest**: ein Workshop-Ordner eines Artikels, den du nicht mehr abonniert hast; Steam lässt sie manchmal liegen.
- **Dein Upload**: eine Mod von dir in `Zomboid/Workshop`. Wird gezeigt, aber nie zum Entfernen angeboten.

Hak die an, die weg sollen. Ein Workshop-Artikel geht mit **N Artikel deabonnieren** (Steam muss laufen und löscht die Dateien selbst). Ein lokaler Ordner oder ein Überrest geht mit **N Ordner in den Papierkorb**: Er lässt sich aus dem Papierkorb wiederherstellen. Jedes fragt vorher, mit den Namen. Solange das Spiel offen ist, kommt nichts in den Papierkorb.

Darunter **Spiel- und Studio-Ordner**: wie viel Platz das Spiel, die Workshop-Mods, lokale Mods, Spielstände, Spielstand-Backups, gespeicherte Mod-Versionen, Backups der Mod-Liste, der Cache, halb installierte Downloads und die Logs belegen, mit einem Knopf, um jeden Ordner zu öffnen. Von dort wird nichts gelöscht.

## Server

Der Server, der auf diesem PC läuft (der, den du unter Server startest, oder der eines Mehrspieler-Tests), mit derselben Karte: Speicher gegenüber seinem Limit, CPU, ob er gestartet ist, die letzte Zeile seines Logs und wie viele Fehler er hat. Während eines Mehrspieler-Tests erscheinen auch seine Spieler. Läuft kein Server, führt ein Knopf zu **Server**, um einen zu starten.

Ein Server auf einem anderen PC, dem du beitrittst, lässt sich von hier nicht messen: nur der, der auf diesem PC läuft.

## Gut zu wissen

- Lösungen unter **Mods** werden in der Mod-Liste des aktiven Profils gespeichert.
- Zusammengeführte Dateien und Shim-Patches wirken beim nächsten Spielstart.
- "Flutet die Konsole" wird aus der `console.txt` der letzten Sitzung gemessen, also aktualisiert es sich, nachdem du gespielt hast.
- Ein Klick auf ein Problem-Etikett in der Mod-Liste öffnet diesen Bildschirm bei dieser Zeile.
