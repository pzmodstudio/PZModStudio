# Salud de mods

Salud de mods revisa tus mods activos como los lee el juego y lista cada problema que encuentra con un botón para arreglarlo. Mira qué puede cargar tu build del juego, requisitos que faltan o están apagados, el orden de carga, mods incompatibles, archivos que cambian dos mods y mods que llenaron la consola la última vez que jugaste.

## Qué ves

- Arriba: tu **Build del juego** y cuántos mods hay activos.
- El botón de la **comunidad** (por ejemplo **comunidad: 40 reglas, 120 mods ↻**): lo que suman a la revisión los archivos compartidos de la comunidad. Hacé clic para descargarlos ahora. **Índice de la comunidad**, al lado, abre la página web del índice: los problemas conocidos de cada mod y sus arreglos.
- **Revisar de nuevo** vuelve a hacer la revisión. También se hace sola cuando cambia tu lista.
- Cinco pestañas, cada una con cuántas filas tiene:
  - **Problemas**: lo que rompe el juego o lo va a romper (requisitos que faltan o están apagados, orden de carga, mods que tu versión no carga, pares incompatibles). Acá está **Arreglar N problema(s) automáticamente**.
  - **Conflictos**: dos mods que traen el mismo archivo, ítem, receta o texto; el juego usa uno, y cuál es decisión tuya. Acá está **Fusionar N archivos automáticamente**.
  - **Para saber**: nada que arreglar (copias ignoradas, mods actualizados desde la última vez que jugaste, notas de la comunidad).
  - **Rendimiento**: tiempo de juego por mod (más abajo).
  - **Disco**: cuánto ocupa cada mod y cuáles se pueden sacar (más abajo).
- Cada fila muestra el mod, qué anda mal, un código corto (como `missing_dependency`) y sus botones de arreglo a la derecha. La pantalla recuerda la última pestaña; la etiqueta de un problema en la Lista de mods abre la pestaña que lo tiene.

Si no hay nada mal, ves **No se encontraron problemas en tus N mods activos.**

## Gravedades

- **Errores**: el juego no va a cargar el mod, o lo carga sin algo que necesita. Arreglá estos primero.
- **Avisos**: el juego carga, pero algo probablemente salga mal: orden equivocado, un archivo que solo usa un mod, una actualización sin descargar, un mod que llena la consola.
- **Información**: conviene saberlo, no hay nada roto: dos mods definen lo mismo, se está usando una copia compatible, mods que se actualizaron desde que jugaste.

## Los problemas que encuentra

- `missing_mod` (No instalado): un mod de tu lista no está en el disco. Si hay un mod instalado con un id casi igual, ofrece activar ese.
- `not_for_build`, `outside_version_range`, `invalid_mod_info`: el juego no va a cargar este mod en tu build, o su `mod.info` está roto.
- `missing_dependency`: el mod requiere otro que no está instalado.
- `dependency_not_enabled`: el mod requerido está instalado pero apagado.
- `dependency_after`, `load_after_order`, `load_before_order`: dos mods están en el orden equivocado.
- `dependency_unavailable`: un mod requerido no puede cargar él mismo.
- `incompatible`: el mod dice que no funciona con otro mod que tenés activo.
- `duplicate_entry`: el mismo mod está dos veces en la lista.
- `two_mod_managers`: otro administrador de mods que dibuja la pantalla Mods del juego está activo junto a Studio Link.
- `workshop_update_pending`: Steam tiene una versión más nueva que todavía no se descargó. Los servidores rechazan a jugadores con versiones de mods distintas.
- `file_conflict`: dos mods traen el mismo archivo y el juego solo usa el último.
- `script_definition_conflict` y `translation_conflict`: dos mods definen el mismo ítem, receta o nombre de forma distinta; gana el que carga último. Las filas que terminan en `_stale` significan que ya lo fusionaste, pero un mod cambió después; `_settled` significa que tu paquete ya lo resuelve.
- `floods_console` (en la Lista de mods se ve como **Llena la consola**): en tu última sesión este mod escribió más de 1000 líneas de errores en la consola. Eso hace más lento el juego y esconde los errores de otros mods. Miralos en el Centro de monitoreo, o apagá el mod.
- `user_rule_order`: no se cumple una de tus propias reglas de orden de carga.
- `community_rule_order`, `community_incompatible`, `community_known_issue`, `community_successor`: lo que dicen los archivos de la comunidad sobre tus mods.
- `package_built_for_other_list`: tu paquete del Studio se armó con una lista de mods distinta a la de este perfil.
- `compatible_copy`, `shadowed_copy`, `updated_since_last_play`: notas sobre qué copia de un mod se usa y qué cambió.

Las filas sobre la misma definición se pueden abrir con **Por archivo** para ver el archivo de cada mod. Hacé clic en una definición ahí para abrirla en el Fusionador.

## Los botones de arreglo

- **Activar X** o **Apagar X**: activa o apaga un mod. Apagar pregunta primero; el mod queda instalado.
- **Mover X antes de Y**: cambia el orden de carga. En una fila "a tu criterio" (dos mods definen lo mismo) dice **Usar la versión de Y**.
- **Quitar X de la lista**: saca una entrada de la lista.
- **Buscar "X" en el Workshop** y **Abrir página del Workshop**: abren el Workshop de Steam en tu navegador.
- **Crear copia compatible**: hace en `Zomboid/mods` una copia adaptada de un mod del Workshop para tu build. El resultado muestra qué cambió y qué puede fallar igual. Iniciá el juego desde el Studio para que use la copia. **Quitar la copia compatible** vuelve al original.
- **Guardar la versión que tenés**: guarda el mod tal como está hoy, antes de que llegue una actualización del Workshop. Podés restaurarlo desde la Lista de mods (clic derecho, Versiones guardadas).
- **Fusionar automáticamente** y **Abrir en el Fusionador**: resuelven un conflicto de archivo o de definición en tu paquete de parche, que carga último.
- **Encender el parche X del shim**: activa un parche del shim de Build 42 que necesita un problema conocido.

Una línea verde confirma cada cambio, con **Guardado en tu lista de mods.**

## Arreglar muchos a la vez

- **Arreglar N problema(s) automáticamente** activa los requisitos que faltan y sube mods en el orden donde hace falta. Nunca apaga nada y se saltea las filas "a tu criterio". Pregunta primero, y la Lista de mods puede deshacerlo.
- **Fusionar N archivos automáticamente** fusiona en tu paquete de parche todos los archivos en conflicto que no tienen cambios superpuestos. Los archivos donde los dos mods cambian las mismas líneas quedan listados para que decidas en el Fusionador.

## Reglas e índice de la comunidad

El Studio descarga una vez al día reglas compartidas desde su propio repositorio: reglas de orden de carga, pares de mods que no van juntos y un índice de problemas conocidos con su arreglo. Aparecen como filas `community_*`. Podés apagar la descarga diaria en Configuración; entonces se usa la última copia.

## Memoria del juego

En la pestaña **Rendimiento**: cuánta memoria puede usar el Java del juego (`-Xmx` en `ProjectZomboid64.json`), de la que tiene la PC, con una recomendación para tus mods activos y un aviso si en una sesión reciente se quedó sin memoria. Elegí un valor (de 2 GB hasta la mitad de la memoria de la PC) y **Aplicar** con el juego cerrado; se aplica en el próximo inicio, y una copia del archivo de antes del cambio queda en `PZModStudio_Backups`.

Cada perfil guarda su propio valor: al activar un perfil, el juego recibe su memoria. Un perfil sin valor propio usa el último elegido. Si una actualización del juego vuelve a poner su valor, el Studio pone el tuyo la próxima vez que abre el juego.

## Tiempo de juego por mod

En la pestaña **Rendimiento**: qué mods frenan el juego mientras jugás. **Medir** pone un mod chico del Studio (PZ Mod Studio Profiler) primero en tu lista de mods y en la partida que jugaste por última vez; abrí el juego (reinicialo si estaba abierto) y jugá unos minutos como siempre. Cada handler de eventos de cada mod lo mide el propio juego y se suma por mod: la parte del tiempo real que se llevó, milisegundos por segundo y su llamada más larga (una larga es un congelamiento que se ve). Abrí una fila para ver los handlers que más cuestan, con su archivo y línea. **Dejar de medir** saca el medidor de la lista y de la partida; los últimos números quedan.

También cuentan las ventanas, barras y acciones con tiempo de un mod (como Clase:render, Clase:update); cada función cuenta solo su propio tiempo. El medidor le suma un poco de tiempo propio a cada handler, así que apagalo cuando termines.

## Espacio en disco

En la pestaña **Disco**: cuánto ocupa cada mod instalado, contado por lo que se va junto (un ítem del Workshop con todos sus mods, una carpeta de `Zomboid/mods`). Medirlo tarda un par de segundos.

- **Nada lo usa** (ámbar): ningún perfil lo tiene, ni prendido ni apagado; ninguna partida lo nombra en su lista; ningún servidor guardado lo cargó; no está en la lista activa del juego.
- **Apagado en todos lados** (celeste): algún perfil lo guarda, pero apagado, y ninguna partida ni servidor lo usa.
- **Sobrante**: una carpeta del Workshop de un ítem al que ya no estás suscrito; Steam a veces las deja.
- **Tu subida**: un mod tuyo en `Zomboid/Workshop`. Se muestra, nunca se ofrece borrarlo.

Marcá los que quieras sacar. Un ítem del Workshop se va con **Desuscribirse** (Steam tiene que estar abierto, y borra los archivos solo). Una carpeta local o un sobrante va con **Mandar a la Papelera**: se puede recuperar desde la Papelera de reciclaje. Antes de cada una se pide confirmación con los nombres. Con el juego abierto no se manda nada a la Papelera.

Abajo, **Carpetas del Studio y del juego**: cuánto ocupan las copias de partidas, las versiones guardadas de mods, la caché y los registros, con un botón para abrir cada carpeta. Desde ahí no se borra nada.

## Para tener en cuenta

- Los cambios de acá se guardan en la lista de mods del perfil activo.
- Los archivos fusionados y los parches del shim se aplican la próxima vez que arranca el juego.
- "Llena la consola" se mide con el `console.txt` de la última sesión, así que se actualiza después de jugar.
- Hacer clic en la etiqueta de un problema en la Lista de mods abre esta pantalla en esa fila.
