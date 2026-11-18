# Diagnostic

Le Diagnostic rassemble ce que l'app sait de vos mods, du jeu et du serveur. Quatre boutons en haut : **Vue d'ensemble**, **Mods**, **Jeu** et **Serveur**. Il s'ouvre toujours sur la Vue d'ensemble ; chaque partie se souvient du dernier onglet consulté.

## Vue d'ensemble et le Docteur

- Trois cartes, une par partie, chacune disant en une ligne comment ça va. **Mods** : quelque chose qui casse le jeu, quelque chose qui vaut la peine d'être corrigé, ou tout est en ordre. **Jeu** : ouvert ou fermé, combien de mémoire il utilise, le dernier démarrage, la limite de mémoire et si une session a manqué de mémoire. **Serveur** : s'il y en a un qui tourne sur ce PC et comment il va. Cliquez sur une carte pour ouvrir cette partie.
- En dessous, **le Docteur** : il demande ce qui ne va pas (**Le jeu plante ou ne démarre pas**, **Un mod ne marche pas**, **C'est lent**, **Autre chose**) et vous guide pas à pas avec ce que l'app sait déjà : ce que la vérification des mods a trouvé, les erreurs du Centre de surveillance et les fois où le jeu a manqué de mémoire. Quand une étape vous envoie ailleurs, en revenant vous retrouvez la même étape. **Recommencer** revient à la question.
- Le bouton **Docteur**, tout en haut, ouvre cette page depuis n'importe quel écran.

## Mods

Vérifie vos mods actifs comme le jeu les lit, et liste chaque problème trouvé avec un bouton pour le corriger. Il regarde ce que votre build peut charger, les prérequis manquants ou désactivés, l'ordre de chargement, les mods incompatibles, les fichiers que deux mods modifient tous les deux, et les mods qui ont rempli la console lors de votre dernière partie.

- En haut : votre **Build du jeu** et le nombre de mods actifs.
- **Notes du Studio** (par exemple **Notes du Studio : 22 mods**) : les règles d'ordre de chargement, les paires incompatibles et les problèmes connus que l'app contient. Quand vous avez marqué des avertissements avec **N'arrive plus**, un bouton à côté dit combien et les réaffiche tous.
- **Revérifier** relance la vérification. Elle se lance aussi d'elle-même quand votre liste change.
- Cinq onglets ; les trois premiers disent combien de lignes demandent quelque chose :
  - **À faire maintenant** : ce qui casse le jeu ou le cassera (prérequis manquants ou désactivés, ordre de chargement, mods que votre build ne peut pas charger, paires incompatibles). **Corriger N problème(s) automatiquement** se trouve ici. Replié en dessous : ce que le Studio a déjà corrigé et ce qui est seulement bon à savoir.
  - **Conflits** : deux mods qui fournissent le même fichier, objet, recette ou texte ; le jeu en utilise un, et lequel, c'est à vous de choisir. **Fusionner N fichiers automatiquement** se trouve ici.
  - **Sécurité** : ce qu'un mod pourrait faire au-delà du modding (voir plus bas).
  - **Bilan** : votre liste mod par mod, chacun sous un verdict : **Cassé**, **À choisir**, **Inutile**, **À vérifier** ou **À garder**. Cliquez sur un mod pour voir pourquoi, et sur **Aller** pour aller à l'avertissement.
  - **Temps par mod** : quels mods ralentissent le jeu pendant que vous jouez (voir plus bas).
- Chaque ligne montre le mod, ce qui ne va pas, un code court (comme `missing_dependency`) et ses boutons de correction à droite. **Copier pour un assistant** copie la ligne avec une question, à coller dans n'importe quel assistant de discussion ; rien n'est envoyé depuis le Studio. Une pastille de problème dans la Liste des mods ouvre l'onglet qui la contient.

Si rien ne va mal, vous voyez **Aucun problème trouvé dans vos N mods actifs.**

### Que faire de chaque ligne

Chaque ligne dit l'une de quatre choses, comme dans la Liste des mods :

- **Vous devez agir** : le jeu casse ou ne charge pas le mod tant que vous ne faites rien. Commencez ici.
- **À faire** : ça marche, mais ça vaut la peine d'être corrigé (mauvais ordre, une mise à jour pas encore téléchargée, un mod qui remplit la console). Le bouton dit ce qu'il change.
- **Bon à savoir** : rien à faire, juste pour info (deux mods définissent la même chose, des mods mis à jour depuis votre dernière partie).
- **Déjà corrigé par le Studio** : n'y touchez pas, c'est ainsi que ça fonctionne (une copie compatible, un fichier dans votre paquet). Chacun peut être annulé si besoin.

Chaque section se replie en cliquant sur son titre.

### Les problèmes qu'il trouve

- `missing_mod` (Non installé) : un mod de votre liste n'est pas sur le disque. Si un mod installé a presque le même id, il propose de l'activer à la place.
- `not_for_build`, `outside_version_range`, `invalid_mod_info` : le jeu ne chargera pas ce mod sur votre build, ou son `mod.info` est cassé.
- `missing_dependency` : le mod nécessite un autre mod qui n'est pas installé.
- `dependency_not_enabled` : le mod requis est installé mais désactivé.
- `dependency_after`, `load_after_order`, `load_before_order` : deux mods sont dans le mauvais ordre.
- `dependency_unavailable` : un mod requis ne peut pas lui-même se charger.
- `incompatible` : le mod dit qu'il ne fonctionne pas avec un autre mod que vous avez activé.
- `duplicate_entry` : le même mod est deux fois dans la liste.
- `two_mod_managers` : un autre gestionnaire de mods qui dessine l'écran Mods du jeu est activé à côté de Studio Link.
- `workshop_update_pending` : Steam a une version plus récente pas encore téléchargée. Les serveurs refusent les joueurs dont les versions de mods diffèrent.
- `file_conflict` : deux mods contiennent le même fichier et le jeu n'utilise que le dernier.
- `function_overridden` (**Écrase un autre mod**) : un mod remplace une fonction du jeu, ou d'un autre mod, après qu'un autre mod l'a modifiée, si bien que cette modification ne s'exécute jamais. Quand beaucoup de fonctions d'un mod sont coupées par un autre (deux versions du même mod activées en même temps), cela fait une seule ligne. Avec Studio Link activé, c'est vérifié dans le jeu en cours : la ligne dit **Vu dans le jeu en cours**, ou `function_seen_other` (**Une autre version dans le jeu**) apparaît quand le jeu a la version d'un autre mod.
- `java_link_broken` (**Java plus ancien que cette build**) : le Java du mod utilise des classes, méthodes ou champs que le jeu installé n'a plus (une mise à jour les a changés). Ce code échoue quand il s'exécute. Si le Centre de surveillance le montre en échec, cherchez une mise à jour du mod ou désactivez-le ; s'il fonctionne, le mod vérifie d'abord.
- `tiledef_clash` : deux mods rangent leurs tuiles sous le même numéro `tiledef` avec des planches différentes, si bien que certaines tuiles s'affichent comme celles de l'autre mod ou disparaissent. Gardez-en un seul activé, ou demandez à un auteur de changer le numéro. Les lignes `file_conflict` disent aussi si le journal de la dernière session (avec cette même liste) confirme quelle copie le jeu a utilisée.
- `game_folder_changed` (**Le dossier du jeu a changé**) : pendant que le jeu tournait, des fichiers de son propre dossier ont changé, là où le jeu n'écrit jamais. L'app compare le dossier du jeu, le dossier Zomboid (sans les sauvegardes ni les journaux) et celui de ZombieBuddy à l'ouverture et à la fermeture du jeu, seulement quand l'app est ouverte.
- `script_definition_conflict` et `translation_conflict` : deux mods définissent différemment le même objet, la même recette ou le même nom ; celui qui se charge en dernier l'emporte. Les lignes finissant par `_stale` veulent dire que vous l'avez déjà fusionné, mais qu'un mod a changé depuis ; `_settled` veut dire que votre paquet le règle déjà.
- `floods_console` (affiché dans la Liste des mods comme **Remplit la console**) : lors de votre dernière session, ce mod a écrit plus de 1 000 lignes d'erreurs dans la console. Cela ralentit le jeu et cache les erreurs des autres mods. Regardez-les dans le Centre de surveillance, ou désactivez le mod.
- `user_rule_order` : une de vos propres règles d'ordre de chargement n'est pas respectée.
- `community_rule_order`, `community_incompatible`, `community_known_issue`, `community_successor` : ce que les règles et l'index de mods du Studio disent de vos mods. Un problème connu a **N'arrive plus** : il n'est plus affiché sur ce PC.
- `learned_issue` : une erreur qu'un mod a donnée dans 3 de vos sessions ou plus avec la même version (voir « Ce que ce PC apprend »).
- `package_built_for_other_list` : votre paquet du Studio a été construit avec une autre liste de mods que ce profil.
- `package_file_stale` : votre paquet contient une ancienne copie d'un fichier d'un mod, et le mod a modifié ce fichier depuis. Le jeu lit l'ancienne copie.
- `package_file_differs`, `package_file_orphan` : votre paquet contient un fichier qu'un seul mod de la liste fournit, ou qu'aucun mod de la liste ne fournit. Retirez-le si vous n'en avez plus besoin.
- `security_executable`, `security_script`, `security_java`, `security_lua` : ce qu'un mod pourrait faire au-delà du modding, dans l'onglet **Sécurité** (voir plus bas).
- `compatible_copy`, `shadowed_copy`, `updated_since_last_play` : des notes sur la copie d'un mod utilisée et ce qui a changé.

Les lignes sur une même définition peuvent s'ouvrir avec **Par fichier** pour voir le fichier de chaque mod. Cliquez sur une définition pour l'ouvrir dans la Fusion.

### Les boutons de correction

- **Activer X** ou **Désactiver X** : active ou désactive un mod. La désactivation demande d'abord ; le mod reste installé.
- **Placer X avant Y** : change l'ordre de chargement. Sur une ligne « à vous de choisir » (deux mods définissent la même chose), il devient **Utiliser plutôt la version de Y**.
- **Retirer X de la liste** : enlève une entrée de la liste.
- **Chercher « X » sur le Workshop** et **Ouvrir la page du Workshop** : ouvrent le Workshop de Steam dans votre navigateur.
- **Faire une copie compatible** : fait une copie adaptée d'un mod du Workshop dans `Zomboid/mods` pour votre build. Le résultat montre ce qui a été changé et ce qui peut encore casser. Lancez le jeu depuis le Studio pour qu'il utilise la copie. **Revenir à l'original** retire la copie.
- **Garder votre version** : enregistre le mod tel qu'il est aujourd'hui, avant qu'une mise à jour du Workshop n'arrive. Vous pouvez la remettre depuis la Liste des mods (clic droit, Versions conservées).
- **Fusionner automatiquement** et **Ouvrir dans la Fusion** : règlent un conflit de fichier ou de définition dans votre paquet de correctifs, qui se charge en dernier.
- **Retirer du paquet** : retire un fichier de votre paquet de correctifs, après confirmation ; le jeu relit le fichier du mod lui-même au prochain lancement.
- **Activer le correctif X du shim** : active un correctif du shim Build 42 dont un problème connu a besoin.

Une ligne verte confirme chaque modification, avec **Enregistré dans votre liste de mods.**

### En corriger plusieurs à la fois

- **Corriger N problème(s) automatiquement** active les prérequis manquants et avance des mods là où il le faut. Il ne désactive jamais rien et saute les lignes « à vous de choisir ». Il demande d'abord, et la Liste des mods peut l'annuler.
- **Fusionner N fichiers automatiquement** fusionne dans votre paquet de correctifs chaque fichier en conflit qui n'a pas de modifications qui se chevauchent. Les fichiers où les deux mods modifient les mêmes lignes sont listés pour que vous décidiez dans la Fusion.

### Les règles et l'index du Studio

Le Studio contient des règles d'ordre de chargement, des paires de mods qui ne vont pas ensemble, et un index de problèmes connus avec leur correctif. Ils viennent avec chaque version de l'app ; rien n'est téléchargé, et rien n'est envoyé. Ils apparaissent sous forme de lignes `community_*`. Un problème connu est mis de côté tout seul quand les fichiers du mod montrent qu'il a disparu (une mise à jour a corrigé le fichier concerné), ou quand le mod a changé après que le problème a été vu et que sa ligne de journal n'apparaît plus dans votre session. **N'arrive plus** en met un de côté à la main.

### Ce que ce PC apprend

En plus de l'index qu'elle contient, chaque app apprend de son propre PC. En arrière-plan, elle lit les journaux de session du jeu et du serveur que le jeu garde (`Zomboid/Logs`) et note quels mods étaient activés, dans quelle version, et quelles erreurs le jeu leur a attribuées (les mêmes que celles du Centre de surveillance). Elle garde cela dans `Zomboid/PZModStudio_Learned.json` et n'envoie rien.

Quand un mod donne la même erreur dans 3 sessions ou plus, avec la même version et sur la build installée, il apparaît dans le Diagnostic comme **Appris sur ce PC** (`learned_issue`), avec le compte comme preuve : « dans 8 de vos 14 sessions ». Quand l'index de l'app parle déjà de ce problème, seule la ligne de l'index s'affiche. Un correctif Java que le journal attribue à plusieurs mods à la fois n'est pas appris, car on ne peut pas dire à qui il appartient. Quand le mod se met à jour, le compte repart de zéro ; quand ses 5 dernières sessions se sont passées sans l'erreur (corrigée, ou un correctif qui a marché), elle n'est plus affichée. **N'arrive plus** la retire à la main. La première page du Diagnostic dit de combien de sessions il a appris.


### Le Docteur et le cerveau

La première page du Diagnostic a **Demander au Docteur**, une discussion qui répond sans IA : elle reconnaît ce que vous demandez à ses mots et construit la réponse à partir de ce que ce PC a appris et des outils de l'app (les mêmes que ceux du serveur MCP). Chaque réponse dit de quel outil elle vient. Demandez-lui ce qu'il sait, pourquoi le jeu s'est fermé, ce qui a changé quand une erreur a commencé, quels mods échouent, si le chargement est plus lent, si les correctifs ont marché, ce qu'a fait la dernière mise à jour du jeu, ou nommez un mod pour voir son historique.

Il pardonne les fautes de frappe et les autres façons de le dire (« ça a planté », « ça a gelé », « qu'est-ce qui a chnagé »), lit les dates (« qu'est-ce qui a changé depuis hier ? », « cette semaine », « depuis le 12/11 »), « lequel met le plus de temps à charger ? » ou « quel mod donne le plus d'erreurs ? », et suit la conversation : après un mod, vous pouvez dire « et depuis quand ? », « désactive-le », « note-le » ou « montre-le ». En cas de doute, il demande « lequel de ceux-ci ? » avec des boutons ; quand il ne comprend pas et que vous lui montrez avec un bouton ce que vous vouliez dire, il retient cette phrase sur ce PC. Chaque réponse commence par la conclusion, puis un conseil avec sa preuve (par exemple que presque tous les plantages venaient du PC à court de mémoire avec le jeu et un serveur ouverts), les détails repliés, et des boutons pour aller à la Liste des mods ou à la partie du Diagnostic concernée.

Aussi :

- **Quel mod en est la cause ?** Il compare les sessions avec et sans chaque mod : « avec X, 3 sur 7 se sont mal passées ; sans lui, 24 sur 196 », et dit à quel point il en est sûr (assez sûr, probable, peut-être le hasard). Un mod présent dans presque toutes les sessions ne peut pas être comparé et n'est pas nommé.
- **Chercher par moitiés** depuis la discussion : il désactive la moitié de vos mods (ce dont les autres ont besoin reste activé), vous laisse jouer l'essai et vous répondez « ça arrive encore » ou « ça n'arrive plus » jusqu'à ce qu'il en reste un. Votre liste est gardée et revient à la fin.
- **Le compte rendu** : quand vous ouvrez la discussion après avoir joué, il raconte comment s'est passée votre dernière session (temps joué, comment elle s'est terminée, nouvelles erreurs, correctifs qui ont marché). « Que s'est-il passé cette semaine ? » fait de même pour une période.
- **Questions sur l'app** (« comment je crée un profil ? », « c'est quoi une copie compatible ? ») : il répond avec le paragraphe de ce guide et un bouton vers cet écran.
- **Groupes, profils et Workshop** : « VP Base » dit comment va ce groupe, « profil Knox Lives » si ses dernières sessions se sont bien passées ; « y a-t-il une nouvelle version de X ? » et « que disent les commentaires de X ? » lisent sa page.
- **Plusieurs questions à la fois** (« qu'est-ce qui a changé et quels mods échouent ? »), des **surnoms** qu'il apprend quand vous choisissez parmi plusieurs mods, **« annule ça »** pour sa dernière modification, et la **mémoire** : la fois suivante, il vous rappelle de quel mod vous avez parlé et comment il va maintenant.

**Avec le jeu (Studio Link) :**

- Sur le menu principal du jeu, la carte PZ Mod Studio (en haut à droite) montre ce que le Docteur dirait en premier : si les dernières sessions avec cette liste se sont mal passées, si la dernière a planté, quelles erreurs se répètent. L'app l'écrit à chaque vérification, donc la carte dit la dernière chose qu'elle savait même quand l'app est fermée.
- Le Link tient un journal de bord de la sauvegarde chargée et indique si vous êtes revenu au menu (plutôt que de fermer la fenêtre). Ainsi chaque session dit « sauvegarde X · revenu au menu », y compris celles jouées avec l'app fermée.
- Dans le menu pause (Échap), il y a **Marquer ce moment** : quand quelque chose de bizarre arrive, appuyez dessus. Demandez ensuite au Docteur « que s'est-il passé quand j'ai marqué ? » et il vous dit ce que le journal du jeu a écrit dans cette minute (d'une minute et demie avant à une demi-minute après).

Il peut aussi changer des choses, toujours avec votre **Le faire** : créer, renommer ou supprimer des groupes du profil actif et y mettre ou en retirer des mods (« groupe A, B dans Cartes », « retire A du groupe Cartes », « renomme le groupe Cartes en Monde », « supprime le groupe Cartes »), proposer des groupes pour vos mods actifs qui n'en ont pas (ceux qui partagent un nom ou ce qu'ils nécessitent), et écrire ou retirer des **notes du Studio** (« note sur A : texte », « retire la note du Studio de A », « note ce que tu as appris »). Les notes du Studio sont gardées à part des vôtres et affichées en violet avec leur propre bouton pour les retirer ; vos notes, il ne les écrit, ne les modifie ni ne les retire jamais.

Ce que le cerveau tire des sessions :

- **Ce qui a changé** : quand une erreur se répète, quand elle a commencé et ce qui était différent de la dernière session sans elle (mods ajoutés, retirés ou mis à jour, sa place dans l'ordre, la build du jeu). Il le dit comme ce qui a changé à ce moment-là, pas comme la cause. Cela s'affiche sur la ligne du Diagnostic et dans l'historique du mod.
- **Correctifs mémorisés** : chaque correctif appliqué depuis le Diagnostic est noté, et les sessions suivantes disent si l'erreur du mod a cessé.
- **Avant de jouer** : quand les dernières sessions avec presque la même liste n'ont même pas atteint le menu ou ont manqué de mémoire, l'app le dit quand vous appuyez sur Jouer, avec ce qui diffère de la dernière qui est arrivée dans une sauvegarde, et demande s'il faut jouer quand même.
- **L'historique d'un mod** : dans la Liste des mods, la section **Sur ce PC** du panneau de droite.
- **Chargement plus lent** et une **mise à jour du jeu** qui a fait échouer plusieurs mods d'un coup.
- **Vrais plantages** : quand Java tombe, il laisse un rapport (`hs_err_pid…log`) dans le dossier du jeu ; le cerveau le lit et le relie à sa session (« il est tombé : le PC a manqué de mémoire » ou « une erreur Java fatale dans cette bibliothèque »). Fermer le jeu avec X ou Alt+F4 n'en laisse aucun, donc cela ne compte jamais comme un plantage.

### Sécurité

L'onglet **Sécurité** liste ce qu'un mod pourrait faire au-delà du modding, d'après ses fichiers : programmes ou scripts qu'il contient, Java capable de lancer des programmes ou d'accéder à internet, et Lua qui appelle ce que les scripts du jeu lui-même n'appellent jamais. Rien de tout cela n'est un verdict : beaucoup de mods font ces choses pour de bonnes raisons. La plupart des lignes sont des Infos ; un Avertissement marque un motif qu'aucun mod n'a de bonne raison d'avoir. La ligne d'un mod du Workshop a **Ouvrir la page du Workshop**.

### Temps par mod

Dans **Mods** > **Temps par mod** : quels mods ralentissent le jeu pendant que vous jouez. **Mesurer** place un petit mod du Studio (PZ Mod Studio Profiler) en premier dans votre liste de mods et dans la dernière sauvegarde jouée ; lancez le jeu (redémarrez-le s'il était ouvert) et jouez quelques minutes comme d'habitude. Chaque gestionnaire d'événement de chaque mod est chronométré par le jeu et additionné par mod : la part du temps réel qu'il a prise, les millisecondes par seconde, et son plus long appel (un appel long, c'est un gel que vous voyez). Ouvrez une ligne pour voir les gestionnaires qui coûtent le plus, avec leur fichier et leur ligne. **Arrêter la mesure** retire le profileur de la liste et de la sauvegarde ; les derniers chiffres restent.

Les fenêtres, barres et actions chronométrées d'un mod comptent aussi (comme Class:render, Class:update) ; chaque fonction ne compte que son propre temps. Le profileur ajoute un peu de temps à chaque gestionnaire, alors arrêtez-le quand vous avez fini.

## Jeu

Ce que l'app sait du jeu lui-même, en dehors des mods. Cinq onglets : **En direct**, **Mémoire**, **Démarrage**, **Images** et **Disque**.

### En direct

Ce que le jeu utilise maintenant : en haut, la mémoire libre du PC (la jauge de la barre du haut mène à cet onglet). En dessous, une carte par jeu ouvert : la mémoire face à sa limite Java, le CPU, l'écran où il se trouve (menu, connexion, en jeu), la dernière ligne de son journal, combien d'erreurs il a, et des boutons pour ouvrir son dossier et son journal. Avec le jeu fermé, il montre ce qu'on sait de la dernière fois : le dernier démarrage, la limite de mémoire, les sessions qui ont manqué de mémoire et les fois où Windows a manqué de mémoire.

### Mémoire

Dans **Jeu** > **Mémoire** : combien de mémoire le Java du jeu peut utiliser (`-Xmx` dans `ProjectZomboid64.json`), sur celle du PC, avec une suggestion pour vos mods actifs et un avertissement quand une session récente a manqué de mémoire. Choisissez une valeur (de 2 Go à la moitié de la mémoire du PC) et **Appliquer** avec le jeu fermé ; cela prend effet au prochain démarrage, et une copie du fichier d'avant la modification va dans `PZModStudio_Backups`.

Chaque profil garde sa propre valeur : activer un profil donne au jeu sa mémoire. Un profil qui n'en a pas utilise la dernière valeur choisie. Si une mise à jour du jeu remet sa propre valeur, le Studio remet la vôtre la prochaine fois qu'il lance le jeu.

Quand Windows lui-même a manqué de mémoire avec le jeu ouvert dans les 14 derniers jours, c'est listé ici aussi, avec ce qui aide (un fichier d'échange plus grand, fermer d'autres gros programmes). Ce n'est pas la mémoire Java du jeu, donc augmenter la valeur ci-dessus ne règle pas cela.

### Démarrage

Dans **Jeu** > **Démarrage** : combien de temps le jeu a mis pour atteindre le menu principal, étape par étape (fichiers des mods, scripts, Lua, modèles, etc.), d'après le journal que chaque lancement laisse. Chaque démarrage est comparé à votre démarrage habituel avec à peu près autant de mods ; une étape en orange a pris bien plus longtemps que d'habitude. Choisissez un démarrage précédent pour le voir.

### Images et saccades

Dans **Jeu** > **Images** : les temps d'image pendant que vous jouez (fps, images de plus de 25, 50 et 100 ms), les chunks chargés et, par mod, les gestionnaires qui ont retenu une image le plus longtemps, avec fichier et ligne. C'est le même profileur que **Mods** > **Temps par mod** qui les compte, alors appuyez d'abord sur **Mesurer** là-bas et jouez quelques minutes. **Relire** lit les chiffres les plus récents.

### Disque

En haut, **Libérer de l'espace** : **Vider le cache** supprime ce que le Studio garde pour ne pas relire ou redemander la même chose (les vérifications de chaque mod, ce que Steam a dit, les images de carte, les traductions) ; il le reconstruit tout seul et ne touche jamais aux mods, sauvegardes, profils ou notes. Après un test multijoueur, **Dossier du joueur 2** envoie le dossier `Zomboid2` du second jeu (avec des copies des mods du serveur) à la Corbeille ; les mods en développement liés là ne sont pas touchés, et le test suivant le recrée. Pas pendant que le test tourne.

Dans **Jeu** > **Disque** : combien prend chaque mod installé, compté par ce qui va ensemble (un objet du Workshop avec tous ses mods, un dossier dans `Zomboid/mods`). La mesure prend quelques secondes.

- **Rien ne l'utilise** (orange) : aucun profil ne l'a, activé ou désactivé ; aucune sauvegarde ne le nomme dans sa liste ; aucun serveur enregistré ne l'a chargé ; il n'est pas dans la liste active du jeu.
- **Désactivé partout** (bleu clair) : un profil le garde, désactivé, et aucune sauvegarde ni aucun serveur ne l'utilise.
- **Reste** : un dossier Workshop d'un objet auquel vous n'êtes plus abonné ; Steam en laisse parfois.
- **Votre envoi** : un mod à vous dans `Zomboid/Workshop`. Affiché, jamais proposé à la suppression.

Cochez ceux dont vous voulez vous débarrasser. Un objet du Workshop s'en va avec **Se désabonner de N objets** (Steam doit être lancé, et supprime les fichiers lui-même). Un dossier local ou un reste s'en va avec **Envoyer N dossiers à la Corbeille** : on peut le restaurer depuis la Corbeille. Chacun demande d'abord, avec les noms. Rien ne va à la Corbeille pendant que le jeu est ouvert.

En dessous, **Dossiers du jeu et du Studio** : combien prennent le jeu, les mods du Workshop, les mods locaux, les sauvegardes, les copies de secours des sauvegardes, les versions de mods conservées, les copies de secours des listes de mods, le cache, les téléchargements à moitié installés et les journaux, avec un bouton pour ouvrir chaque dossier. Rien n'est supprimé depuis là.

## Serveur

Le serveur qui tourne sur ce PC (celui que vous lancez depuis Serveurs, ou celui d'un test multijoueur), avec la même carte : la mémoire face à sa limite, le CPU, s'il a démarré, la dernière ligne de son journal et combien d'erreurs il a. Pendant un test multijoueur, ses joueurs s'affichent aussi. Sans serveur lancé, un bouton mène à **Serveurs** pour en démarrer un.

Un serveur sur un autre PC que vous rejoignez ne peut pas être mesuré d'ici : seulement celui qui tourne sur ce PC.

## Bon à savoir

- Les corrections dans **Mods** sont enregistrées dans la liste de mods du profil actif.
- Les fichiers fusionnés et les correctifs du shim prennent effet au prochain démarrage du jeu.
- « Remplit la console » se mesure sur le `console.txt` de la dernière session, donc cela se met à jour après que vous avez joué.
- Cliquer sur une pastille de problème dans la Liste des mods ouvre cet écran sur cette ligne.
