# PZ Mod Studio

PZ Mod Studio est un gestionnaire de mods gratuit pour Project Zomboid sur Windows. Il range vos mods dans des profils, fait des copies de secours de vos sauvegardes, vérifie votre liste de mods comme le jeu la lit et vous aide à réparer ce qui casse. Appuyez sur « ? » sur n'importe quel écran pour ouvrir sa page d'aide.

La première fois que vous ouvrez le Studio, une visite guidée **Bienvenue** s'affiche : une carte en bas à droite qui passe par chaque écran, avec le vrai écran derrière et son bouton éclairé dans la barre latérale, et ce que le Studio sait déjà de votre installation (les dossiers trouvés, vos profils, les problèmes de votre liste). Avancez avec **Suivant** ou les flèches du clavier ; le X la ferme. Pour la revoir, ouvrez l'aide (« ? ») et cliquez sur **Revoir la visite guidée**.

## Les écrans

La barre latérale liste les écrans sous **Modules du Studio**. Les écrans avec un cadenas demandent d'abord un profil actif : créez-en un ou activez-en un dans Profils.

- **Profils** : des listes de mods nommées ; créez-les, passez de l'une à l'autre en un clic, partagez-les en fichiers `.pzpack`.
- **Sauvegardes** : vos sauvegardes et leurs copies de secours ; faites-en une, gardez-en une ou restaurez-en une.
- **Centre de surveillance** : lancez le jeu et lisez son journal, avec une fiche par erreur et le mod d'où elle vient.
- **Liste des mods** : activez et désactivez des mods et réglez l'ordre de chargement, avec filtres, groupes, notes et annulation.
- **Nouveautés** : vos mods du Workshop mis à jour depuis votre dernière partie, avec leurs notes de mise à jour et un avertissement quand une mise à jour peut casser des sauvegardes.
- **Diagnostic** : ce que l'app sait de vos mods (vérifiés comme le jeu les lit : prérequis manquants ou désactivés, mauvais ordre, mods incompatibles, mods que votre build ne peut pas charger, avec des correctifs), du jeu (mémoire, CPU, démarrage, images, disque) et du serveur qui tourne sur votre PC, avec le Docteur en première page.
- **Parcourir les mods** : cherchez dans le Workshop de Steam, abonnez-vous, tenez vos propres listes et installez des mods téléchargés ailleurs.
- **Cartes** : les cellules de chaque mod de carte sur le monde du jeu, et les endroits où deux mods de carte se disputent les mêmes cellules.
- **Fusion de mods** : quand deux mods modifient le même fichier, combinez-les dans un paquet de correctifs qui se charge en dernier.
- **Développeur** : pour les créateurs de mods : vérifiez les fichiers Lua et de script d'un mod avant le jeu, et lancez le jeu avec seulement ce mod activé. Il n'apparaît qu'avec **Fonctions pour développeurs** activé dans Paramètres de l'app.
- **Serveurs** : hébergez et gérez un serveur dédié et gardez ses mods alignés sur les vôtres.
- **Paramètres de l'app** : où se trouvent votre jeu et vos mods, quels avertissements vous voyez, et d'autres options.

## Lancer le jeu

Le bouton vert en haut à droite lance Project Zomboid avec votre liste de mods. La flèche à côté ouvre le menu :

- **Modes de lancement** : avec débogage (`-debug`, le Centre de surveillance suit le journal en direct) ou le jeu normal, chacun en plein écran ou fenêtré.
- **Destination** : **Menu principal**, comme toujours, ou **Personnalisé…**, qui ouvre **Lancement personnalisé** pour aller directement quelque part :
  - **Une sauvegarde** : vos sauvegardes solo, celles du profil actif en premier. Le jeu s'ouvre et, au menu principal, le mod Studio Link charge cette sauvegarde, avec les mêmes avis et la même copie de secours que **Continuer**. Vient ensuite l'écran noir « cliquez pour commencer » du jeu.
  - **Un serveur** : vos serveurs enregistrés. Rejoint comme **Rejoindre** dans Serveurs, avec le mot de passe enregistré et votre nom d'utilisateur ; le jeu charge les mods du serveur.
  - **Comment** choisit le mode de lancement. **Choisir** en fait la destination du bouton ; **Choisir et lancer** lance aussi le jeu. **Retour au menu principal** l'efface.

Avec une destination choisie, le bouton la nomme (**Jouer : (sauvegarde)** ou **Rejoindre : (serveur)**) et s'en souvient, comme du mode, tant que **Retenir comment le jeu se lance** est activé dans les Paramètres.

## Le mod Studio Link dans le jeu

PZ Mod Studio Link (`Z_StudioLink`) est un petit mod pour Build 42 que le Studio installe et garde activé dans chaque profil. C'est le côté jeu de l'app. Une partie fonctionne seule ; le reste a besoin de l'app.

### Son propre écran Mods

Le bouton **MODS** du menu principal ouvre l'écran Mods du Link au lieu de celui du jeu. Seul, il propose :

- tous les mods installés avec un champ de recherche (nom, id du mod, id Workshop, auteur) et des filtres (**Tous**, **Activés**, **Désactivés**, **Favoris**, **Mis à jour**, **Problèmes**, **Cartes**) ;
- les détails de chaque mod : ce qu'il nécessite et ce qui en a besoin, ses builds, sa page Workshop ;
- activer et désactiver des mods avec ce qu'ils nécessitent, plusieurs à la fois avec Ctrl ou Maj ;
- l'ordre de chargement : trié selon lui, en montant et descendant les mods ou en les glissant, jamais au-dessus de ce qu'un mod nécessite ;
- des groupes par auteur ou par catégorie, les options de chaque mod (**Options du mod**), l'**Ordre des cartes**, et **Annuler** / **Rétablir** ;
- **Fichiers partagés** : quand deux mods activés contiennent le même fichier Lua, le jeu n'exécute qu'une copie, celle du mod chargé plus tard. Le mod dont la copie est utilisée est marqué **Remplace**, celui dont la copie ne l'est pas **Remplacé** ; le bouton de résumé **N partagent des fichiers** les liste (son infobulle explique les marques) et les détails de chaque mod nomment les fichiers et l'autre mod. Vérifié sur les mods que le jeu a chargés, quelques secondes après l'apparition du menu principal ; seuls les fichiers Lua sont examinés ;
- **Recharger les mods** fait relire au jeu ses dossiers de mods, pour qu'un objet du Workshop auquel vous vous êtes abonné (ou qui a été mis à jour) pendant que le jeu tourne apparaisse sans redémarrer ; le jeu recharge son Lua, comme après **Accepter** ;
- **Lignes serveur** copie les mods activés sous forme de lignes `Mods=`, `WorkshopItems=` et `Map=` d'un serveur dédié, dans l'ordre de chargement (chaque mod après ce qu'il nécessite, les cartes dans l'ordre des cartes), à coller à la place de ces lignes dans le `.ini` du serveur, dans `Zomboid/Server` ; les mods sans objet Workshop sont nommés sur une ligne de commentaire, à copier dans le propre dossier de mods du serveur ;
- **Accepter** enregistre dans la liste que lit le jeu, comme le fait l'écran du jeu. **L'écran du jeu** ouvre l'écran d'origine.

Avec l'app, le même écran montre aussi uniquement les mods du profil (**Mods du profil** / **Tous les installés**), les avertissements du Diagnostic sur chaque mod, les mods mis à jour depuis votre dernière session (dans leurs détails, les dernières notes de mise à jour de leur auteur et si la mise à jour peut casser une sauvegarde), vos notes et groupes, et un bouton pour ajouter les nouveaux mods qui ne sont dans aucun profil. **Synchroniser avec PZ Mod Studio** reprend ce que l'app a modifié, et **Ouvrir PZ Mod Studio** met l'app au premier plan.

Si un autre gestionnaire de mods qui prend en charge l'écran Mods est activé, le Link vous demande quel écran vous voulez et le lui laisse si vous le dites.

### La carte du menu principal

Quand l'app a déjà tourné sur ce PC, une carte sur le menu principal montre si l'app est ouverte ou fermée, le profil actif et son nombre de mods, les erreurs et avertissements du Diagnostic, et **Continuer : (sauvegarde)** avec ce qui a changé dans ses mods depuis sa dernière session. Si l'app n'a jamais été installée, la carte vous invite plutôt à l'obtenir, avec **Télécharger** (copie l'adresse de la page de téléchargement) et **Masquer**.

### L'avis d'erreur

Quand le jeu enregistre une erreur Lua, un avis nomme le mod d'où elle vient (et quel mod l'a appelée), avec le dernier message. **Copier le rapport** met l'erreur dans le presse-papiers pour l'envoyer à l'auteur du mod, **Voir dans Mods** ouvre l'écran Mods sur ce mod, et **Voir dans le Studio** ouvre le Centre de surveillance sur sa fiche. L'avis reste silencieux quand un autre mod qui affiche déjà ces erreurs est activé. **Toutes (N)**, et **Erreurs Lua (N)** dans le menu pause, ouvrent toutes les erreurs de la session, chaque type une seule fois avec son nombre d'apparitions, son mod et fichier:ligne : un clic en copie une, **Tout copier** les copie toutes avec la build du jeu, le mode et le nombre de mods activés. Un rapport nomme chaque mod avec son id, sa version et son objet Workshop.

Chaque erreur va aussi dans `Zomboid/Lua/PZModStudio_LinkErrorLog.txt` moins d'une seconde après s'être produite, si bien qu'un plantage, une déconnexion ou un retour au menu ne la perd pas : chaque type une fois en entier (mod, fichier:ligne, message, pile d'appels) et ses répétitions comptées, sous un titre avec la date, la build et s'il s'agissait du menu ou de quelle sauvegarde, avec une ligne là où le jeu est revenu au menu principal ou là où le serveur l'a déconnecté. Le fichier garde environ les 3000 dernières lignes. **Journal des erreurs** dans la liste ouvre son dossier et copie son chemin.

Pour les auteurs de mods : `PZModStudioLink.registerReport("YourModId", fn)` (ou `PZModStudioLinkReports["YourModId"] = fn` depuis du Lua exécuté avant celui du Link) ajoute vos propres informations aux rapports. `fn(err)` renvoie une chaîne, une liste de lignes ou une table de paires nom = valeur ; elle ne s'exécute que lorsqu'un rapport est fait, dans un `pcall`. Son texte va dans **Tout copier**, et dans **Copier le rapport** et le journal des erreurs pour une erreur de votre mod. Les rapports de débogage des autres mods, ceux que les mods enregistrent déjà pour les outils d'erreurs, vont aux mêmes endroits sous le nom que chaque mod leur a donné.

Le compteur rouge **ERROR** du jeu, en bas à droite, est dessiné par le Java du jeu, que le Lua ne peut pas atteindre : le Link ne peut pas le masquer.

### Rejoindre un serveur

Quand le jeu coupe la connexion parce qu'un mod du Workshop diffère de celui du serveur, le Link nomme le mod (le jeu ne donne qu'un numéro), dit quoi faire selon le cas (le serveur se met à jour à son redémarrage, ou Steam doit télécharger la nouvelle version) et propose **Copier les liens Workshop**. Quand la build du jeu diffère de celle du serveur, il le dit avec les deux builds.

Quand vous hébergez (**HÉBERGER**, **DÉMARRER**), le jeu charge la propre liste de mods de ces réglages de serveur, pas celle de l'écran Mods. Si cette liste n'a pas des mods que vous avez activés, ou nomme des mods que ce PC n'a pas, le Link demande d'abord : **Oui** héberge quand même, **Non** revient en arrière (**Modifier les réglages** change ses mods).

### Fonctions modifiées par deux mods

Quand le Diagnostic voit deux mods redéfinir la même fonction, le Link demande au jeu, quelques secondes après le menu principal et après le chargement d'une sauvegarde, de quel fichier vient la fonction restée en place. Le Diagnostic l'affiche comme **Vu dans le jeu en cours**.

### Avec et sans l'app

- Sans l'app : l'écran Mods du Link (avec **Lignes serveur**), l'avis d'erreur et son journal des erreurs (sans **Voir dans le Studio**), et la carte d'invitation.
- Avec un profil actif, même app fermée : l'écran Mods montre le profil tel que l'app l'a laissé ; sur l'écran de chargement, chaque sauvegarde est marquée selon que ses mods correspondent au profil ou non ; une sauvegarde qui utilise des mods hors du profil ne se charge pas ; une sauvegarde avec une liste de mods vide demande d'abord.
- Seulement quand l'app est ouverte : les copies de secours d'une sauvegarde avant son chargement avec des mods modifiés et quand vous la quittez, jouer une sauvegarde sur une copie, ajouter de nouveaux mods au profil, **Synchroniser**, **Ouvrir PZ Mod Studio** et **Voir dans le Studio**.
- Les notes, groupes, profils et le Diagnostic se font dans l'app ; le jeu ne fait que les afficher.
