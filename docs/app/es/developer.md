# Desarrollador

La pantalla Desarrollador es para quienes hacen mods. Podés crear un mod nuevo, revisar todos los archivos de un mod antes de que lo haga el juego, iniciar el juego solo con ese mod activo, recargar su Lua mientras el juego corre, revisarlo para multijugador y contra la build instalada, escribir borradores de traducción, publicarlo en el Steam Workshop y abrir los editores de mapas.

## Qué ves

La pantalla solo aparece en la barra lateral cuando **Funciones para desarrolladores** está encendido en **Configuración**, y, como las demás pantallas, una vez que hay un perfil activo. Arriba, **El mod en el que estás trabajando** tiene el mod sobre el que trabajan todas las herramientas, con **Abrir su carpeta** e **Iniciar el juego solo con este mod**. Debajo hay una pestaña por herramienta: **Revisar archivos**, **Recarga en vivo**, **Multijugador**, **Contra la build**, **Traducciones**, **Workshop**, **Mapas** y **Mod nuevo**. Una pestaña muestra una pequeña marca cuando tiene algo que decir (problemas encontrados, vigilando, el veredicto multijugador). El Studio recuerda la última pestaña que abriste. Todas las pestañas menos **Workshop**, **Mapas** y **Mod nuevo** necesitan primero un mod elegido.

## Crear un mod nuevo

1. Abrí la pestaña **Mod nuevo**.
2. Completá **Nombre**, **Id** (la carpeta y la entrada de la lista: letras, dígitos, `_` y `-`; sigue al nombre hasta que lo cambies), **Autor** y **Descripción**.
3. Marcá **También Build 41** si el mod también tiene que andar en la Build 41.
4. Apretá **Crearlo**.

El Studio crea el mod en `Zomboid/mods` tal como lo lee la Build 42: `42/mod.info`, las carpetas `42/media/lua/client`, `server` y `shared`, `42/media/scripts`, `common/media/lua`, un póster para reemplazar y un archivo Lua que imprime el nombre del mod cuando el juego lo carga. Rechaza un id que no sea un nombre simple o una carpeta que ya existe. **Abrir la carpeta** la abre en el Explorador de Windows, y el mod nuevo queda elegido como el mod en el que trabajás. Activalo en la Lista de mods cuando esté listo.

## Elegir el mod en el que trabajás

En **El mod en el que estás trabajando**, elegí un mod de la lista. Los mods están agrupados según de dónde vienen, primero los que trabajás: **Tus subidas al Workshop (Zomboid/Workshop)**, **Carpetas en Zomboid/mods**, **Instalados desde Nexus, un archivo o un enlace**, las copias que el Studio adaptó a tu build, **Mods del propio Studio**, y al final **Suscripciones del Workshop**, para mirarlos. Debajo de la lista ves el id del mod, su versión, su número del Workshop y su carpeta. **Abrir su carpeta** abre la carpeta del mod. El Studio recuerda tu elección.

## Revisar todos los archivos

En la pestaña **Revisar archivos**, apretá **Revisar todos los archivos**. Cada archivo `.lua` pasa por el mismo parser de Lua que usa el Fusionador de mods, y cada `media/scripts/*.txt` por una revisión de llaves (el juego pierde en silencio un archivo de script entero en su primer bloque desbalanceado). El resultado aparece justo debajo del botón, con cuántos archivos Lua y de script se revisaron. Dice **Todos los archivos se leen bien**, o lista los problemas por archivo con línea y columna. Hacé clic en un problema para abrir el archivo en esa línea en VS Code si está instalado, o si no en el programa que Windows usa para ese archivo.

Esto solo revisa la sintaxis. Qué hace el juego con los archivos lo muestra el Centro de monitoreo.

## Iniciar el juego solo con este mod

**Iniciar el juego solo con este mod**, arriba de la pantalla, arranca el juego en Debug (en ventana) solo con este mod, lo que requiere y el puente del Studio activos. Tu lista de mods queda guardada aparte, y un aviso lo dice. Vuelve cuando el juego se cierra, o en el momento con **Devolver mi lista ahora**. La lista también queda en **Listas anteriores** de la Lista de mods. El botón está apagado mientras dura ese inicio.

## Recargar en el juego mientras corre

Esto te deja cambiar un archivo Lua y verlo en el juego sin reiniciar.

1. Iniciá el juego con el mod puente (`Z_PZModStudio_Bridge`) activo.
2. Abrí la pestaña **Recarga en vivo** y encendé el interruptor de la derecha. Dice **Vigilando (nombre del mod)**.
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
- **Archivos que lo cargan con `require`**: los otros archivos del mod que cargan el archivo cambiado con `require` se recargan justo después, así toman su código nuevo.

El log dice qué se conservó, por ejemplo "1 handler(s), 1 table(s) kept".

## Revisión multijugador

En la pestaña **Multijugador**, **Revisar este mod para multijugador** busca en el Lua cliente y compartido del mod llamadas que cambian directamente el mundo, un vehículo, un animal o el mapa, donde el código del propio juego le pide al servidor. Cada hallazgo muestra el archivo, la línea y el handler del juego que lo hace en el servidor. Una llamada dentro de una rama `if isClient()` cuenta como atendida. Hacé clic en un hallazgo para abrirlo en tu editor. Revisa solo esas llamadas: un mod sin nada listado igual puede fallar en multijugador de maneras que esto no ve.

## Contra la build instalada

En la pestaña **Contra la build**, **Revisar qué nombres tiene esta build** busca cada nombre que llama el Lua del mod en el juego que tenés instalado: sus clases y métodos Java, sus funciones globales, su propio Lua y los eventos que dispara, más lo que definen este mod y tus otros mods activos. Lo que queda es una llamada que fallaría en esa línea: una función que Build 42 sacó, un método renombrado, un evento que ya no se dispara, un error de tipeo. Los hallazgos se agrupan en funciones globales, métodos y eventos, cada uno con su archivo y línea, y un "¿quisiste decir…?" cuando hay un nombre conocido parecido. Hacé clic en uno para abrirlo en tu editor. Una llamada que el archivo también nombra sin llamar (`if foo then`) cuenta como protegida y no se lista.

La primera revisión lee los archivos del juego y tarda unos segundos; después queda guardada por build. Va solo por nombre: un nombre armado mientras corre el juego no se ve, y un mod que trae Java puede definir nombres que el Lua no muestra (el resultado lo dice). Abrí la línea para juzgar.

## Borradores de traducción

La pestaña **Traducciones** muestra el idioma en que está escrito el mod, cuántos textos tiene, y sus idiomas en grupos: los que tienen textos faltantes, los completos y las carpetas del mod que no son idiomas del juego. Marcá los idiomas que quieras y apretá **Escribir borradores para…**: el Studio escribe un borrador de cada texto que le falta a cada idioma, traducido por el mismo servicio que traduce las descripciones de mods (**Traducción** en **Configuración**), en la carpeta `Translate/<LANG>/` del propio mod. Lo que ya está se deja como está. **Abrir la carpeta Translate** la abre. Un borrador es un punto de partida, no una traducción: leelo antes de publicar.

## Publicar en el Workshop

La pestaña **Workshop** sube un mod al Steam Workshop a través del cliente de Steam abierto en esta PC. Usa los mismos archivos que el subidor del propio juego, en `Zomboid/Workshop/<ítem>/`: `workshop.txt`, `preview.png` y `Contents/mods/<mod>/`.

1. Elegí un ítem de la lista. Para un mod tuyo en `Zomboid/mods` que todavía no tiene ítem, apretá **Preparar (mod) para el Workshop**: crea la carpeta del ítem con una copia del mod, un `workshop.txt` a partir de su `mod.info` y su póster como vista previa.
2. Completá **Título**, **Visibilidad** (**Público**, **Solo amigos**, **Privado** u **Oculto (solo con el enlace)**), **Tags (separados por ;)** y **Descripción** (Steam acepta hasta 8000 caracteres), y apretá **Guardar workshop.txt**.
3. **Ver los archivos** muestra lo que se mandaría y qué cambió desde la última subida desde el Studio. Un `.workshopignore` en la carpeta del ítem (un patrón por línea, como `*.psd` o `.git/`) deja archivos afuera.
4. Escribí una nota de cambios si querés, y apretá **Publicar en el Workshop** (o **Subir la actualización** para un ítem ya publicado). Primero se te pregunta. Steam tiene que estar abierto y con la cuenta dueña del ítem.

Se manda lo que dice `workshop.txt`: lo que edites en la página de Steam se reemplaza en la siguiente subida, igual que hace el juego. Si Steam deja un ítem nuevo oculto hasta que aceptes su acuerdo legal del Workshop, el resultado lo dice, con un enlace. **Ver la página en el Workshop** abre el ítem en tu navegador. El Studio no sube mods ajenos: cuando el ítem trae un mod al que estás suscripto, lo dice y el botón queda apagado. El botón del tacho al lado de la lista saca la carpeta del ítem de `Zomboid/Workshop` (a la Papelera de reciclaje); en Steam no cambia nada.

## Herramientas de mapas

En la pestaña **Mapas**, TileZed dibuja los tiles de cada celda (edificios, habitaciones, muebles, suelo); WorldEd dispone las celdas en la grilla del mundo y escribe los archivos de mapa que lee el juego. Vienen de The Indie Stone en un zip que descomprimís en cualquier lado. El Studio los busca en los lugares habituales (tu carpeta de usuario, Documentos, Escritorio, Descargas, la unidad C:) y en la carpeta que escribas en el cuadro de cada uno. Cuando encuentra uno, **Iniciar TileZed** o **Iniciar WorldEd** lo arranca desde su propia carpeta. Las celdas de un mod de mapa van en `media/maps/<nombre>` del mod.

## Bueno saber

- Recargar necesita el juego abierto con el mod puente activo; las otras herramientas no necesitan el juego.
- Recargar en el juego anda sin reiniciar; una constante que cambiaste en una tabla conserva el valor viejo hasta un Reset Lua completo.
- Todas las herramientas menos **Workshop**, **Mapas** y **Mod nuevo** necesitan un mod elegido cuya carpeta conozca el Studio.
