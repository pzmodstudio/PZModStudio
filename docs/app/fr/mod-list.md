# Liste des mods

Vos mods installés : lesquels sont activés et l'ordre dans lequel le jeu les charge. Chaque modification ici est enregistrée tout de suite dans le profil actif et dans `Zomboid/mods/default.txt`, la liste que lit le jeu.

## Ce que vous voyez

- L'en-tête compte les objets du Workshop, les mods, les mods **actifs**, et les mods de carte, de bibliothèque et de script. Des compteurs rouges et orange (**Dépendances manquantes**, **Dépendances désactivées**, **Incompatibilité**, **Ordre de la liste**) apparaissent quand il y a un problème ; cliquez sur l'un d'eux pour passer d'un de ces mods à l'autre.
- La barre d'outils : recherche, filtres, groupes, et les boutons **Tout activer** / **Tout désactiver**.
- La liste. La barre verte en haut (**SE CHARGE EN PREMIER**) est la place des bibliothèques ; la barre orange en bas (**SE CHARGE EN DERNIER**) est l'endroit où un mod l'emporte sur tout fichier partagé par deux mods.
- À droite, le panneau du mod choisi (vide tant que vous n'en avez pas cliqué un). Plus de détails ci-dessous, dans **Le panneau de droite**.

## Le panneau de droite

En haut, toujours visible : l'image du mod (cliquez pour la voir en grand), s'il s'agit d'un mod de carte, de bibliothèque ou de script, d'où il vient (Steam, copie compatible, local...), son nom, son ID, son auteur et sa version, et les boutons **Soutenir l'auteur** avec les liens que son auteur a mis dans le journal des modifications ou sur le Workshop (Ko-fi, Patreon...), ouverts dans votre navigateur. En dessous, un bouton qui l'active ou le désactive et dit où il se charge, et **Voir dans Parcourir les mods**.

Puis trois onglets (celui que vous choisissez reste quand vous changez de mod) :

- **Studio** : ce que l'app sait de ce mod dans votre liste.
  - **Notes** : votre note entière (**Modifier la note**, ou **Ajouter une note**), vos groupes et les profils où il se trouve, l'actif mis en évidence.
  - **Que faire** : chaque avertissement de la vérification avec tout ce qu'il dit, si vous devez agir, si c'est à faire ou si c'est seulement bon à savoir, et son bouton de correction. Quand il n'y a rien à faire, il le dit en vert.
  - **Dernière session** : les erreurs ou avertissements que le jeu a enregistrés (avec la dernière ligne), le temps de chargement et sa part du temps du jeu.
  - **Dépendances** : ce dont il a besoin et quels mods ont besoin de lui ; cliquez sur l'un d'eux pour aller à sa ligne.
  - **Détails des métadonnées du mod** (ce que dit son `mod.info`, avec **Dernières entrées de son journal des modifications**) et **Classes que son Java modifie**, repliés à la fin.
  - **Que faire** et **Dernière session** se replient d'un clic sur leur titre, et restent comme vous les avez laissés.
- **Page Workshop** : la page du mod sans quitter l'app : images et vidéos, abonnés, favoris, taille, dates, auteur, catégories, les objets qu'il nécessite (en vert quand vous l'avez : cliquez pour aller à sa ligne ; en orange sinon : il s'ouvre dans Parcourir les mods) et toute la description avec **Traduire**. Un mod qui n'est pas sur le Workshop montre ici sa propre description.
- **Commentaires** : les commentaires de sa page Workshop, les plus récents en premier, avec **Charger plus**. Quand le profil de l'auteur est privé, Steam ne les montre pas et le panneau le dit.

## La colonne #

Le numéro de chaque ligne est la place du mod parmi les mods **activés** : l'ordre de chargement que lit le jeu, et le même numéro que celui que le jeu affiche. Un mod désactivé affiche **—** et est grisé, car il n'a pas de place dans l'ordre de chargement. Les mods désactivés gardent leur place dans la liste, donc en réactiver un le remet en général là où il était.

## Activer et désactiver des mods

- Cliquez sur le carré d'une ligne, ou faites un clic droit et choisissez **Activer** / **Désactiver**.
- Quand un mod a besoin d'autres mods, une fenêtre liste ce qui s'active avec lui (et ce qui se désactive avec lui). Choisissez **Activer les N**, ou **Seulement ...** pour ce mod seul.
- **Tout activer** et **Tout désactiver** n'agissent que sur les lignes affichées maintenant (après recherche et filtres), et demandent d'abord.
- **Annuler** / **Rétablir** (les flèches à côté, ou Ctrl+Z / Ctrl+Y) reculent et avancent dans les modifications faites ici : déplacements, glisser-déposer, activations, tris. Chaque profil garde les siennes, même après la fermeture de l'app.
- Ctrl+clic sélectionne plusieurs mods, Maj+clic sélectionne une plage, Échap les relâche. Clic droit sur les mods sélectionnés : les activer ou les désactiver, les retirer du profil, ou se désabonner de leurs objets du Workshop.
- Un paquet brouillon ouvert de la Fusion de mods ne peut pas être activé tant qu'il n'est pas empaqueté.

## Changer l'ordre de chargement

- Faites glisser une ligne par son numéro et déposez-la au-dessus ou en dessous d'une autre ligne. Les lignes masquées entre les deux sont respectées.
- Les boutons fléchés à droite : tout en haut, monter d'un, descendre d'un, tout en bas. Monter et descendre échangent avec le mod suivant dans la liste complète, qui peut être un mod désactivé.
- **Pos. n°** : tapez un numéro et appuyez sur **Aller** ou Entrée. Le mod va à cette place parmi les mods activés.
- **Trier les dépendances** : place chaque mod après ce qu'il nécessite et ses mods `loadModAfter`, avant ses mods `loadModBefore`, et après les mods indiqués par vos propres règles. Tout le reste garde votre ordre ; les paquets de PZ Mod Studio vont en dernier. Une fenêtre dit combien de mods ont bougé, avec **Annuler**.

## Trouver des mods

- Le champ de recherche cherche dans le nom, l'id du mod, l'id Workshop et vos notes.
- Les boutons de filtre : **Tous**, **Activés**, **Désactivés**, **Favoris**, **Mis à jour**, **Problèmes**, **Cartes**, **Bibliothèques**, **Non chargeables**. Chacun affiche son nombre.
- En dessous : **Tri : ordre de chargement**, **Tri : nom**, **Tri : installés récemment** ou **Tri : installés il y a longtemps** (selon la date d'installation de chaque mod : la création de son dossier ; pour un mod du Workshop, le premier téléchargement, que les mises à jour ne changent pas ; chaque ligne affiche la date et les mods non installés vont en dernier), le menu **Groupes**, un de vos groupes, la source (**Steam Workshop**, **Vos publications sur le Workshop**, **Installés à la main**, **Copies compatibles**...), un tag (erreurs ou avertissements lors de la dernière session, lent à charger, lourd pour le jeu, un problème du Diagnostic) et **Mods du profil** / **Tous les installés**.
- L'écran Mods du jeu (mod Studio Link) a les mêmes filtres dans le même ordre. Mis à jour, Bibliothèques, le groupe, la source et le tag ont besoin de PZ Mod Studio ; sans lui, le jeu le dit là où ils seraient.
- Le glisser-déposer fonctionne aussi avec le tri par nom : le mod va à la place de celui sur lequel vous le déposez.
- **effacer les filtres** affiche de nouveau tous les mods.

## Groupes et notes

- Le menu **Groupes** dispose la liste en sections repliables : **Groupes : aucun**, **Groupes : auteur**, **Groupes : catégorie** ou **Groupes : les miens**. Chaque titre affiche combien sont activés, avec **Tout activer** / **Tout désactiver**. Dans **Groupes : les miens**, un bouton de note modifie la note du groupe.
- Faites un clic droit sur un mod et choisissez **Groupes…** pour cocher les groupes dont il fait partie, en créer un nouveau (**Y ajouter**), en renommer ou en supprimer un. Un groupe laissé vide disparaît. Les groupes ne changent jamais l'ordre de chargement.
- Choisir un de vos groupes affiche **Activer le groupe** (avec ce que ses mods nécessitent) et **Désactiver le groupe**.
- Faites un clic droit et choisissez **Ajouter une note…** pour écrire une note sur un mod (le panneau de droite le permet aussi). Sa ligne affiche le début de la note à côté du nom : survolez pour tout lire, ou cliquez dessus pour la modifier.
- Chaque profil a ses propres groupes : un groupe appartient au profil dans lequel vous l'avez créé. Les notes portent sur un mod et sont les mêmes dans tous les profils.
- Les notes et les groupes ne sont qu'à vous. Le jeu les montre (les groupes du profil actif) sur l'écran Mods du mod Studio Link.

## Les mods du profil

Avec un profil actif, la liste montre les mods de ce profil. Le bouton **Mods du profil** passe à **Tous les installés**, qui liste chaque mod installé et marque chacun avec les profils où il se trouve. Les mods installés qui ne sont dans aucun profil (abonnés depuis le site de Steam, par exemple) ont une barre bleue avec **Ajouter au profil** (ils arrivent désactivés), **Les afficher**, ou le X pour la masquer.

## Le menu du clic droit

**Activer/Désactiver**, **Ajouter au profil** / **Retirer du profil** (retirer le désactive aussi), **Placer tout en haut**, **Placer tout en bas**, **Ouvrir dans Parcourir les mods** (**Le chercher dans Parcourir les mods** pour un mod non installé), **Ouvrir son dossier**, **Ajouter une note…** (**Modifier la note…** une fois qu'il en a une), **Groupes…**, **Règles de chargement…**, **Garder cette version**, **Versions…**, **Favori**, **Ajouter à une liste…**, **Ajouter à un profil…**, **Copier l'id du mod** et **Se désabonner de son objet du Workshop** (Steam supprime ses fichiers ; les autres mods du même objet partent aussi).

- **Règles de chargement…** : indiquez qu'un mod doit se charger après un autre (ce que vous avez découvert en jouant). Le tri automatique suit la règle et le Diagnostic avertit quand la liste ne la respecte pas. Pour transmettre votre ordre à quelqu'un d'autre, exportez un préréglage : il contient la liste dans l'ordre que vos règles lui ont donné.
## Revenir à une version précédente

Steam ne garde pas les anciennes versions d'un mod du Workshop : une mise à jour remplace ses fichiers, et il n'y a aucun moyen de télécharger la précédente. Le Studio garde donc tout seul une copie de la version installée de chaque mod du Workshop qu'un profil utilise, avant que Steam ne la remplace. Quand une mise à jour casse quelque chose, la version d'avant est déjà sur ce PC.

- **Versions…** (clic droit ; aussi en vue Simple) liste les versions conservées du mod : quand chacune a été conservée, son modversion, quand son auteur l'a publiée, et des tags : **installée** (les mêmes fichiers que ceux de Steam), **utilisée**, **conservée par le Studio** ou **conservée par vous**.
- **Utiliser cette version** la met dans `Zomboid/mods` comme copie locale que le jeu lit à la place de celle du Workshop (quand il est lancé depuis le Studio). Le dossier du Workshop n'est pas touché, donc Steam ne l'annule pas. Refusé pendant que le jeu tourne.
- **Revenir à la version du Workshop** (dans la même fenêtre, ou le correctif du Diagnostic, où le mod affiche **Version conservée utilisée**) déplace la copie dans les copies de secours ; le jeu relit celle du Workshop.
- Le panneau du mod indique quand une version précédente est conservée ou qu'une est utilisée, avec un bouton **Versions…**.
- **Garder cette version** (vue Complète) garde à la main le mod tel qu'il est aujourd'hui : la limite de taille ne le supprime jamais.

Les copies se trouvent dans `Zomboid/PZModStudio_Versions`, ou dans le dossier choisi dans Paramètres, App (**Choisir un dossier…**, un autre disque par exemple). Chaque fichier n'est gardé qu'une fois, donc une version après une mise à jour ne coûte que ce que la mise à jour a changé. La limite (10 Go au départ ; le disque garde toujours 15 % de sa taille libre) et l'interrupteur sont dans Paramètres, App ; au-delà de la limite, les versions précédentes les plus anciennes partent d'abord, puis les copies des mods qui ne sont pas dans le profil actif. Les copies faites auparavant (`Zomboid/PZModStudio_Snapshots`) sont toujours listées.

## Tags des lignes

Une ligne n'affiche que ce qui demande quelque chose, pour que la liste reste lisible :

- Les avertissements du Diagnostic sur lesquels vous devez agir ou qui sont à faire (comme **Nécessite un mod désactivé** ou **Se charge trop tôt**), avec à côté le bouton clé à molette qui applique le correctif. Cliquez sur le tag pour aller au mod concerné ou au Diagnostic.
- Ceux qui sont seulement bons à savoir se replient en un petit tag avec un **i** et leur nombre ; le panneau de droite les montre tous.
- Les erreurs ou avertissements que le jeu a enregistrés pour le mod la dernière fois, un chargement de plus d'1 seconde, et sa part du temps du jeu à partir de 5 % (mesurée dans Diagnostic > Mods > Temps par mod).
- D'où il vient, seulement quand ce n'est pas Steam (**Local**, **Copie compat.**...).
- Avec **Tous les installés**, une icône de profils avec un nombre : **✓** quand il est dans le profil actif (et **+N** quand il est aussi dans d'autres), ou combien de profils le contiennent. Survolez pour voir leurs noms.

Les temps de chargement plus courts, l'origine Steam et tout le reste sont dans le panneau de droite.

## Listes précédentes et préréglages

- **Listes précédentes**, c'est l'annulation pour la liste. Chaque fois que le Studio l'a écrite, la précédente a été gardée. Choisissez-en une et appuyez sur **Revenir à celle-ci** ; votre liste actuelle est gardée aussi.
- Le bouton **Préréglages** enregistre la liste comme préréglage nommé, en charge un, ou exporte et importe des fichiers `.pzpack`. Un pack importé liste les mods qui vous manquent, avec **S'abonner à tous**.

## Bon à savoir

- **Actualiser** réanalyse les dossiers de mods après un abonnement ou un désabonnement.
- Le mod PZ Mod Studio Link reste activé dans chaque profil et est masqué dans la liste quand un profil est actif.
- Pendant que le jeu tourne, les modifications faites ici l'atteignent au menu principal (via le mod Studio Link), ou avec **Synchroniser avec PZ Mod Studio** sur son écran Mods.
