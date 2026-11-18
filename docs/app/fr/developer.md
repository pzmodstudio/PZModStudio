# Développeur

L'écran Développeur est pour celles et ceux qui créent des mods. Vous pouvez créer un nouveau mod, vérifier chaque fichier d'un mod avant le jeu, lancer le jeu avec seulement ce mod activé, recharger son Lua pendant que le jeu tourne, le vérifier pour le multijoueur et face à la build installée, écrire des brouillons de traduction, le publier sur le Workshop de Steam et lancer les éditeurs de cartes.

## Ce que vous voyez

L'écran n'apparaît dans la barre latérale que lorsque **Fonctions pour développeurs** est activé dans **Paramètres de l'app**, et, comme les autres écrans, une fois qu'un profil est actif. En haut, **Le mod sur lequel vous travaillez** contient le mod sur lequel agit chaque outil, avec **Ouvrir son dossier** et **Lancer le jeu avec ce mod seul**. Dessous, il y a un onglet par outil : **Vérifier les fichiers**, **Rechargement à chaud**, **Multijoueur**, **Face à la build**, **Traductions**, **Workshop**, **Cartes** et **Nouveau mod**. Un onglet affiche un petit badge quand il a quelque chose à dire (problèmes trouvés, surveillance en cours, le verdict multijoueur). Le Studio se souvient du dernier onglet ouvert. Tous les onglets sauf **Workshop**, **Cartes** et **Nouveau mod** demandent d'abord de choisir un mod.

## Créer un nouveau mod

1. Ouvrez l'onglet **Nouveau mod**.
2. Remplissez **Nom**, **Id** (le dossier et l'entrée dans la liste : lettres, chiffres, `_` et `-` ; il suit le nom tant que vous ne le changez pas), **Auteur** et **Description**.
3. Cochez **Aussi Build 41** si le mod doit aussi fonctionner sur Build 41.
4. Appuyez sur **Créer**.

Le Studio crée le mod dans `Zomboid/mods` comme Build 42 le lit : `42/mod.info`, les dossiers `42/media/lua/client`, `server` et `shared`, `42/media/scripts`, `common/media/lua`, une affiche à remplacer et un fichier Lua qui affiche le nom du mod quand le jeu le charge. Il refuse un id qui n'est pas un simple nom ou un dossier qui existe déjà. **Ouvrir le dossier** l'ouvre dans l'Explorateur Windows, et le nouveau mod devient celui sur lequel vous travaillez. Activez-le dans la Liste des mods quand il est prêt.

## Choisir le mod sur lequel vous travaillez

Sous **Le mod sur lequel vous travaillez**, choisissez un mod dans la liste. Les mods sont regroupés selon leur provenance, ceux sur lesquels vous travaillez en premier : **Vos envois sur le Workshop (Zomboid/Workshop)**, **Dossiers dans Zomboid/mods**, **Installés depuis Nexus, un fichier ou un lien**, les copies que le Studio a adaptées à votre build, **Les mods du Studio**, et **Abonnements du Workshop** en dernier, pour jeter un œil. Sous la liste, vous voyez l'id du mod, sa version, son numéro Workshop et son dossier. **Ouvrir son dossier** ouvre le dossier du mod. Le Studio se souvient de votre choix.

## Vérifier chaque fichier

Dans l'onglet **Vérifier les fichiers**, appuyez sur **Vérifier tous les fichiers**. Chaque fichier `.lua` passe par le même analyseur Lua que celui de la Fusion de mods, et chaque `media/scripts/*.txt` par une vérification des accolades (le jeu perd en silence tout un fichier de script dès son premier bloc mal fermé). Le résultat apparaît juste sous le bouton, avec le nombre de fichiers Lua et de script vérifiés. Il dit **Tous les fichiers sont lisibles**, ou liste les problèmes par fichier avec ligne et colonne. Cliquez sur un problème pour ouvrir le fichier à cette ligne dans VS Code s'il est installé, ou sinon dans le programme que Windows utilise pour ce fichier.

Cela ne vérifie que la syntaxe. Ce que le jeu fait des fichiers se voit dans le Centre de surveillance.

## Lancer le jeu avec ce mod seul

**Lancer le jeu avec ce mod seul**, en haut de l'écran, lance le jeu en Debug (fenêtré) avec seulement ce mod, ce qu'il nécessite et le pont du Studio activés. Votre liste de mods est mise de côté, et un bandeau l'indique. Elle revient quand le jeu se ferme, ou tout de suite avec **Remettre ma liste maintenant**. La liste est aussi gardée dans **Listes précédentes** dans la Liste des mods. Le bouton est désactivé pendant qu'un tel lancement tourne.

## Recharger dans le jeu pendant qu'il tourne

Cela vous permet de modifier un fichier Lua et de le voir dans le jeu sans redémarrer.

1. Lancez le jeu avec le mod pont (`Z_PZModStudio_Bridge`) activé.
2. Ouvrez l'onglet **Rechargement à chaud** et activez l'interrupteur à droite. Il indique **Surveillance (nom du mod)**.
3. Enregistrez un fichier Lua du mod. Le Studio le voit dès qu'il est enregistré, le vérifie avec l'analyseur et l'envoie au jeu, qui le recharge au menu principal, en jeu ou en pause.

**Recharger tout le Lua de ce mod** envoie tous les fichiers Lua du mod d'un coup.

Le journal sous les boutons montre chaque étape :

- **modifié :** un fichier a été envoyé au jeu.
- **non envoyé, il ne se lit pas :** le fichier a une erreur de syntaxe, avec sa ligne, sa colonne et son message. Il n'est pas envoyé, le jeu n'exécute donc jamais de code cassé.
- La réponse du jeu pour chaque fichier, en vert quand ça a marché et en rouge quand ça a échoué.
- Les temps : détecté, envoyé, appliqué dans le jeu, et de retour dans le Studio, en millisecondes.
- **Le jeu n'est pas lancé** ou **Pas de réponse du jeu en 6 s** quand il n'y avait personne pour recharger (vérifiez que le mod pont est activé).

Quand un fichier est rechargé, son code de premier niveau s'exécute à nouveau, et le pont préserve ce que cela casserait :

- **Gestionnaires d'événements** : chacun de ceux que le fichier ajoute s'exécute une seule fois, un gestionnaire ajouté pour la première fois se met à fonctionner, et un gestionnaire retiré du fichier s'arrête.
- **Tables** : quand le fichier recrée une table globale (`MyMod = {}`, `MyPanel = ISPanel:derive(...)`), l'ancienne table est gardée et reçoit les nouvelles fonctions, si bien que les fenêtres ouvertes et les références gardées exécutent le nouveau code avec leur état. Une valeur que l'ancienne table avait déjà garde son ancienne valeur (le pont ne peut pas distinguer votre état d'une constante que vous avez changée) : pour une constante modifiée, faites un Reset Lua complet.
- **Votre propre mise en place** : `if PZStudio then PZStudio.onReload(function(file) ... end) end` s'exécute après chaque rechargement de ce fichier, pour ce que `OnGameStart` a fait et ne refait pas.
- **Fichiers qui le chargent par `require`** : les autres fichiers du mod qui chargent le fichier modifié avec `require` sont rechargés juste après lui, pour qu'ils reçoivent son nouveau code.

Le journal dit ce qui a été préservé, par exemple « 1 gestionnaire(s), 1 table(s) préservés ».

## Vérification multijoueur

Dans l'onglet **Multijoueur**, **Vérifier ce mod pour le multijoueur** examine le Lua client et partagé du mod à la recherche d'appels qui modifient directement le monde, un véhicule, un animal ou la carte, là où le code du jeu lui-même passe par le serveur. Chaque constat montre le fichier, la ligne et le gestionnaire du jeu qui le fait côté serveur. Un appel dans une branche `if isClient()` compte comme géré. Cliquez sur un constat pour l'ouvrir dans votre éditeur. Seuls ces appels sont vérifiés : un mod sans rien de listé peut quand même mal se comporter en multijoueur d'une façon que cette vérification ne voit pas.

## Face à la build installée

Dans l'onglet **Face à la build**, **Vérifier les noms de cette build** cherche chaque nom que le Lua du mod appelle dans le jeu que vous avez installé : ses classes et méthodes Java, ses fonctions globales, son propre Lua et les événements qu'il déclenche, plus ce que ce mod et vos autres mods actifs définissent. Ce qui reste est un appel qui échouerait à cette ligne : une fonction que Build 42 a supprimée, une méthode renommée, un événement qui ne se déclenche plus, une faute de frappe. Les constats sont regroupés en fonctions globales, méthodes et événements, chacun avec son fichier et sa ligne, et un « vouliez-vous dire » quand un nom connu est proche. Cliquez sur l'un d'eux pour l'ouvrir dans votre éditeur. Un appel que le fichier nomme aussi sans l'appeler (`if foo then`) compte comme protégé et est laissé de côté.

La première vérification lit les fichiers du jeu et prend quelques secondes ; ensuite, elle est gardée par build. Elle ne va que par nom : un nom construit pendant que le jeu tourne n'est pas vu, et un mod qui contient du Java peut définir des noms que le Lua ne peut pas montrer (le résultat le dit). Ouvrez la ligne pour juger.

## Brouillons de traduction

L'onglet **Traductions** montre la langue dans laquelle le mod est écrit, combien de textes il a, et ses langues par groupes : celles où il manque des textes, les complètes, et les dossiers du mod qui ne sont pas des langues du jeu. Cochez les langues voulues et appuyez sur **Écrire des brouillons pour…** : le Studio écrit un brouillon de chaque texte qui manque à chaque langue, traduit par le même service que celui qui traduit les descriptions des mods (**Traduction** dans **Paramètres de l'app**), dans le propre dossier `Translate/<LANG>/` du mod. Ce qui s'y trouve déjà est laissé tel quel. **Ouvrir le dossier Translate** l'ouvre. Un brouillon est un point de départ, pas une traduction : relisez-le avant de publier.

## Publier sur le Workshop

L'onglet **Workshop** envoie un mod sur le Workshop de Steam via le client Steam lancé sur ce PC. Il utilise les mêmes fichiers que l'outil d'envoi du jeu lui-même, dans `Zomboid/Workshop/<item>/` : `workshop.txt`, `preview.png` et `Contents/mods/<mod>/`.

1. Choisissez un objet dans la liste. Pour un mod à vous dans `Zomboid/mods` qui n'a pas encore d'objet, appuyez sur **Préparer (mod) pour le Workshop** : cela crée le dossier de l'objet avec une copie du mod, un `workshop.txt` tiré de son `mod.info` et son affiche comme aperçu.
2. Remplissez **Titre**, **Visibilité** (**Public**, **Amis uniquement**, **Privé** ou **Non répertorié (par lien)**), **Tags (séparés par ;)** et **Description** (Steam accepte jusqu'à 8000 caractères), et appuyez sur **Enregistrer workshop.txt**.
3. **Voir les fichiers** montre ce qui serait envoyé et ce qui a changé depuis le dernier envoi depuis le Studio. Un `.workshopignore` dans le dossier de l'objet (un motif par ligne, comme `*.psd` ou `.git/`) exclut des fichiers.
4. Écrivez une note de mise à jour si vous voulez, et appuyez sur **Publier sur le Workshop** (ou **Envoyer la mise à jour** pour un objet déjà publié). On vous demande d'abord. Steam doit être lancé et connecté au compte qui possède l'objet.

Ce qui est envoyé, c'est ce que dit `workshop.txt` : une modification faite sur la page Steam est remplacée au prochain envoi, comme le fait le jeu. Si Steam garde un nouvel objet masqué tant que vous n'avez pas accepté son accord juridique du Workshop, le résultat le dit, avec un lien. **Voir la page Workshop** ouvre l'objet dans votre navigateur. Le Studio n'envoie pas le mod de quelqu'un d'autre : quand l'objet contient un mod auquel vous êtes abonné, il le dit et le bouton reste désactivé. Le bouton corbeille à côté de la liste retire le dossier de l'objet de `Zomboid/Workshop` (vers la Corbeille) ; rien ne change sur Steam.

## Outils de carte

Dans l'onglet **Cartes**, TileZed dessine les tuiles de chaque cellule (bâtiments, pièces, mobilier, sol) ; WorldEd pose les cellules sur la grille du monde et écrit les fichiers de carte que lit le jeu. Ils viennent de The Indie Stone en un seul zip à décompresser n'importe où. Le Studio les cherche aux endroits habituels (votre dossier utilisateur, Documents, Bureau, Téléchargements, le lecteur C:) et dans le dossier que vous tapez dans le champ sous chacun d'eux. Quand il en trouve un, **Lancer TileZed** ou **Lancer WorldEd** le lance depuis son propre dossier. Les cellules d'un mod de carte vont dans `media/maps/<name>` du mod.

## Bon à savoir

- Le rechargement demande le jeu lancé avec le mod pont activé ; les autres outils n'ont pas besoin du jeu.
- Le rechargement dans le jeu fonctionne sans redémarrer ; une constante modifiée dans une table garde son ancienne valeur jusqu'à un Reset Lua complet.
- Tous les outils sauf **Workshop**, **Cartes** et **Nouveau mod** demandent un mod choisi dont le Studio connaît le dossier.
