# Mapas

Mapas muestra el mundo del juego y todos los mods de mapa instalados en una sola imagen, para que veas qué partes de Knox Country reemplaza un mapa y dónde dos mods de mapa se pelean por el mismo terreno. También lista los mods de mapas que no agregan tierra propia y avisa de choques de casillas.

## Qué ves

- El mapa real del mundo del juego, con las celdas de cada mod de mapa dibujadas encima en su propio color.
- A la derecha, la lista de mods de mapa que agregan celdas.
- Arriba del mapa, un aviso por cada par de mods de mapa que comparten celdas o números de casillas.
- Abajo, **Otros mods de mapas**: puntos de aparición, listas de mapas y paquetes de casillas.

El mundo está dividido en celdas; una celda son 256 x 256 casillas.

## Moverte por el mapa

- Girá la **rueda** del mouse para acercar y alejar. El zoom se hace sobre el punto que está bajo el puntero.
- **Mantené apretado el botón izquierdo y arrastrá** para mover el mapa.
- **Doble clic** en el mapa, o el botón de su esquina superior derecha (**Ver el mapa entero**), para volver a verlo completo.
- Dejá el puntero sobre una celda para ver su número, si es parte del mundo del juego y qué mapas la cubren.

La imagen se arma con las imágenes de mapa que traen el juego y los mods. Mientras tanto ves **Armando el mapa…**. Un mod de mapa que no trae imagen igual muestra sus celdas como cuadrados de color.

## Qué significan los colores

- **El mundo del juego**: el mapa base, con una grilla suave encima.
- **El color de cada mod de mapa**: sus celdas. El cuadradito al lado de su nombre en la lista muestra el color. Las celdas de un mod sobre el mundo del juego reemplazan lo que había.
- **Rojo**: celdas que comparten dos mods de mapa. Ahí uno reemplaza al otro en tu partida, salvo que uno esté construido sobre el otro.

## La lista de mods de mapa

Cada fila muestra el nombre del mapa, el mod del que viene, cuántas celdas tiene, cuántas están sobre el mundo del juego (o **fuera del mundo del juego**), en qué mapas **se apoya**, y **apagado** si el mod está apagado.

- Destildá la casilla de una fila para ocultar ese mapa de la imagen. Esto solo cambia la vista, no tu lista de mods.
- Apretá el ícono de lista de una fila (**Verlo en la lista de mods**) para saltar a ese mod en la Lista de mods, resaltado.
- Marcá **Solo mods de mapa prendidos en mi lista** para ver solo los mapas que tenés activos.
- El botón de actualizar de arriba vuelve a leer los mapas. También se vuelven a leer cuando cambia tu lista de mods.

## Choques

- Un aviso **rojo** significa que dos mods de mapa comparten celdas y los dos están prendidos: en tu próxima partida uno reemplaza al otro ahí. Debajo, cada mod tiene un botón para verlo en la Lista de mods y un botón **Desactivar** que lo apaga (después de preguntarte).
- Un aviso **ámbar** significa que chocarían si los dos estuvieran prendidos.
- Un aviso de definición de casillas significa que dos mods usan el mismo número de definición de casillas. Si los dos están activos, sus casillas se pisan y aparecen paredes, pisos o muebles equivocados. Dejá activo solo uno.

## Otros mods de mapas

Estos mods son de mapas pero no agregan celdas: puntos de aparición, una lista de mapas para otro mapa, o las casillas con que se arman los mapas. Cada uno lleva la etiqueta **puntos de aparición / lista de mapas** o **casillas**. Necesitan que esté activo el mapa o el mod para el que son. Hacé clic en uno para verlo en la Lista de mods.

## Para tener en cuenta

- Dos mapas que comparten celdas no chocan cuando uno nombra al otro en su `map.info` (`lots=`); la lista lo muestra como **se apoya en**.
- Las imágenes del mapa se arman una vez y se guardan en `Zomboid/PZModStudio_Cache/maps/`, así la pantalla abre más rápido la próxima vez.
- Nada de esta pantalla cambia tu juego, salvo **Desactivar**.
