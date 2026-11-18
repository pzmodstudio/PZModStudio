# Profils

Un profil est une liste de mods nommée : quels mods en font partie, lesquels sont activés, et leur ordre de chargement. Ici, vous créez des profils, passez de l'un à l'autre en un clic, les partagez et les supprimez. La plupart des autres écrans restent verrouillés tant qu'aucun profil n'est actif.

## Ce que vous voyez

- En haut, combien de profils vous avez enregistrés et le bouton **Nouveau profil**, qui ouvre une fenêtre qui vous guide pour en créer un en trois étapes.
- Une carte par profil enregistré, l'actif en premier. L'actif a un cadre vert, le tag **Actif** et le bouton **Voir ses mods**, qui ouvre la Liste des mods ; les autres disent **Inactif** et ont **Activer (1 clic)**.
- Chaque carte montre sa description, combien de mods sont activés (**Mods actifs**), le **Total** des mods de son ordre, et la build : **Build 41/42** quand vous l'avez réglée à la main, ou **Joué sur ...** avec la build sur laquelle il a été joué la dernière fois depuis le Studio.

## Créer un profil

Appuyez sur **Nouveau profil** : une fenêtre s'ouvre avec trois étapes (**Nom**, **Quels mods**, **Vérifier**), chacune avec une explication en haut. **Suivant** et **Retour** passent de l'une à l'autre ; Échap ou le X la ferme sans rien créer.

1. Tapez un **Nom du profil** (obligatoire) et, si vous voulez, une **Description (facultative)**.
2. Sous **Quels mods**, choisissez avec quoi le profil commence :
   - **Ce qui est activé** : les mods activés en ce moment.
   - **Tout ce qui est installé** : tous les mods que le Studio a trouvés.
   - **Les choisir** : une liste avec un champ de recherche et des boutons **Tous** / **Aucun** ; cochez les mods voulus.
   - **Rien pour l'instant** : un profil vide. Le Studio vous demande de confirmer, car y passer désactive tous les mods. Quand un profil est actif, un bouton **Trouver des mods dans Parcourir les mods** vous y emmène pour vous abonner d'abord.
3. Sous **Vérifier**, vous voyez le nom et combien de mods commencent activés. Appuyez sur **Créer et activer**.

Le nouveau profil est créé et activé tout de suite, et garde l'ordre de chargement de votre liste actuelle. Si le Studio n'a encore trouvé aucun mod (dossier du jeu ou Zomboid non défini dans Paramètres de l'app), il refuse, pour qu'un profil ne désactive pas tous vos mods par erreur.

## Changer de profil

Appuyez sur **Activer (1 clic)** sur une carte. Le Studio alors :

- écrit les mods de ce profil dans la liste que lit le jeu (`Zomboid/mods/default.txt`) ;
- le marque comme actif et ouvre la Liste des mods avec son ordre ;
- installe le mod PZ Mod Studio Link et le garde activé dans chaque profil, pour que l'écran Mods du jeu montre le profil.

Si un mod du profil a besoin d'un agent Java dans le fichier de lancement du jeu (Zombie Buddy, par exemple), une fenêtre montre d'abord ce qui sort de ce fichier ou y revient. Appuyez sur **Activer ...** pour continuer. Si le jeu tourne, cette partie attend que le jeu se ferme. Une copie du fichier de lancement est gardée avant.

## Isoler ce profil

**Isoler ce profil** garde à part, pour ce profil :

- ses **sauvegardes** solo ;
- les **options du jeu** : graphismes, son et touches ;
- les **réglages des mods** : ce que chaque mod enregistre dans le dossier Zomboid (ses options, ses réglages, ses données).

Avec ce profil activé, le jeu n'utilise que les siens, et avec un autre profil ceux de l'autre (ou ceux partagés, quand l'autre n'est pas isolé). Quand vous l'isolez, vous choisissez quelles sauvegardes partagées il emporte ; le reste reste partagé. Les options et les réglages des mods commencent comme une **copie** de ceux que vous avez maintenant, donc le profil démarre exactement comme avant et les autres profils gardent les leurs.

Les **mods eux-mêmes sont les mêmes pour tous les profils** : ils ne sont ni copiés ni déplacés, et il n'y a rien à réinstaller. Un mod (ou une partie d'un mod) activé dans un profil et désactivé dans un autre fonctionne comme toujours. Les sauvegardes des serveurs (`Saves/Multiplayer`), les journaux du jeu, les serveurs, et les fichiers de l'app et du Link ne sont pas séparés non plus.

Les grosses données qui ne sont pas des réglages ne sont pas copiées quand vous isolez un profil : un seul fichier de plus de 20 Mo ou un dossier de plus de 50 Mo (le cache d'un mod, des enregistrements ou des dumps) reste **partagé**, le même pour tous les profils à partir de là, si bien qu'isoler ne remplit jamais le disque. Les options et touches du jeu lui-même sont toujours copiées, quelle que soit leur taille. Ce qui est resté partagé est noté dans `Zomboid/PZModStudio_ProfileSavesShared.json`.

Quand vous changez de profil, l'app remet tout à sa place en renommant, sans rien copier. Elle ne le fait que lorsque Project Zomboid est fermé : le jeu, et aussi tout serveur Project Zomboid sur ce PC (le serveur du Studio depuis l'écran Serveurs, ou un serveur lancé à la main). S'il y en a un qui tourne, l'app dit lequel et vous demande de le fermer d'abord. Ce qui appartient aux autres profils attend dans `Zomboid/PZModStudio_ProfileSaves`. Si le PC s'éteint au milieu d'un changement, l'app le termine à sa prochaine ouverture.

Chaque sauvegarde et chaque réglage que l'app déplace, copie ou garde dans les copies de secours est noté dans le journal du Studio (`Zomboid/PZModStudio_Logs/studio.log`, lignes avec `[Profiles]`), d'où vers où, donc si une sauvegarde semble manquer, vous pouvez voir où elle est partie.

**Ne plus isoler** remet ses sauvegardes avec celles partagées et ramène les options et les réglages des mods partagés ; ceux propres au profil sont gardés dans `Zomboid/PZModStudio_Backups/ProfileSettings`, sans rien écraser. Supprimer le profil fait la même chose, donc une sauvegarde ou un réglage n'est jamais perdu.

## Modifier, partager et supprimer

- Le bouton crayon (**Renommer ce profil**) ouvre **Modifier le profil de mods** : changez le nom, la description et la **Build du jeu**. Laissez la build sur **Comme lors de la dernière partie** ou réglez **Build 41** / **Build 42**. Avec une build réglée, le Studio demande avant de lancer ce profil sur une autre build, même avant qu'il ait été joué. Appuyez sur **Enregistrer les modifications**.
- Le bouton partager enregistre le profil dans un fichier `.pzpack` que vous pouvez envoyer à n'importe qui. Il ne contient que les mods activés, dans l'ordre, avec leurs id Workshop. Qui l'ouvre peut s'abonner à ce qui lui manque.
- Le bouton corbeille supprime le profil après confirmation. Si certains de ses objets du Workshop ne sont utilisés par aucun autre profil, la confirmation est une fenêtre qui les liste (**Utilisés seulement par ce profil**), cochés, avec **Tout cocher** / **Tout décocher**. **Supprimer et se désabonner** supprime le profil et vous désabonne des objets cochés (Steam doit être lancé) ; **Supprimer seulement le profil** les garde. Les objets qu'un autre profil utilise, et les mods locaux, ne sont jamais touchés. Si le profil utilise des paquets de correctifs de la Fusion de mods (`Z_PZModStudio_...`), une seconde question demande s'il faut les supprimer aussi ; **Annuler** les garde.

## Bon à savoir

- Un profil supprimé n'est pas perdu : son fichier est déplacé dans `Zomboid/PZModStudio_Backups/profiles`.
- Supprimer le profil actif ne laisse aucun profil actif, et l'écran Mods du jeu cesse d'afficher un profil.
- Les modifications faites dans la Liste des mods (activer des mods, réordonner) sont enregistrées tout de suite dans le profil actif.
- Les modifications acceptées dans l'écran Mods du jeu reviennent aussi dans le profil actif.
- Un mod activé devient un des mods du profil. Les mods qui n'appartiennent à aucun profil sont proposés dans la Liste des mods avec **Ajouter au profil**.
- Les sauvegardes ne passent jamais d'une build majeure à l'autre (41 et 42), c'est pourquoi un profil se souvient de sa build.
