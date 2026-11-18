# Cartes

Cartes montre le monde du jeu et chaque mod de carte installé sur une seule image, pour que vous voyiez quelles parties de Knox Country une carte remplace et où deux mods de carte se disputent le même terrain. Il liste aussi les mods de carte qui n'ajoutent pas de terrain à eux, et avertit des conflits de tuiles.

## Ce que vous voyez

- La vraie carte du monde du jeu, avec les cellules de chaque mod de carte dessinées par-dessus dans leur propre couleur.
- À droite, la liste des mods de carte qui ajoutent des cellules.
- Au-dessus de la carte, un avertissement pour chaque paire de mods de carte qui partagent des cellules ou des numéros de tuiles.
- En dessous, **Autres mods de carte** : points d'apparition, listes de cartes et packs de tuiles.

Le monde est découpé en cellules ; une cellule fait 256 x 256 tuiles.

## Se déplacer sur la carte

- Cliquez d'abord sur la carte, puis tournez la **molette** de la souris pour zoomer et dézoomer. Le zoom se fait sur le point sous le pointeur. Tant que vous n'avez pas cliqué, la molette fait défiler la page (**Cliquez sur la carte pour zoomer avec la molette** vous le rappelle), et elle le refait dès que le pointeur quitte la carte.
- **Maintenez le bouton gauche et glissez** pour déplacer la carte.
- **Double-cliquez** sur la carte, ou appuyez sur le bouton dans son coin en haut à droite (**Voir toute la carte**), pour tout revoir.
- Laissez le pointeur sur une cellule pour voir son numéro, si elle fait partie du monde du jeu, et quelles cartes la couvrent (**Plusieurs cartes ici** quand deux la partagent).

L'image est assemblée à partir des images de carte que le jeu et les mods fournissent. Pendant ce temps, vous voyez **Assemblage de la carte…**. Un mod de carte qui ne fournit pas d'image montre quand même ses cellules en carrés de couleur.

## Ce que veulent dire les couleurs

- **Le monde du jeu** : la carte de base, avec une grille légère par-dessus.
- **La couleur de chaque mod de carte** : ses cellules. Le petit carré à côté de son nom dans la liste montre la couleur. Les cellules d'un mod de carte sur le monde du jeu remplacent ce qui s'y trouvait.
- **Rouge** : des cellules que deux mods de carte partagent. Là, l'un remplace l'autre dans votre partie, sauf si l'un est construit par-dessus l'autre.

## La liste des mods de carte

Chaque ligne montre le nom de la carte, le mod d'où elle vient, combien de cellules elle a, combien sont sur le monde du jeu (ou **hors du monde du jeu**), sur quelles cartes elle **s'appuie**, et **désactivé** si le mod est désactivé.

- Décochez la case d'une ligne pour masquer cette carte de l'image. Cela ne change que l'affichage, pas votre liste de mods.
- Appuyez sur l'icône de liste d'une ligne (**Voir dans la Liste des mods**) pour aller à ce mod dans la Liste des mods, mis en évidence.
- Cochez **Seulement les mods de carte activés dans ma liste** pour n'afficher que les cartes que vous avez activées. Cela masque aussi les avertissements et les autres mods de carte qui ne concernent que des mods désactivés. Le choix est retenu.
- Le bouton d'actualisation en haut relit les cartes. Elles sont aussi relues quand votre liste de mods change.

## Conflits

- Un avertissement **rouge** veut dire que deux mods de carte partagent des cellules et sont tous les deux activés : dans votre prochaine partie, l'un remplace l'autre à cet endroit. En dessous, chaque mod a un bouton pour le voir dans la Liste des mods et un bouton **Désactiver** qui le désactive (après confirmation).
- Ce qui ne serait en conflit que si vous activiez des mods désactivés aujourd'hui se replie en une ligne grise (**N conflits possibles si vous activez des mods désactivés aujourd'hui : rien à faire pour l'instant**). Ouvrez-la pour voir chacun d'eux.
- Un avertissement de définition de tuiles veut dire que deux mods utilisent le même numéro de définition de tuiles. Si les deux sont activés, leurs tuiles s'écrasent mutuellement et vous obtenez de mauvais murs, sols ou meubles. N'en gardez qu'un activé. En dessous, chaque mod a un bouton pour le voir dans la Liste des mods, et **Désactiver** quand plus d'un est activé.

## Autres mods de carte

Ces mods concernent les cartes mais n'ajoutent pas de cellules : points d'apparition, une liste de cartes pour une autre carte, ou les tuiles avec lesquelles les cartes sont construites. Chacun est marqué **points d'apparition / liste de cartes** ou **tuiles**. Ils ont besoin que la carte ou le mod auquel ils sont destinés soit activé. Cliquez sur l'un d'eux pour le voir dans la Liste des mods.

## Bon à savoir

- Deux cartes qui partagent des cellules ne sont pas en conflit quand l'une nomme l'autre dans son `map.info` (`lots=`) ; la liste l'affiche comme **s'appuie sur**.
- Les images de carte sont faites une fois et gardées dans `Zomboid/PZModStudio_Cache/maps/`, pour que l'écran s'ouvre plus vite la fois suivante.
- Rien sur cet écran ne change votre jeu, sauf **Désactiver**.
