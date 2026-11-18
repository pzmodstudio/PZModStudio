# Serveurs

L'écran Serveurs a trois onglets en haut : **Serveurs publics** et **Mes serveurs**, pour trouver un serveur multijoueur et le rejoindre, et **Héberger un serveur**, pour faire tourner votre propre serveur dédié depuis ce PC. Sous **Héberger un serveur**, une barre dit si votre serveur est en marche, avec **Voir comment il tourne** (ouvre Diagnostic > Serveur : mémoire, CPU, s'il a démarré et ce que dit son journal), **Partager l'invitation** et le bouton qui relit les fichiers du serveur.

## Rejoindre un serveur : en trouver un

Sous **Serveurs publics**, appuyez sur **Chercher des serveurs**. Le Studio demande à Steam chaque serveur Project Zomboid, comme le fait le navigateur de serveurs du jeu, via le client Steam lancé sur ce PC (pas de clé nécessaire ; Steam vous affiche dans Project Zomboid pendant la demande). Les serveurs apparaissent au fur et à mesure qu'ils répondent ; tous prennent environ une minute et demie. **Arrêter** met fin à la recherche, **Chercher à nouveau** recommence.

Réduisez la liste avec le champ (**Nom ou carte**) et les filtres : **Seulement ma build**, **Avec des joueurs**, **Pas plein**, **Sans mot de passe**, **Moddés et vanilla** / **Moddés** / **Vanilla**, et triez par **Plus de joueurs**, **Ping le plus bas** ou **Nom**. Faites un clic droit sur un serveur pour un menu : **Voir ce qu'il dit**, **Rejoindre**, **Mettre en favori**, **Copier l'adresse**, créer un profil avec ses mods et, pour un serveur de **Mes serveurs**, **Oublier ce serveur**.

**Mes serveurs** contient les serveurs que vous mettez en favori (**Favoris**) et ceux que vous avez rejoints depuis ici (**Rejoints depuis ici**). Pour ajouter un serveur privé ou celui d'un ami, tapez son adresse sous **Ajouter un serveur par son adresse** (`ip:port` ; port 16261 s'il est omis), un nom facultatif et le mot de passe, et appuyez sur **Ajouter**. Le mot de passe est gardé dans le magasin d'identifiants de Windows, jamais dans un fichier.

## Rejoindre un serveur : se connecter

1. Cliquez sur un serveur. Le panneau de droite montre sa build, s'il est ouvert ou sur liste blanche uniquement, son dernier wipe et ses mods. Chaque mod est coloré : **Installé et activé**, **Installé**, ou **Le jeu le télécharge**.
2. Si le serveur tourne sur une autre build que la vôtre, un avertissement le dit : le jeu ne vous laissera pas entrer tant que les deux ne correspondent pas (branche bêta de Steam).
3. Tapez le mot de passe du serveur s'il en a un, et cochez **Retenir le mot de passe du serveur** pour le garder. Un mot de passe gardé s'affiche comme une ligne verte (**Mot de passe du serveur gardé dans Windows**) à la place du champ : le Studio le remplit quand vous rejoignez. **Modifier** ouvre le champ pour en mettre un nouveau et **Oublier** le supprime. Le mot de passe de votre compte se tape dans la fenêtre de connexion du jeu.
4. Fermez le jeu s'il est ouvert, et appuyez sur **Rejoindre**.

Le jeu démarre sur la fenêtre de connexion de ce serveur avec le mot de passe du serveur rempli, et votre nom d'utilisateur aussi si vous l'avez tapé dans **Votre nom d'utilisateur sur ce serveur** (le Studio le garde ; le mod Studio Link le remplit). Là, vous tapez le mot de passe de votre compte et appuyez sur **SE CONNECTER**, qui reste désactivé sans nom d'utilisateur. Un nouveau nom d'utilisateur crée un nouveau compte sur ce serveur. Le jeu télécharge les mods du serveur et ne les utilise que là : votre propre liste de mods reste telle quelle ; le profil avec lequel vous rejoignez les reçoit (voir plus bas).

Avec le mod Studio Link, le serveur va aussi dans les **Favoris** du jeu (l'onglet Favoris de son écran Multijoueur), avec votre nom d'utilisateur en dessous, comme si vous l'aviez ajouté avec **Ajouter un serveur** et **Ajouter un nouveau compte**. Ainsi, si vous êtes déconnecté, il est là pour le rejoindre à nouveau sans fermer le jeu. Il n'est jamais ajouté deux fois, et le mot de passe de votre compte n'est pas gardé ; celui du serveur l'est, comme le jeu le garde.

## Créer un profil pour un serveur

Un serveur n'annonce que ses premiers mods (Steam coupe la liste). Après l'avoir rejoint une fois depuis ici, le Studio connaît la liste complète, et le bouton devient **Créer un profil avec ces N mods**. Avant cela, **Créer un profil avec les N mods qu'il annonce** utilise la liste partielle. Le profil s'appelle « Server: (nom) » et garde l'ordre du serveur. Les mods qui ne sont pas installés maintenant sont signalés avant la création du profil.

Pour jouer avec, activez-le dans Profils avant de rejoindre. Chaque fois que vous rejoignez depuis ici, une fois que le jeu a chargé les mods du serveur, ils vont dans le profil actif au moment où vous avez rejoint : activés, dans l'ordre du serveur, avec ceux que le jeu vous a fait télécharger. Ses autres mods y restent, désactivés, sauf ceux qui ne tournent que de votre côté (ZombieBuddy et ses mods Java côté client uniquement, comme Viewpoint) : ils restent comme vous les aviez. Le panneau du serveur dit de quel profil il s'agit.

## Héberger un serveur : la liste des serveurs

À gauche, **Configurations de serveur (.ini)** liste les configurations de serveur dans `Zomboid/Server`, avec leur nombre de mods et d'objets Workshop et **En cours** pour celle qui tourne. Tapez un nom et appuyez sur **Créer** pour en faire une nouvelle, ou **⚡ Créer la configuration par défaut (servertest.ini)** quand il n'y en a aucune. La corbeille supprime une configuration du disque (on vous demande d'abord). Choisissez-en une pour voir ses onglets. Le bandeau du haut affiche **EN LIGNE** ou **HORS LIGNE**, **Partager l'invitation** (copie un texte d'invitation avec le nom, le port, le nombre max de joueurs, le mot de passe et le nombre de mods ; vous complétez votre IP publique et ouvrez le port) et un bouton pour recharger les fichiers du serveur.

## Hôte et lancement

- **Mémoire (-Xmx)** : 4, 8, 12 ou 16 Go pour le serveur.
- **Mot de passe admin** : définit le mot de passe du compte `admin` au démarrage du serveur. Le Studio le garde dans le Gestionnaire d'identification de Windows (jamais dans un fichier), pour qu'un assistant connecté au Studio puisse redémarrer ce serveur sans qu'on le lui donne.
- **Authentification et réseau** : **Steam activé** ou **NoSteam (LAN)**.
- **Lancer le serveur dédié (nom)** le démarre dans sa propre fenêtre de console. Pendant qu'il tourne, **Arrêter le serveur dédié** arrête le serveur que le Studio a lancé (jamais d'autres programmes Java), et **Redémarrer** l'arrête et le relance aussitôt (pour prévenir d'abord les joueurs, utilisez l'onglet **Redémarrages**).

**Programme du serveur.** Un serveur tourne à partir des fichiers du jeu lui-même, sauf si vous choisissez autrement. Le cadre **Programme du serveur** propose à la place le **Project Zomboid Dedicated Server** officiel : un programme à part que Steam donne gratuitement (app 380870), qui peut être sur une autre branche que votre jeu et se met à jour séparément.

1. Choisissez un **Dossier du serveur dédié** : un dossier vide ou nouveau sur un disque avec de la place (pas le dossier du jeu, pas `Zomboid`). Un serveur dédié déjà présent sur ce PC (depuis Bibliothèque > Outils de Steam, ou un SteamCMD précédent) est trouvé et proposé.
2. Choisissez la **Branche** : **Stable (public)** ou **Instable (bêta)**. Elle doit correspondre au jeu de ceux qui rejoignent ; le cadre dit sur quelle branche est votre jeu et commence sur celle-là. Une branche que Steam protège par un mot de passe ne peut pas être installée : le cadre le dit.
3. Appuyez sur **Installer**. Le Studio utilise SteamCMD, le Steam en ligne de commande de Valve. S'il n'est pas sur ce PC, il demande d'abord, puis le télécharge depuis l'adresse de Valve elle-même (environ 3 Mo), vérifie que Valve l'a signé, et le laisse se mettre à jour. Ensuite SteamCMD télécharge le serveur depuis Steam (plusieurs Go) : le cadre montre chaque étape avec son pourcentage et ses gigaoctets (environ 7 Go pour 42.22), et la taille du dossier une fois terminé. **Arrêter** l'interrompt ; appuyer de nouveau sur Installer reprend là où il en était.
4. Une fois installé, choisissez **Le serveur dédié**. Les serveurs démarrent alors depuis ce dossier, avec le même dossier Zomboid (vos configurations de serveur, sauvegardes et `Zomboid/mods`). **Les fichiers du jeu** revient à la façon habituelle. Changez-le avec le serveur arrêté.

**Mettre à jour** relance SteamCMD sur ce dossier et cette branche (en changeant de branche si vous l'avez changée). Sur une installation complète, il ne récupère que ce qui a changé et finit en quelques secondes (« Déjà à jour » quand rien n'a changé) ; après une installation interrompue, ou une vérification des fichiers arrêtée, il vérifie chaque fichier, ce qui prend de nombreuses minutes. Le cadre dit aussi la version du jeu sous laquelle le serveur démarre et si c'est celle de votre jeu : un jeu ne rejoint qu'un serveur de sa propre version.

Les mods Java avec ZombieBuddy ont aussi besoin des fichiers `zbNative.dll` et `ZombieBuddy.jar` de ZombieBuddy dans ce dossier : copiez-les depuis le dossier du jeu (le bouton **Copier les fichiers de ZombieBuddy vers le serveur dédié** de **Mods et synchro** le fait) (ZombieBuddy les y met quand le jeu démarre avec lui). Tant qu'ils n'y sont pas, le serveur démarre sans son agent et **Mods et synchro** le dit.

**Mods Java (ZombieBuddy).** Les mods Build 42 avec du code Java ont besoin de ZombieBuddy, et ZombieBuddy a besoin de son agent dans la ligne de commande du serveur. Quand le `Mods=` du serveur contient ZombieBuddy (écrit aussi `\ZombieBuddy`), le Studio le démarre avec `-agentlib:zbNative=policy=deny-new`, et une note dans cet onglet le dit. *deny-new* veut dire que le serveur ne charge que les jars que ce PC a déjà approuvés dans la fenêtre de ZombieBuddy (ils sont gardés dans `.zombie_buddy/mod_approvals.json` dans votre dossier utilisateur) et refuse le reste sans demander : un serveur n'a pas de fenêtre, et la question le bloquerait pour de bon. **Mods et synchro** dit quels jars ne sont pas encore approuvés.

## Redémarrages

PZ Mod Studio peut redémarrer le serveur pour vous, en prévenant d'abord les joueurs dans le jeu. Un redémarrage envoie les avertissements via RCON (`servermsg`), puis `save`, puis `quit` (le serveur sauvegarde le monde en quittant ; il n'est fermé de force que s'il ne s'arrête pas dans les 90 secondes), et le relance de la même façon qu'il avait été lancé (même mémoire, Steam ou NoSteam, programme).

- **Redémarrages programmés** : à des heures de la journée (l'horloge de ce PC) et/ou **Toutes les** 2 à 24 heures à partir du démarrage du serveur. Une heure moins de 30 minutes après le démarrage du serveur est sautée.
- **Prévenir les joueurs avant** : 60, 30, 15, 10, 5 et 1 minutes, celles que vous cochez (30, 15, 5 et 1 au départ). Les messages sont dans la langue de l'app.
- **Redémarrer quand un mod du Workshop de ce serveur est mis à jour** : toutes les 15 minutes, le Studio demande à Steam (son Web API publique) quand chaque objet du `WorkshopItems=` du serveur a été mis à jour pour la dernière fois. Un objet mis à jour après le démarrage du serveur déclenche un redémarrage avec les mêmes avertissements, et le serveur télécharge la mise à jour en démarrant.
- **Le redémarrer s'il s'arrête tout seul** (activé au départ) : après un crash, s'il avait démarré (un serveur qui échoue pendant son démarrage échouerait de nouveau, il n'est donc pas relancé). Au plus 3 fois en 10 minutes ; après cela, il reste éteint et l'onglet le dit en rouge. Le lancer à la main remet le compte à zéro. Un arrêt que vous avez demandé (Arrêter, Redémarrer, un assistant, Discord) ou un `quit` d'un admin dans le jeu n'est jamais pris pour un crash.

Le haut de l'onglet dit ce qui arrive : le prochain redémarrage programmé, ou celui en cours de compte à rebours avec quand et pourquoi. **Redémarrer maintenant (prévenir les joueurs)** lance un compte à rebours du nombre de minutes choisi (**sans avertissement (maintenant)** redémarre tout de suite) ; **Annuler** l'arrête et prévient les joueurs. **Derniers redémarrages** liste ce qui s'est passé et pourquoi : Programmé, Mise à jour de mod, Demandé, Après un crash, Laissé éteint : trop de crashs, Arrêté tout seul, Arrêté pendant que l'app était fermée.

**Seulement quand l'app est ouverte.** Les redémarrages sont faits par PZ Mod Studio. Le serveur continue de tourner si vous fermez l'app, mais alors plus rien ne le redémarre, ne prévient personne ni ne le relance après un crash. Le planning est enregistré (`Zomboid/PZModStudio_ServerSchedule.json`) et reprend quand vous rouvrez l'app ; un serveur qui s'est arrêté pendant que l'app était fermée est noté, pas relancé.

## Mods et synchro

Cela compare vos mods actifs avec ce que contient le `.ini` du serveur. Un serveur ne lit pas votre liste de mods : il a besoin de trois lignes dans son `.ini`, `Mods=`, `WorkshopItems=` et `Map=`, et les trois doivent concorder, sinon le serveur tourne discrètement sans les mods. Le Studio les vérifie et liste ce qui ne va pas (un mod jamais téléchargé, un objet jamais chargé, une carte ignorée).

**Écrire ma liste dans ce serveur (Mods, WorkshopItems et Map)** écrit les trois lignes à partir de vos mods actifs, dans l'ordre de chargement, et garde tous les autres réglages tels quels. Le fichier précédent est enregistré dans `Zomboid/PZModStudio_Backups`.

Un paquet PZ Mod Studio (`Z_PZModStudio_*`) activé va aussi dans `Mods=`, mais c'est un mod local, pas un objet du Workshop : le serveur de ce PC l'a, les joueurs ne l'obtiennent pas en téléchargeant, et le jeu ne les laisse pas rejoindre sans lui. Partagez le paquet avec eux, ou désactivez-le avant d'écrire la liste.

La même vérification lit ce que le serveur fera des **mods Java** : un jar que ZombieBuddy n'a pas encore approuvé sur ce PC (nouveau, ou modifié depuis : une mise à jour du mod ou une recompilation donne un nouveau jar) est refusé par le serveur. Lancez une fois le jeu avec ce mod activé, autorisez-le dans la fenêtre de ZombieBuddy, puis lancez le serveur. Un jar que vous avez dit à ZombieBuddy de refuser, et les mods Java dans une liste sans ZombieBuddy (seul leur Lua se charge), sont signalés aussi.

Avec **NoSteam (LAN)** choisi dans **Hôte et lancement**, cela liste aussi les mods qui ne sont que dans le dossier du Workshop de Steam. Un serveur lancé sans Steam ne lit que `Zomboid/mods` : il les signale « not found », et laisse aussi de côté tous les mods qui les nécessitent. **Copier N mod(s) Workshop dans Zomboid/mods** y copie le dossier Workshop de chacun. Une copie, pas un lien : avec certains mods du Workshop liés dans `Zomboid/mods` (une jonction de répertoire), un serveur NoSteam plante pendant la création du monde, donc un lien vers un dossier du Workshop est signalé aussi et le même bouton le remplace par une copie. Une copie l'emporte aussi sur l'objet du Workshop dans votre propre jeu. Quand l'objet du Workshop est mis à jour après la copie par le Studio, la copie est signalée comme périmée et le bouton la recopie. Un dossier du même nom que le Studio n'a pas mis là (votre propre copie) n'est jamais remplacé.

**Cette liste fonctionne-t-elle en multijoueur ?** (**Vérifier**) donne un verdict pour la liste et un par mod. Il lit ce que la description de chaque mod dit du multijoueur, et liste les appels que le code client d'un mod fait directement là où le jeu les fait passer par le serveur ; en multijoueur, ils ne changent que le monde de ce joueur, ou sont refusés.

## Joueurs

**Joueurs connectés** liste qui est en ligne, d'après le journal du serveur lui-même, avec rôle, Steam id, où il est entré et s'il est mort (les journaux du jeu n'ont ni ping ni santé). Pendant que le serveur tourne, le Studio lui parle via RCON (le `RCONPassword` et le `RCONPort` de son `.ini` ; un serveur que le Studio lance sans mot de passe en reçoit un) :

- **Expulser** sur la ligne d'un joueur le déconnecte ; il peut rejoindre de nouveau. Vous pouvez donner une raison, qu'il voit.
- **Bannir** le bannit de ce serveur, avec une raison facultative. Pour lever un bannissement, utilisez la console du serveur : `unbanuser "name"`.
- Le champ en haut envoie un message à tout le monde dans le jeu (**Envoyer à tous**, ou Entrée) ; il s'affiche sur leur écran.

La réponse du serveur apparaît sous le champ.

## .ini rapide

Modifiez les réglages principaux et appuyez sur **Enregistrer les réglages dans le .ini** : **Nom public du serveur**, **Mot de passe du serveur (connexion)** (vide pour un serveur ouvert), **Mot de passe admin (-adminpassword)**, **Joueurs max**, **Port par défaut**, **Description publique**, **Combat JcJ** et **Pause quand vide**.

## Toutes les options

Chaque option du serveur et du bac à sable, comme le jeu les affiche : par page, avec son aide, sa valeur par défaut et les options de bac à sable de vos mods. **Serveur** et **Bac à sable** changent de liste, la recherche trouve une option par nom ou par aide, et **Seulement celles qui ne sont pas par défaut** montre ce que vous avez changé. La flèche d'une option la remet à sa valeur par défaut. **Enregistrer** n'écrit que ce que vous avez changé, les deux fichiers étant d'abord copiés (dans `PZModStudio_Backups/server_settings`), et refuse pendant que ce serveur tourne. Le Studio Link note la liste des options la première fois que vous ouvrez le jeu jusqu'au menu principal (et de nouveau quand la build, la langue ou vos mods changent).

## Discord

Un webhook Discord permet au serveur de publier dans un salon : quand il est en ligne, arrêté, planté ou de retour après un redémarrage, qui arrive et qui part, et quand ses mods du Workshop ont été mis à jour et qu'il a besoin d'un redémarrage. Personne n'est mentionné.

1. Dans Discord, ouvrez les paramètres du salon, Intégrations, Webhooks, Nouveau webhook, et copiez son URL.
2. Collez-la sous **Adresse du webhook** et appuyez sur **Enregistrer**. Elle est gardée dans le Gestionnaire d'identification de Windows et n'est plus jamais affichée.
3. Appuyez sur **Envoyer un test** et regardez le salon.
4. Cochez **Publier les nouvelles de ce serveur dans le salon** et choisissez les événements : **En ligne, arrêté et planté**, **Joueurs qui arrivent et qui partent**, **Redémarrages**, **Mods Workshop en attente d'un redémarrage**.

**L'oublier** retire l'adresse de ce PC. Chaque serveur a son propre webhook.

### Discuter avec le jeu : le bot du serveur

Le serveur dédié de Project Zomboid a son propre bot Discord. Le Studio le configure dans le `.ini` du serveur ; il vit dans le serveur, donc il fonctionne dès que le serveur tourne, même avec le Studio fermé.

1. Dans le Discord Developer Portal : **New Application**, puis **Bot** : **Reset Token** et copiez-le, et activez **Message Content Intent**.
2. Dans **Discuter avec le jeu**, collez le **Token du bot**, écrivez le **Salon du chat** (son nom tel qu'il est dans Discord, sans `#`) et, si vous en voulez un, le **Salon du journal**, cochez **Activer le bot** et appuyez sur **Enregistrer dans le .ini**.
3. Appuyez sur **Inviter le bot sur votre Discord**, choisissez votre serveur Discord, et lancez (ou relancez) le serveur de jeu.

Le chat général du jeu va alors dans ce salon, et ce qui y est écrit arrive dans le jeu. Le jeu lit le token dans le `.ini`, il y est donc écrit : quiconque a ce fichier peut utiliser le bot. Si le `.ini` a un salon de commandes, l'écran vous avertit : tout ce que n'importe qui y écrit s'exécute comme commande de console admin ; **Le vider** sauf si ce salon n'est qu'à vous.

Un **message d'état qui se met à jour tout seul** (activé par défaut) garde un seul message dans le salon qui dit si le serveur est en ligne, depuis quand, et qui est connecté, au lieu d'en publier un nouveau à chaque fois.

### Commandes depuis Discord (quand le Studio est ouvert)

Désactivées sauf si vous les activez. Avec le bot ci-dessus configuré, écrivez un **Salon des commandes** et le **Rôle autorisé à les utiliser** (noms tels qu'ils sont dans Discord) et cochez **Activer les commandes**. Le Studio lit ce salon toutes les 5 secondes et y répond ; les membres sans le rôle se voient répondre qu'ils ne peuvent pas.

- `!status` et `!players` : si le serveur est en ligne et qui est connecté.
- `!restart [minutes]` (1 à 60, 5 si rien n'est dit) : les joueurs sont prévenus dans le jeu à ce moment-là, puis 5 et 1 minute avant ; ensuite le serveur est arrêté de la façon habituelle (il sauvegarde) et relancé. `!restart cancel` l'annule.
- `!save` sauvegarde le monde ; `!msg <text>` dit quelque chose à tout le monde dans le jeu.
- `!help` les liste. Elles fonctionnent aussi en espagnol (`!estado`, `!jugadores`, `!reiniciar`, `!guardar`, `!mensaje`).

Elles parlent au serveur via RCON : un serveur créé par le jeu n'a pas de mot de passe RCON, donc le Studio lui en donne un au hasard la prochaine fois qu'il le lance (le `.ini` est copié d'abord) ; cela permet aussi à **Arrêter** de sauvegarder avant de quitter. Elles ne fonctionnent que lorsque PZ Mod Studio est ouvert, et n'exécutent jamais une commande de console tapée à la main. Utilisez un salon que seuls les admins voient : le salon du chat va aussi dans le jeu.

## Bon à savoir

- La publication sur Discord ne se fait que lorsque PZ Mod Studio est ouvert et héberge le serveur.
- Lancer un serveur demande que le dossier du jeu soit défini dans **Paramètres de l'app**.
- Rejoindre lance le jeu : fermez-le d'abord.
- Pour rejoindre votre propre serveur depuis ce PC, utilisez son adresse LAN (par exemple `192.168.0.10:16261`) : un client Steam ne peut pas rejoindre `127.0.0.1` ni l'IP publique de ce PC. Un serveur lancé avec **NoSteam (LAN)** demande un jeu lancé avec `-nosteam` (là, `127.0.0.1` fonctionne), et un jeu lancé avec `-debug` ne peut rejoindre qu'en tant qu'admin.
- Les assistants connectés au Studio (MCP) peuvent faire la même chose : `server_start`, `server_status`, `server_stop`, `server_join`, `server_schedule`, `server_restart` et `dedicated_server`.
