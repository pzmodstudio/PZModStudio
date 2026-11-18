# Centre de surveillance

Le Centre de surveillance affiche le journal du jeu au fur et à mesure qu'il s'écrit et transforme chaque erreur en fiche qui dit quel mod le jeu a mis en cause et ce que vous pouvez y faire. Il suit `console.txt` que vous ayez lancé le jeu depuis le Studio ou depuis Steam.

## Ce que vous voyez

- À gauche, la visionneuse de journaux, avec un onglet par journal.
- À droite, **ERREURS ET AVERTISSEMENTS** : les fiches d'erreur, et au-dessus le panneau **Trouver le mod responsable**.
- En haut, **Voir comment le jeu tourne** ouvre Diagnostic > Jeu > En direct : mémoire, CPU et l'écran où se trouve le jeu.
- Au-dessus du journal, pendant que le jeu tourne (vue Complète), le panneau **En direct** : voir plus bas.
- En haut, **📁 Dossier des journaux** ouvre dans l'Explorateur Windows le dossier contenant `console.txt` et le sous-dossier `Logs`.
- À côté, le bouton du filtre anti-spam. Tant qu'il affiche **Filtre anti-spam : ACTIF**, il masque les lignes sur les traductions manquantes ; appuyez dessus pour passer à **Filtre : TOUT AFFICHER**.

Si le dossier du jeu n'est pas encore défini, l'écran n'affiche que **Configurer les chemins dans les Paramètres**.

## En direct (pendant que le jeu tourne)

Dans la vue Complète, pendant que le jeu est ouvert, un panneau **En direct** se trouve au-dessus du journal. Sa ligne de titre résume tout (FPS, mémoire Java, CPU, erreurs Lua) ; un clic le replie. Il est relu toutes les 2 secondes :

- **FPS, dernières minutes** : une courbe des 10 dernières minutes (passez la souris dessus pour lire un moment) ; la ligne pointillée correspond à 30 FPS. En dessous, l'image moyenne et l'image la plus longue des 10 dernières secondes, la mémoire que Java utilise face à sa limite, la mémoire de tout le processus avec ce que le PC a de libre, et le CPU. Une seconde courbe suit la mémoire.
- **Temps Lua maintenant, par mod** : les 5 mods dont le Lua a pris le plus de temps sur les 10 dernières secondes, en millisecondes par seconde de jeu. Cela vient du profileur du Studio. Quand le profileur est désactivé, rien n'est chronométré, et le panneau le dit avec **L'activer pour le prochain lancement** : le profileur passe en premier dans la liste de mods (et dans la dernière sauvegarde jouée) et mesure à partir du prochain lancement du jeu.
- **Erreurs Lua de cette session** : combien, et quels mods le jeu a mis en cause, telles que le Studio Link les compte dans le jeu ; en dessous, les fiches d'erreur les plus récentes, marquées **nouveau** quand elles sont arrivées pendant que vous regardiez.
- Une dernière ligne dit si l'affichage du Link est visible dans le jeu et ce que coûte son propre code.

Les FPS, la mémoire Java et les erreurs viennent du Studio Link (version 1.20 ou plus récente), une fois que vous êtes dans une partie. Le panneau disparaît quand le jeu se ferme.

### L'affichage dans le jeu

Le Studio Link peut afficher les mêmes chiffres par-dessus le jeu : FPS et temps d'image, mémoire Java face à sa limite, CPU, les 5 mods avec le plus de temps Lua sur les 10 dernières secondes (avec le profileur activé) et les erreurs Lua de la session avec les mods mis en cause. Ouvrez-le depuis le menu pause avec **Affichage en direct**, ou donnez-lui une touche dans **Options > Mods > PZ Mod Studio Link** du jeu (il n'en a aucune tant que vous n'en choisissez pas, il ne prend donc jamais une touche du jeu). Faites-le glisser où vous voulez ; il y reste et revient à la partie suivante. Sa dernière ligne dit ce qu'il coûte lui-même, en millisecondes par seconde. La limite de mémoire et le CPU ne sont connus que lorsque PZ Mod Studio est ouvert.

## La visionneuse de journaux

Deux onglets sont toujours là :

- **console.txt (Client)** : le journal du jeu en direct. Le Studio commence à lire à la fin du fichier, vous voyez donc ce qui se passe à partir du moment où le Studio est ouvert. Quand une nouvelle session de jeu commence, le fichier repart de zéro et le Studio le lit depuis sa première ligne. L'onglet garde les 1 500 dernières lignes, même quand vous êtes sur un autre écran.
- **Serveur dédié (en direct)** : le journal du serveur dédié, pendant qu'un serveur que vous avez lancé dans **Serveurs** tourne. Il affiche ses 1 500 dernières lignes et est relu toutes les 2 secondes.

Pour lire d'autres journaux, appuyez sur **Charger un journal**. Le menu liste les journaux trouvés dans votre dossier Zomboid (avec leur taille) ; choisissez-en un pour l'ouvrir dans un nouvel onglet. **📂 Choisir un autre fichier (.txt)...** ouvre n'importe quel fichier texte. Un journal chargé affiche ses 2 000 dernières lignes. Ces onglets se ferment avec leur X ; les deux onglets en direct ne se ferment pas.

Sous les onglets :

- Le champ de recherche ne garde que les lignes qui contiennent ce que vous tapez.
- Les pastilles **Tous**, **🚨 Erreurs**, **🌙 Lua** et **☕ Java** ne gardent que les lignes de ce type. **⚡ Pont** n'apparaît qu'avec les Fonctions pour développeurs activées.
- Le pied de page indique combien de lignes sont affichées sur le total.

Les petits boutons à droite des onglets :

- Le bouton disquette enregistre le contenu de l'onglet dans un nouveau fichier dans `Zomboid/Logs/`.
- Le bouton copier copie dans le presse-papiers les lignes visibles (après filtres).
- Le bouton corbeille vide la visionneuse pour cet onglet. Il ne touche pas au fichier sur le disque.

Les lignes d'erreur s'affichent en rouge, les avertissements en orange.

## Les fiches d'erreur

Une fiche apparaît quand le jeu lève une erreur, ou quand il écrit un avertissement et nomme un mod. Tout le reste reste dans le journal à gauche. Les fiches d'exceptions (rouges) viennent en premier, les avertissements (orange, marqués **avertissement · le jeu a continué**) après. Un même problème n'apparaît qu'une fois, quel que soit le nombre de fois où le jeu le répète ; le nombre comme **600×** dit combien de fois il est dans le journal, et ceux qui se répètent sont ceux qu'il vaut mieux corriger en premier.

Au-dessus des fiches, un badge compte les exceptions et un autre les avertissements (chaque fiche dit s'il y a quelque chose à faire), avec une ligne qui résume le journal. Quand le Studio se souvient des sessions précédentes, chaque fiche dit **nouveau** ou **vu dans N sessions depuis** une date, et **Seulement les nouveaux** masque le reste.

Une fiche peut montrer :

- **Le nom du mod**, quand le jeu l'a nommé sur la ligne d'erreur ou dans la pile d'appels en dessous.
- **appelé depuis ...** : les mods dont le code a appelé celui qui a échoué, lus dans la pile d'appels du jeu lui-même. La faute peut être chez l'un ou chez l'autre.
- **Extrait du journal** : la ligne elle-même.
- Pour un `require` qui a échoué, l'endroit où ce module se trouve vraiment maintenant, quand le Studio le trouve.
- Que faire ensuite, ou une note disant que le shim Build 42 le couvre déjà.
- Quand l'erreur est dans une copie que contient votre paquet du Studio : une note quand le mod a modifié ce fichier après la copie, ou quand plus aucun mod de votre liste ne fournit ce fichier.

Quand une nouvelle session de jeu commence, les fiches de la précédente sont effacées. **Abandonner** les efface à la main.

## Corriger depuis une fiche

- **Corriger le chemin** (ou **Corriger le chemin dans File.lua**) : apparaît quand un mod demande un module par un ancien chemin et que le Studio connaît le nouveau. Il réécrit le `require` dans chaque fichier des mods actifs qui le demande et les publie dans votre paquet du Studio. Le mod n'est pas modifié et reste activé. Une ligne verte dit ce qui a été corrigé ; relancez le jeu.
- **Patcher File.lua** ou **Patcher un de ses fichiers** : ouvre les fichiers de ce mod dans la Fusion de mods pour que vous en corrigiez un. Votre copie va dans un paquet qui se charge en dernier, donc le jeu la lit à la place de celle du mod. Les avertissements sans correctif connu n'ont pas ce bouton.
- **Retirer la copie de File.lua** : apparaît quand l'erreur est dans un fichier que votre paquet du Studio a déjà patché. Cela retire cette copie du paquet (après confirmation), et le jeu relit le fichier du mod lui-même. Toute modification faite à la copie est perdue.
- **Le désactiver** : demande confirmation, désactive le mod et vous emmène à la Liste des mods.
- **Un paquet de fusion peut corriger cela — ouvrir la Fusion de mods** : pour les erreurs auxquelles répond le shim Build 42. Le shim voyage dans un paquet de fusion publié ; la fiche vous dit si le vôtre est déjà actif ou encore un brouillon.

## Rapports

- **Enregistrer le rapport** écrit un fichier Markdown avec l'erreur, la build de votre jeu, le mod et son code, plus les lignes du journal autour. Il est prêt à être donné à un auteur de mod ou à un assistant IA.
- **Copier le rapport** met le même rapport dans le presse-papiers.
- **🔍 Voir dans le journal** met le fichier ou le mot-clé de l'erreur dans le champ de recherche à gauche.
- **Copier la ligne** ne copie que la ligne d'erreur.

## Trouver le mod responsable

Pour les problèmes que le journal ne nomme pas : un gel, une recette qui n'apparaît jamais, une sauvegarde qui ne s'ouvre pas.

1. Ouvrez le panneau. Si le problème n'arrive pas à chaque fois, cochez **N'arrive pas à chaque fois**. Si le journal montre un texte de l'erreur, tapez-le : après chaque partie de test, l'app lit le journal et propose la réponse. Appuyez sur **Commencer à couper la liste en deux**, puis confirmez.
2. Une partie de vos suspects est activée ; ceux que ce PC soupçonne déjà (sessions qui ont mal tourné, erreurs qui se répètent) sont essayés en premier. Les mods qu'un autre mod nécessite restent activés à chaque tour, pour que rien ne casse pour cette raison.
3. Appuyez sur **Lancer le jeu**, jouez comme quand c'est arrivé, et fermez-le.
4. Répondez **Ça arrive encore** ou **C'est réglé**. La liste est de nouveau coupée en deux. Le badge montre le tour et combien de lancements il reste (environ 8 pour une centaine de mods).
5. Quand il reste un seul mod, jouez une fois de plus pour le confirmer.

À la fin, le panneau nomme le mod, montre les lignes que le journal contient à son sujet (avec **Patcher File.lua** quand une ligne nomme un fichier, et **Afficher ses lignes dans la console**), et propose **Patcher un de ses fichiers**, **Le désactiver** ou **Le garder activé**. Votre liste d'origine est remise quelle que soit la façon dont la recherche se termine. **Arrêter et remettre ma liste** l'arrête avant la fin.

Si le dernier mod ne le fait pas à lui seul, la recherche continue toute seule : elle garde ce mod activé et cherche l'autre mod qui le fait avec lui, puis nomme la paire. Si elle ne trouve ni un mod ni une paire, le panneau dit pourquoi : un des mods requis, plus de deux mods, ou pas un mod du tout.

## Bon à savoir

- La recherche écrit elle-même votre Liste des mods. N'activez ni ne désactivez de mods dans la Liste des mods pendant qu'elle tourne, sinon la réponse suivante ne veut plus rien dire.
- Une recherche survit à la fermeture du Studio ; elle est enregistrée dans `Zomboid/PZModStudio_Bisect.json`.
- **Lancer le jeu** dans la recherche lance le jeu normalement, sans le mode debug.
- Les correctifs publiés dans un paquet prennent effet au prochain démarrage du jeu.
