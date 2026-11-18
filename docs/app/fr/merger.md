# Fusion de mods

La Fusion de mods répare les fichiers que deux de vos mods actifs ou plus modifient tous les deux, et vous permet de corriger un bug dans un fichier d'un mod. Vos décisions vont dans un paquet : un petit mod à part qui se charge en dernier, pour que le jeu lise votre version de ces fichiers.

## Ce qu'est un paquet

- Un paquet est un dossier de mod dans `Zomboid/mods` dont le nom commence par `Z_PZModStudio_` (celui par défaut est `Z_PZModStudio_MergedPatch`).
- Quand vous le publiez, le Studio y écrit vos fichiers fusionnés (pour Build 41, et dans son dossier `42` pour Build 42), ajoute un petit fichier de compatibilité, et place le paquet **en dernier** dans la liste de mods du jeu. Le mod qui se charge en dernier est celui dont le jeu utilise la copie d'un fichier, donc votre copie fusionnée l'emporte.
- Un paquet a deux états. **🟢 EMPAQUETÉ / FERMÉ** veut dire qu'il est publié et que le jeu le lit. **🟡 OUVERT (brouillon)** veut dire que vous êtes en train de le modifier ; tant que c'est un brouillon, il est retiré de la liste de mods du jeu, donc le jeu ne le lit pas avant que vous le publiiez à nouveau.
- Chaque paquet appartient aux profils qui l'ont parmi leurs mods. Un paquet que vous créez pendant qu'un profil est actif est ajouté à ce profil.
- Un paquet se souvient du profil qui était actif lors de sa dernière publication (**construit avec** sur sa carte, en orange quand un autre profil est actif maintenant) : ses fusions règlent la liste de ce profil. Avec un profil qui n'a pas de paquet à lui, **Paquet pour « nom »** dans la barre du haut en crée un à son nom.

## Ce que vous voyez

Quand aucun paquet n'est ouvert, vous voyez la liste **Paquets de fusion**. Chaque carte montre le nom du paquet, ses fichiers, son état, les profils auxquels il appartient et ces boutons :

- **🔍 Ouvrir / Entrer dans le paquet** l'ouvre dans l'espace de travail et analyse vos mods actifs à la recherche de conflits.
- **Visible dans la ModList** / **Masqué dans la ModList** active ou désactive le paquet dans la liste de mods du jeu (`default.txt`).
- Le crayon modifie son nom et sa description ; la corbeille le supprime du disque (on vous demande d'abord). Les profils qui le contenaient gardent le nouveau nom après un renommage.
- L'icône de téléchargement l'enregistre en fichier `.pzmerge` à partager ; l'icône de dossier ouvre son dossier dans l'Explorateur Windows.
- **Paquets des autres profils** montre les paquets qui appartiennent à d'autres profils ; **Seulement « nom »** revient à ceux du profil actif.

La barre du haut a **Charger un paquet (.pzmerge)** pour importer un paquet que quelqu'un vous a envoyé, et **Nouveau paquet** pour en créer un (un nom, une description facultative, puis **Créer le paquet**). Quand le profil actif n'a pas de paquet de fusion, l'écran le dit et explique quand il en faut un, avec un bouton pour voir les paquets des autres profils. Les mods du Studio lui-même (le Live IPC Bridge et le Studio Link) ne sont pas des paquets de fusion : ils se trouvent en dessous, repliés sous **Mods du Studio**, et vous pouvez voir leurs fichiers mais pas les modifier, les renommer ni les supprimer.

## Travailler dans un paquet

La barre du haut montre **← Retour à la liste**, un sélecteur **Paquet :** pour passer à un autre paquet, l'état et la visibilité, et ces boutons :

- **Corriger un fichier de mod** : ouvre un fichier d'un mod pour le corriger (voir plus bas).
- **⚡ Tout fusionner et publier** : fusionne chaque fichier en conflit avec le moteur automatique et publie le paquet d'un coup : il est écrit dans `Zomboid/mods` et placé en dernier dans votre liste de mods, et le jeu le lit à son prochain lancement. Relisez les fichiers ensuite.
- **↩️ Tout défusionner** : jette toutes les fusions enregistrées de ce paquet. Une copie est gardée dans `Zomboid/PZModStudio_Backups`, et les fichiers déjà publiés restent jusqu'à ce que vous publiiez à nouveau.
- **Tous** / **Sans doublons** : masque les fichiers dont les mods fournissent des copies identiques, pour ne lister que les vraies collisions.
- Icônes de dossier, d'export et de nouvelle analyse.
- **📦 Empaqueter et publier** : l'étape finale. Elle écrit dans le paquet la décision actuelle de chaque fichier et place le paquet en dernier dans la liste de mods du jeu.

À gauche, **Conflits VFS** liste chaque fichier en collision avec son type (Lua, **DÉFINITION** pour les scripts d'objets et de recettes, **TEXTES** pour les traductions) et son état : **À vérifier**, **Fusion auto** ou **Résolu**.

## Décider ce que le jeu lit pour un fichier

1. Cliquez sur un fichier dans la liste. La moitié haute montre le fichier vanilla, la copie de chaque mod et un résumé de ce que chaque mod change par rapport au jeu de base ; les vrais conflits (les mêmes lignes modifiées différemment) sont marqués. Cliquez sur une modification pour faire défiler chaque panneau jusqu'à elle.
2. Sous **Que doit lire le jeu ?**, choisissez une réponse :
   - **Garder les deux** (pour le Lua et les autres fichiers) : les modifications de chaque mod ensemble. Là où deux modifient les mêmes lignes, celui qui se charge en dernier l'emporte et l'autre est gardé en commentaire.
   - **Les fusionner** (pour les définitions et les traductions) : ce qui peut contenir les deux valeurs garde les deux ; là où une seule rentre, ce que le jeu fait aujourd'hui reste et l'autre est laissé en commentaire.
   - **Seulement (nom du mod)** : la version de ce mod. Pour une définition, les réglages des autres mods que celui-ci ne touche pas s'appliquent toujours.
   - **Vanilla** : le fichier du jeu lui-même, comme si aucun mod n'y avait touché (pas proposé quand le jeu de base n'a pas ce fichier).
3. Ou modifiez le code dans l'éditeur en dessous ; il affiche alors **Votre propre modification**. **Ajouter** et **Retirer** sur le panneau d'un mod ajoutent ou enlèvent toute la copie de ce mod, et **Charger la base** met le fichier vanilla dans l'éditeur.
4. Appuyez sur **✓ Enregistrer la fusion de ce fichier**. Le fichier devient **Résolu**.
5. Quand chaque fichier est fait, appuyez sur **📦 Empaqueter et publier**.

**Restaurer la fusion AST** remet la fusion automatique dans l'éditeur, et **Défusionner ce fichier** oublie la décision enregistrée pour ce fichier. Faites glisser la barre entre les panneaux pour les redimensionner.

## Corriger un bug dans un mod

Utilisez **Corriger un fichier de mod** quand un mod a un bug et qu'il n'y a rien à fusionner. Choisissez le mod, puis le fichier Lua ou de script, modifiez-le et appuyez sur **✓ Enregistrer la fusion de ce fichier**. Votre copie va dans le paquet et le jeu la lit à la place de celle du mod, pour ce fichier seulement ; le mod reste activé et le reste n'est pas touché. Enregistrer une copie sans modification est refusé, car cela ne ferait que figer le fichier dans sa version d'aujourd'hui.

Le Centre de surveillance et le Diagnostic peuvent vous envoyer ici directement : le bon paquet s'ouvre et le fichier nommé par l'erreur s'ouvre dans l'éditeur, parfois avec le correctif déjà appliqué.

## Paquets empaquetés et publication sur place

Un paquet empaqueté est en lecture seule : ses fichiers restent cachés derrière un résumé jusqu'à ce que vous appuyiez sur **🔓 Défusionner / Ouvrir pour modifier** (ou **Dépaqueter pour modifier**) et confirmiez. Dépaqueter retire les fichiers publiés du jeu jusqu'à ce que vous publiiez à nouveau.

Un fichier d'un mod ouvert pour être corrigé est l'exception : dans un paquet empaqueté, son enregistrement **publie sur place**. Tout ce que le paquet contient est publié à nouveau avec votre fichier à côté, le paquet reste empaqueté, et le jeu lit le nouveau fichier à son prochain lancement. Quand le Diagnostic ou le Centre de surveillance vous envoie à un fichier d'un paquet empaqueté, ce fichier est affiché sans le dépaqueter.

## Partager des paquets

**Charger un paquet (.pzmerge)** importe un fichier de paquet. S'il a été fait avec des mods que vous n'avez pas, avec des copies différentes ou sur une autre build du jeu, une fenêtre **Avant de publier** les liste, avec **S'abonner à tous** pour les mods du Workshop manquants. La solution la plus sûre est que vous ayez tous les deux les copies les plus récentes, puis de relancer l'analyse et de fusionner à nouveau avant de publier.

## Bon à savoir

- Le jeu lit un paquet à son démarrage : après une publication, relancez le jeu s'il tourne.
- Si le dossier du jeu n'est pas trouvé, l'écran vous demande d'abord de régler les chemins dans **Paramètres de l'app**.
- Quand vous quittez un fichier avec des modifications non enregistrées, on vous demande s'il faut les abandonner.
- Un paquet que vous supprimez est retiré du disque ; les mods qu'il fusionnait ne sont pas touchés.
