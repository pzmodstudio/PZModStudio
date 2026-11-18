# Server

Der Bildschirm Server hat oben drei Reiter: **Öffentliche Server** und **Meine Server**, um einen Mehrspieler-Server zu finden und ihm beizutreten, und **Server hosten**, um deinen eigenen dedizierten Server auf diesem PC laufen zu lassen. Unter **Server hosten** sagt eine Leiste, ob dein Server an ist, mit **Sehen, wie er läuft** (öffnet Diagnose > Server: Arbeitsspeicher, CPU, ob er gestartet ist und was sein Log sagt), **Einladung teilen** und dem Knopf, der die Serverdateien neu einliest.

## Einem Server beitreten: einen finden

Drück unter **Öffentliche Server** auf **Server suchen**. Das Studio fragt Steam nach jedem Project-Zomboid-Server, so wie es der eigene Browser des Spiels tut, über den Steam-Client, der auf diesem PC läuft (kein Schlüssel nötig; Steam zeigt dich währenddessen als in Project Zomboid). Server erscheinen, sobald sie antworten; alle zusammen dauern etwa anderthalb Minuten. **Stopp** beendet die Suche, **Erneut suchen** fängt von vorn an.

Grenze die Liste mit dem Feld (**Name oder Karte**) und den Filtern ein: **Nur mein Build**, **Mit Spielern**, **Nicht voll**, **Ohne Passwort**, **Mit Mods und Vanilla** / **Mit Mods** / **Vanilla**, und sortiere nach **Meiste Spieler**, **Niedrigster Ping** oder **Name**. Rechtsklick auf einen Server öffnet ein Menü: **Details ansehen**, **Beitreten**, **Mit Stern markieren**, **Adresse kopieren**, ein Profil mit seinen Mods erstellen und, für einen Server unter **Meine Server**, **Server vergessen**.

**Meine Server** enthält die Server, die du markierst (**Mit Stern**), und die, denen du von hier aus beigetreten bist (**Von hier beigetreten**). Um einen privaten Server oder den eines Freundes hinzuzufügen, gib unter **Server per Adresse hinzufügen** seine Adresse ein (`ip:port`; Port 16261, wenn weggelassen), einen optionalen Namen und das Passwort, und drück **Hinzufügen**. Das Passwort wird im Anmeldeinformationsspeicher von Windows aufbewahrt, nie in einer Datei.

## Einem Server beitreten: beitreten

1. Klick auf einen Server. Das Feld rechts zeigt seinen Build, ob er offen oder nur per Whitelist ist, seinen letzten Wipe und seine Mods. Jede Mod ist eingefärbt: **Installiert und an**, **Installiert** oder **Das Spiel lädt sie herunter**.
2. Läuft der Server auf einem anderen Build als deinem, sagt das eine Warnung: Das Spiel lässt dich nicht hinein, bis beide übereinstimmen (Steam-Betazweig).
3. Gib das Serverpasswort ein, falls er eins hat, und hak **Server-Passwort merken** an, um es zu behalten. Ein gespeichertes erscheint als grüne Zeile (**Serverpasswort in Windows gespeichert**) statt des Feldes: Das Studio trägt es beim Beitreten ein. **Ändern** öffnet das Feld für ein neues und **Vergessen** löscht es. Dein Kontopasswort kommt ins Verbindungsfenster des Spiels.
4. Schließ das Spiel, wenn es offen ist, und drück **Beitreten**.

Das Spiel startet im Verbindungsfenster dieses Servers mit eingetragenem Serverpasswort, und auch mit deinem Benutzernamen, wenn du ihn unter **Dein Benutzername auf diesem Server** eingegeben hast (das Studio merkt ihn sich; die Mod Studio Link trägt ihn ein). Dort gibst du dein Kontopasswort ein und drückst **Verbinden**, das ohne Benutzernamen aus bleibt. Ein neuer Benutzername erstellt ein neues Konto auf diesem Server. Das Spiel lädt die Mods des Servers herunter und nutzt sie nur dort: Deine eigene Mod-Liste bleibt, wie sie ist; das Profil, mit dem du beitrittst, bekommt sie (siehe unten).

Mit der Mod Studio Link kommt der Server außerdem in die **Favoriten** des Spiels (der Reiter Favoriten seines Mehrspieler-Bildschirms), mit deinem Benutzernamen darunter, als hättest du ihn mit **Server hinzufügen** und **Neues Konto hinzufügen** eingetragen. Wird die Verbindung getrennt, ist er also da, um wieder beizutreten, ohne das Spiel zu schließen. Er wird nie doppelt hinzugefügt, und dein Kontopasswort wird nicht gespeichert; das Serverpasswort schon, so wie das Spiel es speichert.

## Ein Profil für einen Server erstellen

Ein Server kündigt nur seine ersten paar Mods an (Steam kürzt die Liste). Nachdem du ihm einmal von hier aus beigetreten bist, kennt das Studio die volle Liste, und der Knopf wird zu **Profil mit diesen N Mods erstellen**. Vorher nutzt **Profil mit den N angekündigten Mods erstellen** die unvollständige Liste. Das Profil heißt "Server: (Name)" und behält die Reihenfolge des Servers. Mods, die gerade nicht installiert sind, werden genannt, bevor das Profil erstellt wird.

Um damit zu spielen, aktiviere es unter Profile, bevor du beitrittst. Jedes Mal, wenn du von hier aus beitrittst und das Spiel die Mods des Servers geladen hat, kommen sie in das Profil, das beim Beitreten aktiv war: an, in der Reihenfolge des Servers, mit denen, die das Spiel dich herunterladen ließ. Seine anderen Mods bleiben darin, ausgeschaltet, außer denen, die nur auf deiner Seite laufen (ZombieBuddy und seine reinen Client-Java-Mods, wie Viewpoint): Sie bleiben, wie du sie hattest. Das Feld des Servers sagt, welches Profil das ist.

## Server hosten: die Serverliste

Links listet **Serverkonfigurationen (.ini)** die Serverkonfigurationen in `Zomboid/Server`, mit der Anzahl ihrer Mods und Workshop-Artikel und **Läuft** für die, die gerade an ist. Gib einen Namen ein und drück **Erstellen**, um eine neue zu machen, oder **⚡ Standardkonfiguration erstellen (servertest.ini)**, wenn es keine gibt. Der Papierkorb löscht eine Konfiguration von der Festplatte (du wirst vorher gefragt). Wähle eine, um ihre Reiter zu sehen. Das Banner oben zeigt **ONLINE** oder **OFFLINE**, **Einladung teilen** (kopiert einen Einladungstext mit Name, Port, maximalen Spielern, Passwort und Anzahl der Mods; du trägst deine öffentliche IP ein und leitest den Port weiter) und einen Knopf, um die Serverdateien neu zu laden.

## Hosten & Starten

- **Arbeitsspeicher (-Xmx)**: 4, 8, 12 oder 16 GB für den Server.
- **Admin-Passwort**: legt das Passwort des Kontos `admin` fest, wenn der Server startet. Das Studio speichert es in der Windows-Anmeldeinformationsverwaltung (nie in einer Datei), damit ein mit dem Studio verbundener Assistent diesen Server neu starten kann, ohne es zu bekommen.
- **Anmeldung & Netzwerk**: **Steam an** oder **NoSteam (LAN)**.
- **Dedizierten Server starten (Name)** startet ihn in seinem eigenen Konsolenfenster. Während er läuft, stoppt **Dedizierten Server stoppen** den Server, den das Studio gestartet hat (nie andere Java-Programme), und **Neu starten** stoppt und startet ihn sofort neu (um die Spieler vorher zu warnen, nimm den Reiter **Neustarts**).

**Serverprogramm.** Ein Server läuft aus den eigenen Dateien des Spiels, außer du wählst etwas anderes. Das Feld **Serverprogramm** bietet stattdessen den offiziellen **Project Zomboid Dedicated Server** an: ein eigenes Programm, das Steam kostenlos anbietet (App 380870), das auf einem anderen Zweig als dein Spiel sein kann und getrennt davon aktualisiert wird.

1. Wähle einen **Ordner des dedizierten Servers**: einen leeren oder neuen Ordner auf einer Festplatte mit Platz (nicht der Ordner des Spiels, nicht `Zomboid`). Ein dedizierter Server, der schon auf diesem PC ist (aus Steams Bibliothek > Tools oder von einem früheren SteamCMD), wird gefunden und angeboten.
2. Wähle den **Zweig**: **Stabil (public)** oder **Instabil (Beta)**. Er muss zum Spiel derer passen, die beitreten; das Feld sagt, auf welchem Zweig dein Spiel ist, und startet auf diesem. Einen Zweig, den Steam hinter einem Passwort hält, kann man nicht installieren: Das Feld sagt das.
3. Drück **Installieren**. Das Studio nutzt SteamCMD, das Kommandozeilen-Steam von Valve. Ist es nicht auf diesem PC, fragt es zuerst, lädt es dann von Valves eigener Adresse herunter (etwa 3 MB), prüft, dass Valve es signiert hat, und lässt es sich selbst aktualisieren. Dann lädt SteamCMD den Server von Steam herunter (mehrere GB): Das Feld zeigt jeden Schritt mit Prozent und Gigabyte (etwa 7 GB für 42.22) und am Ende die Größe des Ordners. **Stopp** bricht ab; erneutes Drücken auf Installieren macht dort weiter, wo es war.
4. Ist er installiert, wähle **Der dedizierte Server**. Ab dann starten Server aus diesem Ordner, mit demselben Zomboid-Ordner (deine Serverkonfigurationen, Spielstände und `Zomboid/mods`). **Die Spieldateien** geht zurück zum üblichen Weg. Ändere das bei gestopptem Server.

**Aktualisieren** führt SteamCMD erneut für diesen Ordner und Zweig aus (und wechselt den Zweig, wenn du ihn geändert hast). Bei einer vollständigen Installation holt es nur, was sich geändert hat, und ist in Sekunden fertig ("Already up to date", wenn sich nichts geändert hat); nach einer abgebrochenen Installation oder einer gestoppten Dateiprüfung prüft es jede Datei, was viele Minuten dauert. Das Feld sagt auch, als welche Version des Spiels der Server startet und ob es die deines Spiels ist: Ein Spiel tritt nur einem Server seiner eigenen Version bei.

Java-Mods mit ZombieBuddy brauchen auch ZombieBuddys `zbNative.dll` und `ZombieBuddy.jar` in diesem Ordner: Kopiere sie aus dem Ordner des Spiels (ZombieBuddy legt sie dort ab, wenn das Spiel damit startet). Bis sie da sind, startet der Server ohne seinen Agenten, und **Mods & Abgleich** sagt das.

**Java-Mods (ZombieBuddy).** Mods für Build 42 mit Java-Code brauchen ZombieBuddy, und ZombieBuddy braucht seinen Agenten in der Befehlszeile des Servers. Hat das `Mods=` des Servers ZombieBuddy (auch als `\ZombieBuddy` geschrieben), startet das Studio ihn mit `-agentlib:zbNative=policy=deny-new`, und ein Hinweis auf diesem Reiter sagt das. *deny-new* bedeutet, dass der Server nur die Jars lädt, die dieser PC im Fenster von ZombieBuddy schon erlaubt hat (sie stehen in `.zombie_buddy/mod_approvals.json` in deinem Benutzerordner), und den Rest ohne Nachfrage ablehnt: Ein Server hat kein Fenster, und die Frage würde ihn für immer anhalten. **Mods & Abgleich** sagt, welche Jars noch nicht erlaubt sind.

## Neustarts

PZ Mod Studio kann den Server für dich neu starten und die Spieler vorher im Spiel warnen. Ein Neustart schickt die Warnungen über RCON (`servermsg`), dann `save`, dann `quit` (der Server speichert die Welt beim Beenden; er wird nur dann mit Gewalt geschlossen, wenn er nicht innerhalb von 90 Sekunden geht), und startet ihn auf dieselbe Weise neu, wie er gestartet wurde (gleicher Speicher, Steam oder NoSteam, Programm).

- **Geplante Neustarts**: zu Tageszeiten (Uhr dieses PCs) und/oder **Alle** 2 bis 24 Stunden ab dem Serverstart. Eine Zeit weniger als 30 Minuten nach dem Serverstart wird übersprungen.
- **Spieler vorher warnen**: 60, 30, 15, 10, 5 und 1 Minute, die, die du anhakst (anfangs 30, 15, 5 und 1). Die Nachrichten sind in der Sprache der App.
- **Neu starten, wenn eine Workshop-Mod dieses Servers aktualisiert wird**: Alle 15 Minuten fragt das Studio Steam (seine öffentliche Web-API), wann jeder Artikel aus `WorkshopItems=` des Servers zuletzt aktualisiert wurde. Ist einer nach dem Serverstart aktualisiert worden, gibt es einen Neustart mit denselben Warnungen, und der Server lädt das Update beim Start herunter.
- **Wieder starten, wenn er sich von selbst beendet** (anfangs an): nach einem Absturz, wenn er schon hochgefahren war (ein Server, der beim Starten scheitert, würde wieder scheitern, also wird er nicht neu gestartet). Höchstens 3-mal in 10 Minuten; danach bleibt er aus, und der Reiter sagt das in Rot. Ein Start von Hand zählt wieder ab null. Ein Stopp, den du angefordert hast (Stoppen, Neustart, ein Assistent, Discord), oder ein `quit` eines Admins im Spiel wird nie für einen Absturz gehalten.

Oben im Reiter steht, was als Nächstes kommt: der nächste geplante Neustart oder der, der gerade herunterzählt, mit wann und warum. **Jetzt neu starten (Spieler warnen)** startet einen Countdown mit den Minuten, die du wählst (**keine Warnung (sofort)** startet sofort neu); **Abblasen** stoppt ihn und sagt es den Spielern. **Letzte Neustarts** listet, was passiert ist und warum: Geplant, Mod-Update, Angefordert, Nach einem Absturz, Aus gelassen: zu viele Abstürze, Von selbst gestoppt, Gestoppt, während die App geschlossen war.

**Nur solange die App offen ist.** Die Neustarts macht PZ Mod Studio. Der Server läuft weiter, wenn du die App schließt, aber dann startet ihn nichts neu, warnt niemanden und startet ihn nach einem Absturz nicht. Der Zeitplan wird gespeichert (`Zomboid/PZModStudio_ServerSchedule.json`) und geht weiter, wenn du die App wieder öffnest; ein Server, der bei geschlossener App gestoppt ist, wird vermerkt, nicht neu gestartet.

## Mods & Abgleich

Das vergleicht deine aktiven Mods mit dem, was die `.ini` des Servers hat. Ein Server liest deine Mod-Liste nicht: Er braucht drei Zeilen in seiner `.ini`, `Mods=`, `WorkshopItems=` und `Map=`, und alle drei müssen übereinstimmen, sonst spielt der Server still ohne die Mods. Das Studio prüft sie und listet, was falsch ist (eine nie heruntergeladene Mod, ein nie geladener Artikel, eine ignorierte Karte).

**Meine Liste in diesen Server schreiben (Mods, WorkshopItems und Map)** schreibt alle drei Zeilen aus deinen aktiven Mods, in Ladereihenfolge, und lässt jede andere Einstellung, wie sie war. Die vorherige Datei wird in `Zomboid/PZModStudio_Backups` gesichert.

Ein Paket von PZ Mod Studio (`Z_PZModStudio_*`), das an ist, kommt auch in `Mods=`, aber es ist eine lokale Mod, kein Workshop-Artikel: Der Server auf diesem PC hat es, Spieler bekommen es nicht per Download, und das Spiel lässt sie ohne es nicht beitreten. Teile das Paket mit ihnen oder schalte es aus, bevor du die Liste schreibst.

Dieselbe Prüfung liest, was der Server mit **Java-Mods** machen wird: Ein Jar, das ZombieBuddy auf diesem PC noch nicht erlaubt hat (neu oder seitdem geändert: ein Mod-Update oder ein Neubau ist ein neues Jar), wird vom Server abgelehnt. Starte das Spiel einmal mit dieser Mod an, erlaube sie im Fenster von ZombieBuddy, und starte dann den Server. Ein Jar, das ZombieBuddy auf deinen Wunsch ablehnen soll, und Java-Mods in einer Liste ohne ZombieBuddy (nur ihr Lua lädt) werden auch genannt.

Ist unter **Hosten & Starten** **NoSteam (LAN)** gewählt, listet es auch die Mods, die nur im Workshop-Ordner von Steam sind. Ein ohne Steam gestarteter Server liest nur `Zomboid/mods`: Er meldet diese als "not found" und lässt auch jede Mod weg, die sie benötigt. **N Workshop-Mod(s) nach Zomboid/mods kopieren** kopiert den Workshop-Ordner jeder einzelnen dorthin. Eine Kopie, kein Link: Mit einigen Workshop-Mods, die in `Zomboid/mods` verlinkt sind (eine Verzeichnisverbindung), stürzt ein NoSteam-Server beim Erstellen der Welt ab, also wird auch ein Link auf einen Workshop-Ordner markiert, und derselbe Knopf ersetzt ihn durch eine Kopie. Eine Kopie gewinnt auch in deinem eigenen Spiel gegen den Workshop-Artikel. Wird der Workshop-Artikel aktualisiert, nachdem das Studio ihn kopiert hat, wird die Kopie als veraltet markiert, und der Knopf kopiert sie erneut. Ein gleichnamiger Ordner, den nicht das Studio dort abgelegt hat (deine eigene Kopie), wird nie ersetzt.

**Funktioniert diese Liste im Mehrspieler?** (**Prüfen**) gibt ein Urteil für die Liste und eins pro Mod. Es liest, was die Beschreibung jeder Mod über Mehrspieler sagt, und listet Aufrufe, die der Client-Code einer Mod direkt macht, wo das Spiel sie über den Server leitet; im Mehrspieler ändern sie nur die Welt dieses Spielers oder werden abgelehnt.

## Spieler

**Verbundene Spieler** listet, wer online ist, aus dem eigenen Log des Servers gelesen, mit Rolle, Ping, Gesundheit und Position. Während der Server läuft, spricht das Studio über RCON mit ihm (`RCONPassword` und `RCONPort` seiner `.ini`; ein Server, den das Studio ohne Passwort startet, bekommt eins):

- **Kicken** in der Zeile eines Spielers trennt ihn; er kann wieder beitreten. Du kannst einen Grund angeben, den er sieht.
- **Bannen** sperrt ihn von diesem Server, mit optionalem Grund. Um einen Bann zurückzunehmen, nutze die Serverkonsole: `unbanuser "name"`.
- Das Feld oben schickt eine Nachricht an alle im Spiel (**An alle senden** oder Enter); sie erscheint auf ihrem Bildschirm.

Die Antwort des Servers erscheint unter dem Feld.

## Schnell-.ini

Bearbeite die wichtigsten Einstellungen und drück **Einstellungen in .ini speichern**: **Öffentlicher Servername**, **Server-Passwort (Beitritt)** (leer für einen offenen Server), **Admin-Passwort (-adminpassword)**, **Max. Spieler**, **Standard-Port**, **Öffentliche Beschreibung**, **PVP-Kampf** und **Pausieren, wenn leer**.

## Alle Optionen

Jede Server- und Sandbox-Option, so wie das Spiel sie zeigt: nach Seite, mit ihrer Hilfe, ihrem Standardwert und den Sandbox-Optionen deiner Mods. **Server** und **Sandbox** wechseln die Liste, die Suche findet eine Option nach Name oder Hilfe, und **Nur die nicht auf Standard** zeigt, was du geändert hast. Der Pfeil einer Option setzt sie auf ihren Standardwert zurück. **Speichern** schreibt nur, was du geändert hast, beide Dateien vorher gesichert (nach `PZModStudio_Backups/server_settings`), und lehnt ab, solange dieser Server läuft. Der Studio Link merkt sich die Liste der Optionen, wenn du das Spiel zum ersten Mal bis ins Hauptmenü öffnest (und wieder, wenn sich der Build, die Sprache oder deine Mods ändern).

## Discord

Ein Discord-Webhook lässt den Server in einen Kanal posten: wenn er hochgefahren, gestoppt, abgestürzt oder nach einem Neustart zurück ist, wer beitritt und geht, und wenn seine Workshop-Mods aktualisiert wurden und er einen Neustart braucht. Niemand wird angepingt.

1. Öffne in Discord die Einstellungen des Kanals, Integrationen, Webhooks, Neuer Webhook, und kopiere seine URL.
2. Füge sie unter **Webhook-Adresse** ein und drück **Speichern**. Sie wird in der Windows-Anmeldeinformationsverwaltung gespeichert und nie wieder angezeigt.
3. Drück **Test senden** und schau in den Kanal.
4. Hak **Neuigkeiten dieses Servers in den Kanal posten** an und wähle die Ereignisse: **Läuft, gestoppt und abgestürzt**, **Spieler kommen und gehen**, **Neustarts**, **Workshop-Mods, die auf einen Neustart warten**.

**Vergessen** entfernt die Adresse von diesem PC. Jeder Server hat seinen eigenen Webhook.

### Mit dem Spiel chatten: der eigene Bot des Servers

Der dedizierte Server von Project Zomboid hat einen eigenen Discord-Bot. Das Studio richtet ihn in der `.ini` des Servers ein; er lebt im Server, also funktioniert er immer, wenn der Server läuft, auch bei geschlossenem Studio.

1. Im Discord Developer Portal: **New Application**, dann **Bot**: **Reset Token** und kopieren, und **Message Content Intent** einschalten.
2. Füge unter **Mit dem Spiel chatten** das **Bot-Token** ein, schreib den **Chat-Kanal** (sein Name, wie er in Discord ist, ohne `#`) und, wenn du willst, den **Log-Kanal**, hak **Bot einschalten** an und drück **In der .ini speichern**.
3. Drück **Bot auf deinen Discord einladen**, wähle deinen Discord-Server und starte den Spielserver (neu).

Der allgemeine Chat im Spiel geht dann in diesen Kanal, und was dort geschrieben wird, kommt ins Spiel. Das Spiel liest das Token aus der `.ini`, also steht es dort: Wer diese Datei hat, kann den Bot benutzen. Hat die `.ini` einen Befehlskanal, warnt dich der Bildschirm: Was immer dort jemand schreibt, läuft als Admin-Konsolenbefehl; **Leeren**, außer dieser Kanal gehört nur dir.

Eine **Statusnachricht, die sich selbst bearbeitet** (standardmäßig an), hält eine Nachricht im Kanal, die sagt, ob der Server läuft, seit wann und wer drin ist, statt jedes Mal eine neue zu posten.

### Befehle aus Discord (solange das Studio offen ist)

Aus, bis du sie einschaltest. Mit dem oben eingerichteten Bot schreibst du einen **Befehlskanal** und die **Rolle, die sie nutzen darf** (Namen, wie sie in Discord sind), und hakst **Befehle einschalten** an. Das Studio liest diesen Kanal alle 5 Sekunden und antwortet dort; Mitgliedern ohne die Rolle wird gesagt, dass sie es nicht dürfen.

- `!status` und `!players`: ob der Server läuft und wer drin ist.
- `!restart [minutes]` (1 bis 60, 5 wenn nicht angegeben): Die Spieler werden dann im Spiel gewarnt, 5 und 1 Minute vorher; dann wird der Server auf die übliche Weise gestoppt (er speichert) und neu gestartet. `!restart cancel` stoppt es.
- `!save` speichert die Welt; `!msg <text>` sagt allen im Spiel etwas.
- `!help` listet sie auf. Sie funktionieren auch auf Spanisch (`!estado`, `!jugadores`, `!reiniciar`, `!guardar`, `!mensaje`).

Sie sprechen über RCON mit dem Server: Ein Server, den das Spiel erstellt hat, hat kein RCON-Passwort, also gibt ihm das Studio beim nächsten Start ein zufälliges (die `.ini` wird vorher gesichert); damit kann **Stoppen** auch vor dem Beenden speichern. Sie funktionieren nur, solange PZ Mod Studio offen ist, und führen nie einen von Hand getippten Konsolenbefehl aus. Nimm einen Kanal, den nur Admins sehen: Der Chat-Kanal geht auch ins Spiel.

## Gut zu wissen

- In Discord gepostet wird nur, solange PZ Mod Studio offen ist und den Server hostet.
- Zum Starten eines Servers muss der Spielordner in den **App-Einstellungen** gesetzt sein.
- Beitreten startet das Spiel: Schließ es vorher.
- Um von diesem PC deinem eigenen Server beizutreten, nutze seine LAN-Adresse (zum Beispiel `192.168.0.10:16261`): Ein Steam-Client kann `127.0.0.1` oder der eigenen öffentlichen IP dieses PCs nicht beitreten. Ein mit **NoSteam (LAN)** gestarteter Server braucht ein mit `-nosteam` gestartetes Spiel (dort funktioniert `127.0.0.1`), und ein mit `-debug` gestartetes Spiel darf nur als Admin beitreten.
- Mit dem Studio verbundene Assistenten (MCP) können dasselbe: `server_start`, `server_status`, `server_stop`, `server_join`, `server_schedule`, `server_restart` und `dedicated_server`.
