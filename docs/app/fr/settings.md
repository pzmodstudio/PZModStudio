# Paramètres de l'app

Les Paramètres de l'app, c'est là que vous dites au Studio où se trouvent votre jeu et vos mods, et choisissez comment le Studio se comporte : langue, fenêtre, avertissements, démarrage, et quelques extras facultatifs. Chaque modification est enregistrée dès que vous la faites.

## Simple et Complète

La carte en haut, **Quelle part du Studio afficher**, choisit l'une des deux vues. Le même interrupteur est toujours en bas de la barre de gauche, sous **Vue**. En changer ne supprime ni ne désactive jamais rien : cela ne fait qu'afficher ou masquer des écrans, et vous pouvez aller et venir quand vous voulez.

- **Simple** montre ce dont vous avez besoin pour jouer avec des mods :
  - **Accueil** : le profil utilisé, **Jouer**, **Continuer** (votre dernière partie avec ce profil, dès que vous en avez une) et le bilan en une ligne : vert quand il n'y a rien à faire, rouge avec **Voir quoi faire** quand quelque chose doit d'abord être corrigé.
  - **Profils**, **Liste des mods**, **Parcourir les mods**, **Sauvegardes** et **Diagnostic**.
  - La **Liste des mods** permet d'activer et de désactiver des mods, de les ordonner (à la main ou avec **Tri automatique**), de chercher, et de filtrer par activés, désactivés et avec un problème. Chaque mod n'est marqué en rouge, en mots simples, que pour ce qui doit être corrigé, avec son bouton à côté. Les groupes, notes, préréglages, les autres filtres et les temps de chargement restent dans la vue Complète.
  - **Diagnostic** s'ouvre directement sur ce sur quoi vous devez agir, chacun avec un bouton. Les suggestions facultatives (le jeu fonctionne sans elles) et ce qui est seulement bon à savoir ou déjà corrigé sont comptés sous la liste, pas affichés ; **Tout afficher (Complète)** les montre. **Corriger N problème(s) automatiquement** compte et corrige seulement les lignes de la liste que vous voyez : les suggestions masquées sont laissées de côté.
  - Ici, les Paramètres gardent les onglets **Chemins des dossiers**, **Alertes** et **L'app**.
- **Complète**, c'est tout le Studio : chaque écran de la barre latérale (Centre de surveillance, Nouveautés, Cartes, Fusion de mods, Serveurs, Développeur), chaque onglet du Diagnostic et chaque onglet des Paramètres.

Une nouvelle installation commence en **Simple**, avec un accueil en trois étapes : où votre jeu a été trouvé, créer ou choisir un profil, jouer. Si le Studio a déjà été utilisé sur ce PC, il commence en **Complète**, comme il a toujours été, et la première fois une ligne en haut propose d'essayer Simple : **Essayer Simple** ou **Non merci**. Elle ne s'affiche qu'une fois, et jamais si vous avez déjà choisi une vue. Le choix est retenu sur ce PC.

Un lien depuis un autre écran peut quand même ouvrir un écran que Simple masque (les étapes du Docteur, par exemple) ; il apparaît dans la barre latérale tant qu'il est ouvert.

## Ce que vous voyez

L'écran a des onglets en haut, côte à côte :

- **Chemins des dossiers** : où se trouvent le jeu, le Workshop, votre dossier Zomboid et la liste de mods.
- **Alertes** : quels avertissements le Studio vous montre.
- **L'app** : comment le Studio démarre, sa fenêtre et son bouton de fermeture, la langue, retenir comment le jeu se lance, l'écran Mods du jeu, la recherche de nouvelles versions et la traduction.
- **Avancé** : le diagnostic de l'app, les fonctions pour développeurs et, avec elles, les mods Java que vous avez autorisés dans ZombieBuddy.
- **Correctifs Build 42** : ce que couvre le shim de compatibilité Build 42, avec un interrupteur par mod qu'il corrige.
- **Serveur d'agents MCP** : affiché seulement avec **Fonctions pour développeurs** activé.

## Dossiers

Appuyez sur **Détecter les chemins** pour laisser le Studio tout trouver tout seul. Pour définir un dossier à la main, appuyez sur **Parcourir...** à côté. Une note **Enregistré.** confirme la modification.

- **Dossier d'installation de Project Zomboid** : le dossier qui contient `ProjectZomboid64.exe`. Il affiche **Jeu trouvé** quand le dossier est bon, et **Le Studio ne trouve pas le jeu ici** quand il ne l'est pas. Les installations Steam, GOG et manuelles fonctionnent toutes.
- **Dossier du Steam Workshop (facultatif sans Steam)** : l'endroit où Steam télécharge les mods du Workshop (`workshop/content/108600`). Laissez-le vide si vous jouez sur GOG ou mettez les mods dans `Zomboid/mods` à la main.
- **Dossier Zomboid de l'utilisateur (données et mods locaux)** : votre dossier `C:\Users\<User>\Zomboid`, avec les journaux, les sauvegardes, les profils et les mods locaux. Il est obligatoire.
- **Fichier de la liste des mods (Zomboid/mods/default.txt)** : le fichier que lit le jeu pour savoir quels mods sont activés et dans quel ordre. Il est trouvé automatiquement ; **Parcourir...** ici choisit un fichier, pas un dossier.

Sans dossier du jeu valide, des écrans comme la Fusion de mods vous demandent de venir ici d'abord.

## Avertissements

Sous **Notifications et avertissements**, vous pouvez masquer ou rétablir trois messages :

- **Avertissement au lancement sur les conflits de mods non fusionnés** : l'avertissement avant le lancement quand vos mods actifs partagent des fichiers qu'aucun paquet de fusion actif ne fusionne (ou fusionne à partir d'anciennes versions des mods).
- **Avis au lancement sur les mods qui définissent les mêmes choses** : l'avis avant le lancement quand deux mods actifs définissent différemment le même objet, la même recette ou le même texte et qu'aucun paquet ne décide lequel reste. Rien ne casse : celui qui se charge en dernier l'emporte.
- **Message explicatif du tri automatique des dépendances** : l'explication affichée quand vous triez les dépendances dans la Liste des mods.

Chacun a **Masquer l'avertissement** / **Rétablir l'avertissement** (ou **Masquer la notification** / **Rétablir la notification**).

## Le démarrage du Studio et sa fenêtre

- **Que doit ouvrir le Studio au démarrage ?** : **Écran des profils (accueil)** (recommandé) montre le gestionnaire de profils ; **Ouvrir directement le dernier profil** charge le dernier profil actif et ouvre la Liste des mods.
- **Ouvrir la fenêtre comme je l'ai laissée** : même taille, même place et même écran, agrandie ou en plein écran si elle l'était. Si cet écran n'est plus là, elle s'ouvre sur l'écran principal.
- **Le bouton fermer (X)** : **Ferme le Studio** le quitte (le jeu continue de tourner s'il est ouvert). **Le garde dans la zone de notification, près de l'horloge** masque la fenêtre et le Studio continue de surveiller le jeu (copies de secours quand vous quittez, le profil dans l'écran Mods du jeu). Cliquez sur l'icône de la zone de notification pour le rouvrir ; son menu a **Quitter**.

## Langue

Choisissez la langue de l'app sous **Langue** : English, Español, Português (Brasil), Français, Deutsch, Русский ou 简体中文. Une ligne qui manque dans une langue s'affiche en anglais. Les messages du jeu lui-même (erreurs, avertissements, les lignes du journal du Centre de surveillance) restent tels que le jeu les écrit.

## Le lancement du jeu et l'écran Mods du jeu

- **Retenir comment le jeu se lance** : activé, le bouton de lancement démarre dans le mode utilisé la dernière fois (Debug, Fenêtré, Normal...). Désactivé, il démarre toujours en Mode debug (plein écran), le mode recommandé.
- **L'écran Mods du jeu, avec un autre gestionnaire de mods activé** : un seul gestionnaire de mods peut dessiner l'écran Mods du jeu. Quand un autre est activé, choisissez **Me demander dans le jeu**, **Celui de PZ Mod Studio (redemande si l'autre est activé)** ou **Celui de l'autre gestionnaire**. Les copies de secours des sauvegardes et la vérification avant le chargement d'une sauvegarde restent à PZ Mod Studio dans tous les cas.

## Mises à jour de l'app

- **Rechercher les nouvelles versions de PZ Mod Studio** : au démarrage et toutes les 6 heures tant qu'il est ouvert, le Studio demande à GitHub quelle est la dernière version publiée. Quand il y en a une plus récente, une barre sous l'en-tête le dit. **Mettre à jour** télécharge la nouvelle version, vérifie que le projet l'a signée (la signature de `SHA256SUMS.txt` et le SHA-256 du zip), la met à la place de la vôtre et redémarre le Studio ; si quelque chose ne correspond pas, rien ne change. **Télécharger** ouvre la page de la version dans votre navigateur, et **Pas maintenant** masque l'avis jusqu'à la version suivante. Quand le Studio est installé dans un dossier où il ne peut pas écrire (Program Files, pour tous les utilisateurs), mettez-le à jour avec l'installateur de la page. Rien n'est installé sans que vous le demandiez, et rien n'est envoyé. **Vérifier maintenant** demande tout de suite. Quand GitHub ne répond pas, le Studio demande à sa page itch.io à la place ; l'avis ne propose alors que la page de téléchargement, pas **Mettre à jour**. Désactivé, GitHub n'est pas interrogé.

## Anciennes versions des mods

- **Garder les anciennes versions des mods** (l'app le demande la première fois, en disant la place que cela prendrait et sur quel disque ; rien n'est gardé avant votre réponse) : Steam ne garde pas les anciennes versions d'un mod du Workshop. Le Studio garde une copie de la version installée de chaque mod du Workshop qu'un profil utilise, dans `Zomboid/PZModStudio_Versions`, avant que Steam ne la remplace ; **Versions…** dans la Liste des mods en remet une. **Jusqu'à** fixe la place qu'elles peuvent prendre (10 Go au départ ; une limite que vous avez choisie reste comme vous l'avez choisie) ; chaque fichier n'est gardé qu'une fois, donc une version après une mise à jour ne coûte que ce qui a changé. Au-delà de la limite, les versions précédentes les plus anciennes partent d'abord, puis les copies des mods qui ne sont pas dans le profil actif ; les versions gardées à la main restent. Quelle que soit la limite, le Studio ne remplit jamais le disque : chaque passage laisse toujours libres 15 % de la taille du disque, et quand c'est cela qui freine la réserve, la carte dit **Limité par l'espace libre sur D:** (avec la limite qui s'applique pour l'instant). **Dossier** montre où sont les copies et combien elles prennent ; **Choisir un dossier…** les déplace, par exemple vers un autre disque : chaque fichier est copié et vérifié par rapport à l'original, et seulement ensuite l'ancien dossier est supprimé (sur le même disque, il est simplement renommé). Un dossier qui contient déjà d'autres fichiers reçoit un dossier `PZModStudio_Versions` à l'intérieur. Le déplacement est refusé pendant que le jeu tourne ou qu'un passage est en cours, et pour la racine d'un disque, le dossier du Workshop de Steam, le dossier du jeu, `Zomboid/mods` et `Zomboid/Saves`. La ligne en dessous dit combien de place est utilisée et combien de vos mods du Workshop sont couverts ; les mods laissés sans copie faute de place sont nommés. **Garder maintenant** lance un passage tout de suite (il se lance aussi tout seul peu après le démarrage du Studio et chaque fois que Steam met quelque chose à jour). Rien n'est fait pendant que le jeu tourne, et le dossier du Workshop n'est que lu.

## Diagnostic

**Diagnostic** rassemble ce dont un rapport de bug a besoin : les versions du Studio et de Windows, les dossiers, la build du jeu, le profil actif, le Studio Link, Zombie Buddy, si Steam limite les requêtes, et la fin du journal du Studio. **Copier le diagnostic** le copie ; votre nom d'utilisateur Windows et le nom de votre PC sont remplacés par `<user>` et `<pc>`. Regardez-le avant de le coller quelque part. **Ouvrir le dossier des journaux** ouvre le dossier contenant les journaux du Studio lui-même.

## Traduction

Parcourir les mods a un bouton Traduire sur la page d'un mod et un pour les résumés d'une page de résultats ; les brouillons de traduction de l'écran Développeur utilisent le même service. Sans clé, le service gratuit Google Traduction est utilisé (il n'est pas officiel et peut s'arrêter un jour). Avec une clé API DeepL (gratuite jusqu'à 500 000 caractères par mois), DeepL est utilisé en premier : appuyez sur **Obtenir une clé DeepL**, collez-la et appuyez sur **Vérifier et enregistrer**. La clé est gardée dans le Gestionnaire d'identification de Windows, sur ce PC seulement ; **Supprimer** l'efface. Les traductions déjà faites sont gardées sur ce PC, pour qu'un texte ne soit pas renvoyé ; **Les supprimer** les efface.

## Fonctions pour développeurs

**Fonctions pour développeurs** affiche l'écran Développeur dans la barre latérale, l'onglet **Serveur d'agents MCP** (qui permet à un assistant IA de piloter le Studio) et le mod compagnon qui exécute des commandes dans le jeu. Un joueur n'en a pas besoin ; laissez-le désactivé sauf si vous développez avec. Le choix est retenu sur ce PC.

## Correctifs Build 42

Cet onglet explique le shim de compatibilité Build 42 : un fichier que le Studio écrit dans chaque paquet de fusion qu'il construit, et qui corrige les appels que les mods Build 41 font à des choses que Build 42 a renommées ou supprimées. Les parties 1 à 6 s'appliquent toujours tant qu'un paquet de fusion est actif. La partie 7 corrige des mods précis, chacun avec un interrupteur **ON** / **OFF**. Un interrupteur est écrit tout de suite dans chaque paquet du Studio sur le disque ; aucune publication n'est nécessaire, et le jeu le lit à son prochain lancement.

## Serveur d'agents MCP

Seulement avec **Fonctions pour développeurs** activé. Choisissez votre client IA, appuyez sur **Copier le JSON MCP** et collez-le dans le fichier de configuration de ce client ; des notes pas à pas pour chaque client sont affichées en dessous. Ne lancez pas `pz-mcp-server.exe` en double-cliquant dessus : une fenêtre noire vide est normale, et votre client IA le démarre et l'arrête tout seul.

## Bon à savoir

- Les choix de dossier, de langue, de fenêtre et d'avertissements s'appliquent tout de suite ; rien ne demande de redémarrage, sauf les interrupteurs du shim, que le jeu lit à son prochain lancement.

## Sous Linux

La version Linux (AppImage ou .deb) trouve Steam dans ses dossiers habituels (`~/.local/share/Steam`, `~/.steam`, celui du Flatpak) et le dossier Zomboid dans votre dossier personnel. Certaines choses ne marchent que sous Windows : mettre l'app à jour sur place (sous Linux, **Télécharger** ouvre la page de la version), déplacer et capturer la fenêtre du jeu pour un assistant IA (garer, captures d'écran, clics, touches), le journal de manque de mémoire de Windows, s'abonner via le client Steam et télécharger SteamCMD (installez-le depuis votre distribution). Les clés et mots de passe enregistrés sont gardés dans un fichier privé, `~/.config/PZModStudio/secrets.json`, au lieu d'un coffre d'identifiants. Pour un client IA, le serveur console est `/usr/bin/pz-mcp-server` avec le .deb ; l'AppImage sert lui-même le MCP avec `--mcp`.
