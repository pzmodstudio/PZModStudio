# Sauvegardes

Vos sauvegardes de Project Zomboid et leurs copies de secours. Le Studio fait tout seul une copie de secours d'une sauvegarde aux moments où elle est le plus en danger ; ici, vous voyez ces copies, en faites une vous-même, en gardez une pour de bon, ou en remettez une.

## Les sauvegardes de chaque profil

Chaque profil a ses propres sauvegardes. Une sauvegarde appartient au profil qui était actif quand vous y avez joué la dernière fois avec le Studio ouvert ; une sauvegarde jouée sans lui n'appartient à un profil que si les mods de ce profil sont exactement ceux de la sauvegarde. Une sauvegarde n'est jamais déplacée vers un autre profil.

- Cet écran montre les sauvegardes du profil actif. **Tout afficher (N de plus)** ajoute les sauvegardes des autres profils et celles sans profil, chacune avec une pastille **Profil : <name>** ou **Aucun profil**. **Seulement celles de <name>** revient aux sauvegardes du profil actif.
- La sauvegarde d'un serveur s'affiche sous son adresse (`ip:port`) avec une pastille **En ligne**, et appartient au profil avec lequel vous avez rejoint ce serveur depuis **Serveurs** (le Link ne tourne pas sur un serveur, donc le Studio se fie à sa propre liste de serveurs).
- Dans le jeu, avec le mod Studio Link : l'écran **Charger** liste les sauvegardes du profil actif, avec **Tout afficher** à côté du champ de recherche, et **Continuer** charge la sauvegarde la plus récente du profil actif (il est masqué quand le profil n'en a aucune). Sans le Studio, le jeu liste toutes les sauvegardes comme d'habitude.

## Ce que vous voyez

- En haut, **Ouvrir le dossier des copies de secours** ouvre le dossier où vont les copies de secours (`Zomboid/PZModStudio_SaveBackups`), et la flèche ronde actualise l'écran.
- Une barre avec la place sur le disque : combien est libre, la taille du disque, et combien les copies de secours utilisent. Elle passe à l'orange avec **Peu de place restante** quand votre plus grosse sauvegarde risque de ne plus tenir deux fois.
- Une ligne par sauvegarde : le nom de son monde, puis le mode de jeu, la taille, le nombre de fichiers, le nombre de mods et quand elle a changé pour la dernière fois. Une pastille **faite sur <build>** nomme la build du jeu qui a écrit la sauvegarde ; une build plus récente la met à jour quand vous la chargez.
- Quand les mods de la sauvegarde ont changé depuis sa dernière session, une ligne orange le dit : **Depuis sa dernière session (...)**, avec combien de mods ont été ajoutés, retirés, mis à jour, ou que l'ordre a changé. Survolez-la pour voir les noms des mods.
- Quand la liste de mods de la sauvegarde nomme des mods qui ne sont pas installés, une ligne rouge le dit : **Ne se chargera pas : N mods de sa liste ne sont pas installés**, avec un bouton par mod. Cliquez sur l'un d'eux pour l'ouvrir dans **Parcourir les mods** (l'objet du Workshop quand la liste le nomme, sinon une recherche de son id). Les sauvegardes des serveurs ne gardent pas de liste à elles : le jeu télécharge les mods du serveur quand vous le rejoignez.
- Quand les propres mods de la sauvegarde apportent des cartes que la sauvegarde ne charge pas, une ligne orange le dit : **Ne charge pas N cartes de ses mods**, avec une pastille par dossier de carte. Une sauvegarde garde la liste des cartes avec lesquelles elle a été créée et le jeu n'y ajoute jamais rien, donc un mod de carte activé plus tard se charge sans sa carte (terrain nu là où ses cellules devraient être). Avec le jeu fermé, **Ajouter les cartes à la sauvegarde** fait une copie de secours de la sauvegarde, met ces cartes dans sa liste et déplace le terrain nu déjà créé dans leurs cellules vers le dossier des copies de secours, après confirmation.
- Quand le profil actif a des mods activés que la liste propre de la sauvegarde n'a pas, une ligne verte le dit : **N mods activés dans <profil> ne sont pas dans cette sauvegarde**, avec une étiquette par mod (un globe marque ceux qui apportent des zones de carte). Une sauvegarde se charge avec sa propre liste, pas avec le profil : un mod activé après sa création ne se charge pas avec elle. Jeu fermé, **Les ajouter à cette sauvegarde** sauvegarde la partie puis, après confirmation, les met dans sa liste là où l'ordre de chargement du profil les place ; les cartes qu'ils apportent sont ajoutées comme avec **Ajouter les cartes à la sauvegarde**. Les sauvegardes d'un autre profil ne sont pas touchées.
- **Personnage et véhicules** ouvre ce que contient la sauvegarde, lu dans ses propres fichiers : chaque personnage avec son nom, sa position et s'il est mort, les objets qu'il porte, et chaque véhicule avec sa position, sa distance au personnage et combien d'objets il contient. Les nombres d'objets sont approximatifs.
- À droite de chaque ligne, son état :
  - **Copiée telle quelle** : la copie de secours la plus récente correspond à la sauvegarde.
  - **Modifiée depuis la copie de secours du ...** : la sauvegarde a changé après sa dernière copie de secours.
  - **Pas de copie de secours** : il n'y en a pas encore.
  - **Copie en cours...** pendant qu'une copie de secours se fait.

## Quand le Studio fait une copie de secours

- **Quand vous l'avez quittée** : quand vous revenez au menu principal, ou quand le jeu se ferme alors que vous êtes dans la sauvegarde.
- **Avant son chargement avec des mods modifiés** : le mod Studio Link la demande quand vous chargez une sauvegarde dont les mods ne sont pas ceux de sa dernière session, ou ont été mis à jour depuis.
- **Avant la modification de sa liste de mods** : quand la liste de mods de la sauvegarde va être modifiée dans le jeu, ou avant **Ajouter les cartes à la sauvegarde**.
- **Avant une restauration** : la sauvegarde telle qu'elle était, juste avant que vous ne restauriez une copie de secours plus ancienne.

Une sauvegarde qui n'a pas changé depuis sa copie de secours la plus récente n'est pas zippée à nouveau.

## Faire une copie de secours vous-même

Appuyez sur **Copie de secours maintenant** sur la ligne. La copie de secours est un zip qui est relu après avoir été écrit ; les copies vérifiées affichent **vérifiée**. Si la sauvegarde n'a pas changé, aucun nouveau zip n'est fait : la copie de secours la plus récente est marquée comme gardée à la place. Le bouton est grisé quand le disque n'a pas la place pour la sauvegarde plus un peu de marge.

## Les copies de secours d'une sauvegarde

Cliquez sur la flèche à gauche d'une ligne pour ouvrir ses copies de secours. Chacune montre quand elle a été faite, pourquoi (**faite par vous**, **quand vous l'avez quittée**, ...), la taille du zip, la taille de la sauvegarde, les fichiers et les mods. Pour chaque copie de secours :

- L'épingle la garde : une copie de secours gardée n'est jamais retirée pour faire de la place. Appuyez de nouveau pour en refaire une copie automatique.
- **Restaurer** remet cette copie de secours comme sauvegarde, après confirmation.
- Le bouton corbeille (**Retirer cette copie**) la supprime, après confirmation. Cela ne peut pas être annulé.

## Restaurer une sauvegarde

1. Fermez Project Zomboid. Une restauration est refusée pendant que le jeu tourne.
2. Ouvrez les copies de secours de la sauvegarde et appuyez sur **Restaurer** sur celle que vous voulez.
3. Confirmez. Le Studio vérifie la copie de secours, fait une copie de secours de la sauvegarde telle qu'elle est maintenant (pour que vous puissiez annuler), décompresse la copie à côté de la sauvegarde et seulement ensuite les échange.

## Bon à savoir

- Les 5 copies de secours automatiques les plus récentes de chaque sauvegarde sont gardées ; les plus anciennes partent quand une nouvelle est faite. Les copies de secours que vous faites ou épinglez restent jusqu'à ce que vous les supprimiez.
- Les copies de secours automatiques demandent que PZ Mod Studio soit ouvert pendant que vous jouez, et que le mod Studio Link soit activé dans le jeu (il dit à l'app quand vous entrez dans une sauvegarde et quand vous la quittez).
- La sauvegarde d'un serveur fait l'objet d'une copie de secours quand le jeu se ferme avec le Studio ouvert, si elle a changé pendant cette session.
- Avec l'app fermée, le mod Link avertit avant le chargement que la sauvegarde ne peut pas être copiée, et vous laisse continuer sans copie de secours.
- Le mod Link peut aussi jouer une sauvegarde sur une copie (`<world>_copy`), que l'app fait ; l'original reste tel quel.
- Quand vous chargez une sauvegarde dans le jeu, le Studio Link pose d'abord la même question : **Les ajouter et charger** (l'app sauvegarde la partie, puis les ajoute), **Charger sans eux** (la sauvegarde reste telle quelle) ou **Retour**. L'ajout demande que PZ Mod Studio soit ouvert.
