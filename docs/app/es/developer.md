# Desarrollador

La pantalla Desarrollador es para quienes hacen mods. Podés crear un mod nuevo, revisar todos los archivos de un mod antes de que lo haga el juego, iniciar el juego solo con ese mod activo, recargar su Lua mientras el juego corre, revisarlo para multijugador y abrir los editores de mapas.

## Qué ves

La pantalla solo aparece en la barra lateral cuando **Funciones para desarrolladores** está encendido en **Configuración**, y, como las demás pantallas, una vez que hay un perfil activo. De arriba a abajo tiene estas tarjetas: **Un mod nuevo**, **El mod en el que estás trabajando**, **Recargar en el juego mientras corre**, **Revisión multijugador** y **Herramientas de mapas**. Los resultados de una revisión de archivos aparecen abajo de todo.

## Crear un mod nuevo

1. Apretá **Mod nuevo…**.
2. Completá **Nombre**, **Id** (la carpeta y la entrada de la lista: letras, dígitos, `_` y `-`; sigue al nombre hasta que lo cambies), **Autor** y **Descripción**.
3. Marcá **También Build 41** si el mod también tiene que andar en la Build 41.
4. Apretá **Crearlo**.

El Studio crea el mod en `Zomboid/mods` tal como lo lee la Build 42: `42/mod.info`, las carpetas `42/media/lua/client`, `server` y `shared`, `42/media/scripts`, `common/media/lua`, un póster para reemplazar y un archivo Lua que imprime el nombre del mod cuando el juego lo carga. Rechaza un id que no sea un nombre simple o una carpeta que ya existe. **Abrir la carpeta** la abre en el Explorador de Windows, y el mod nuevo queda elegido como el mod en el que trabajás. Activalo en la Lista de mods cuando esté listo.

## Elegir el mod en el que trabajás

En **El mod en el que estás trabajando**, elegí un mod de la lista. Tus propios mods (**En Zomboid/mods (tuyos)**) van primero; los del Workshop van después, para mirarlos. **Abrir su carpeta** abre la carpeta del mod. El Studio recuerda tu elección.

## Revisar todos los archivos

Apretá **Revisar todos los archivos**. Cada archivo `.lua` pasa por el mismo parser de Lua que usa el Fusionador de mods, y cada `media/scripts/*.txt` por una revisión de llaves (el juego pierde en silencio un archivo de script entero en su primer bloque desbalanceado). El resultado dice **Todos los archivos se leen bien**, o lista los problemas por archivo con línea y columna. Hacé clic en un problema para abrir el archivo en esa línea en VS Code si está instalado, o si no en el programa que Windows usa para ese archivo.

Esto solo revisa la sintaxis. Qué hace el juego con los archivos lo muestra el Centro de monitoreo.

## Iniciar el juego solo con este mod

**Iniciar el juego solo con este mod** arranca el juego en Debug (en ventana) solo con este mod, lo que requiere y el puente del Studio activos. Tu lista de mods queda guardada aparte, y un aviso lo dice. Vuelve cuando el juego se cierra, o en el momento con **Devolver mi lista ahora**. La lista también queda en **Listas anteriores** de la Lista de mods. El botón está apagado mientras dura ese inicio.

## Recargar en el juego mientras corre

Esto te deja cambiar un archivo Lua y verlo en el juego sin reiniciar.

1. Iniciá el juego con el mod puente (`Z_PZModStudio_Bridge`) activo.
2. Encendé el interruptor a la derecha de **Recargar en el juego mientras corre**. Dice **Vigilando (nombre del mod)**.
3. Guardá un archivo Lua del mod. El Studio lo ve en cuanto se guarda, lo revisa con el parser y lo manda al juego, que lo recarga en el menú principal, en la partida o en pausa.

**Recargar todos los Lua de este mod ahora** manda todos los archivos Lua del mod de una vez.

El registro debajo de los botones muestra cada paso:

- **cambió:** se mandó un archivo al juego.
- **no se mandó, no parsea:** el archivo tiene un error de sintaxis, con su línea, columna y mensaje. No se manda, así el juego nunca corre código roto.
- La respuesta del juego para cada archivo, en verde cuando anduvo y en rojo cuando falló.
- Los tiempos: detectado, enviado, aplicado en el juego y de vuelta en el Studio, en milisegundos.
- **El juego no está corriendo** o **Sin respuesta del juego en 6 s** cuando no había dónde recargar (fijate que el mod puente esté activo).

Cuando se recarga un archivo, su código de nivel superior corre de nuevo, y el puente cuida lo que eso rompería:

- **Manejadores de eventos**: cada uno que el archivo agrega corre una sola vez, uno que agregás por primera vez empieza a funcionar, y uno que sacás del archivo deja de correr.
- **Tablas**: cuando el archivo vuelve a crear una tabla global (`MiMod = {}`, `MiPanel = ISPanel:derive(...)`), se queda la tabla vieja con las funciones nuevas, así que las ventanas abiertas y las referencias guardadas corren el código nuevo con su estado. Un valor que la tabla vieja ya tenía conserva el valor viejo (el puente no puede distinguir tu estado de una constante que cambiaste): para una constante cambiada, usá un Reset Lua completo.
- **Tu propia preparación**: `if PZStudio then PZStudio.onReload(function(file) ... end) end` corre después de cada recarga de ese archivo, para lo que hacía `OnGameStart` y no se vuelve a hacer.

El log dice qué se conservó, por ejemplo "1 handler(s), 1 table(s) kept".

## Revisión multijugador

**Revisar este mod para multijugador** busca en el Lua cliente y compartido del mod llamadas que cambian directamente el mundo, un vehículo, un animal o el mapa, donde el código del propio juego le pide al servidor. Cada hallazgo muestra el archivo, la línea y el handler del juego que lo hace en el servidor. Una llamada dentro de una rama `if isClient()` cuenta como atendida. Hacé clic en un hallazgo para abrirlo en tu editor. Revisa solo esas llamadas: un mod sin nada listado igual puede fallar en multijugador de maneras que esto no ve.

## Herramientas de mapas

TileZed dibuja los tiles de cada celda (edificios, habitaciones, muebles, suelo); WorldEd dispone las celdas en la grilla del mundo y escribe los archivos de mapa que lee el juego. Vienen de The Indie Stone en un zip que descomprimís en cualquier lado. El Studio los busca en los lugares habituales (tu carpeta de usuario, Documentos, Escritorio, Descargas, la unidad C:) y en la carpeta que escribas en el cuadro de cada uno. Cuando encuentra uno, **Iniciar TileZed** o **Iniciar WorldEd** lo arranca desde su propia carpeta. Las celdas de un mod de mapa van en `media/maps/<nombre>` del mod.

## Bueno saber

- Recargar necesita el juego abierto con el mod puente activo; las otras herramientas no necesitan el juego.
- Recargar en el juego anda sin reiniciar; una constante que cambiaste en una tabla conserva el valor viejo hasta un Reset Lua completo, y una tabla `local` que otros archivos obtuvieron con `require` se queda con las funciones viejas.
- La revisión, la recarga y la revisión multijugador necesitan un mod elegido cuya carpeta conozca el Studio.
