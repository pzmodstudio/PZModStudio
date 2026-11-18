# Diagnóstico

Diagnóstico junta lo que la app sabe de tus mods, del juego y del servidor. Tiene cuatro botones arriba: **Resumen**, **Mods**, **Juego** y **Servidor**. Al entrar siempre abre en Resumen; cada parte recuerda la última pestaña que miraste.

## Resumen y el Doctor

- Tres tarjetas, una por parte, con cómo está en una línea. **Mods**: si hay algo que rompe el juego, algo que conviene arreglar, o todo en orden. **Juego**: abierto o cerrado, cuánta memoria usa, el último arranque, el límite de memoria y si alguna sesión se quedó sin memoria. **Servidor**: si hay uno corriendo en esta PC y cómo está. Clic en una tarjeta para entrar a esa parte.
- Abajo, **el Doctor**: te pregunta qué te pasa (**El juego se cierra o no arranca**, **Un mod no anda**, **Va lento**, **Otra cosa**) y te lleva paso a paso con lo que la app ya sabe: lo que encontró la revisión de mods, los errores del Centro de monitoreo y las veces que el juego se quedó sin memoria. Si un paso te manda a otro lado, al volver sigue en el mismo paso. **Empezar de nuevo** vuelve a la pregunta.
- El botón **Doctor** de arriba de todo abre esta portada desde cualquier pantalla.

## Mods

Revisa tus mods activos como los lee el juego y lista cada problema que encuentra con un botón para arreglarlo. Mira qué puede cargar tu build del juego, requisitos que faltan o están apagados, el orden de carga, mods incompatibles, archivos que cambian dos mods y mods que llenaron la consola la última vez que jugaste.

- Arriba: tu **Build del juego** y cuántos mods hay activos.
- **notas del Studio** (por ejemplo **notas del Studio: 22 mods**): las reglas de orden, los pares incompatibles y los problemas conocidos que la app trae adentro. Si marcaste avisos con **Ya no pasa**, un botón al lado dice cuántos y los vuelve a mostrar todos.
- **Revisar de nuevo** vuelve a hacer la revisión. También se hace sola cuando cambia tu lista.
- Cinco pestañas; las tres primeras muestran cuántas filas piden algo:
  - **Qué hacer ahora**: lo que rompe el juego o lo va a romper (requisitos que faltan o están apagados, orden de carga, mods que tu versión no carga, pares incompatibles). Acá está **Arreglar N problema(s) automáticamente**. Plegado abajo: lo que ya arregló el Studio y lo que es solo para saber.
  - **Conflictos**: dos mods que traen el mismo archivo, ítem, receta o texto; el juego usa uno, y cuál es decisión tuya. Acá está **Fusionar N archivos automáticamente**.
  - **Seguridad**: lo que un mod podría hacer más allá de modificar el juego (más abajo).
  - **Auditoría**: tu lista mod por mod, con un veredicto cada uno: **Roto**, **Elegir**, **Sobra**, **Revisar** o **Dejar**. Tocá un mod para ver por qué, e **Ir** para ir al aviso.
  - **Tiempo por mod**: qué mods frenan el juego mientras jugás (más abajo).
- Cada fila muestra el mod, qué anda mal, un código corto (como `missing_dependency`) y sus botones de arreglo a la derecha. **Copiar para un asistente** copia la fila con una pregunta, para pegarla en cualquier asistente de chat; desde el Studio no se manda nada. La etiqueta de un problema en la Lista de mods abre la pestaña que lo tiene.

Si no hay nada mal, ves **No se encontraron problemas en tus N mods activos.**

### Qué tenés que hacer con cada fila

Cada fila dice una de estas cuatro cosas, las mismas que en la Lista de mods:

- **Hay que hacer algo**: el juego se rompe o no carga el mod hasta que hagas algo. Empezá por acá.
- **Te conviene**: funciona, pero conviene arreglarlo (orden equivocado, una actualización sin descargar, un mod que llena la consola). El botón dice qué cambia.
- **Para saber**: nada que hacer, es para que sepas (dos mods definen lo mismo, mods que se actualizaron desde que jugaste).
- **Ya lo arregló el Studio**: no toques nada, así funciona (una copia compatible, un archivo en tu paquete). Cada uno se puede deshacer si lo necesitás.

Cada sección se puede plegar haciendo clic en su título.

### Los problemas que encuentra

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
- `function_overridden` (**Anula a otro mod**): un mod reemplaza una función del juego, o de otro mod, después de que otro mod la cambió, así que ese cambio nunca corre. Cuando muchas funciones de un mismo mod quedan anuladas por otro (dos versiones del mismo mod prendidas a la vez), sale una sola fila. Si Studio Link está activo, se comprueba en el juego abierto: la fila dice **Comprobado en el juego**, o sale `function_seen_other` (**Otra versión en el juego**) cuando el juego tiene la versión de otro mod.
- `java_link_broken` (**Java viejo para esta build**): el Java del mod usa clases, métodos o campos que el juego instalado ya no tiene (una actualización los cambió). Ese código falla cuando corre. Si el Monitor lo muestra fallando, buscá una actualización del mod o apagalo; si anda bien, el mod se fija antes.
- `tiledef_clash`: dos mods guardan sus tiles bajo el mismo número de `tiledef` con hojas distintas, así que algunos tiles se ven como los del otro mod o faltan. Dejá prendido uno solo, o pedile a un autor que cambie el número. Los avisos de `file_conflict` dicen además si el log de la última partida (con esta misma lista) confirma qué copia usó el juego.
- `game_folder_changed` (**Cambió la carpeta del juego**): mientras el juego estuvo abierto cambiaron archivos de su propia carpeta, donde el juego nunca escribe. La app compara la carpeta del juego, la de Zomboid (sin partidas ni logs) y la de ZombieBuddy al abrirse y al cerrarse el juego, solo mientras la app está abierta.
- `script_definition_conflict` y `translation_conflict`: dos mods definen el mismo ítem, receta o nombre de forma distinta; gana el que carga último. Las filas que terminan en `_stale` significan que ya lo fusionaste, pero un mod cambió después; `_settled` significa que tu paquete ya lo resuelve.
- `floods_console` (en la Lista de mods se ve como **Llena la consola**): en tu última sesión este mod escribió más de 1000 líneas de errores en la consola. Eso hace más lento el juego y esconde los errores de otros mods. Miralos en el Centro de monitoreo, o apagá el mod.
- `user_rule_order`: no se cumple una de tus propias reglas de orden de carga.
- `community_rule_order`, `community_incompatible`, `community_known_issue`, `community_successor`: lo que dicen las reglas y el índice del Studio sobre tus mods. Un problema conocido tiene **Ya no pasa**: deja de mostrarse en esta PC.
- `learned_issue`: un error que un mod dio en 3 o más de tus partidas con la misma versión (ver "Lo que aprende esta PC").
- `package_built_for_other_list`: tu paquete del Studio se armó con una lista de mods distinta a la de este perfil.
- `package_file_stale`: tu paquete lleva una copia vieja de un archivo de un mod, y el mod cambió ese archivo después. El juego lee la copia vieja.
- `package_file_differs`, `package_file_orphan`: tu paquete lleva un archivo que trae un solo mod de la lista, o que no trae ningún mod de la lista. Sacalo si ya no lo necesitás.
- `security_executable`, `security_script`, `security_java`, `security_lua`: lo que un mod podría hacer más allá de modificar el juego, en la pestaña **Seguridad** (más abajo).
- `compatible_copy`, `shadowed_copy`, `updated_since_last_play`: notas sobre qué copia de un mod se usa y qué cambió.

Las filas sobre la misma definición se pueden abrir con **Por archivo** para ver el archivo de cada mod. Hacé clic en una definición ahí para abrirla en el Fusionador.

### Los botones de arreglo

- **Activar X** o **Apagar X**: activa o apaga un mod. Apagar pregunta primero; el mod queda instalado.
- **Mover X antes de Y**: cambia el orden de carga. En una fila "a tu criterio" (dos mods definen lo mismo) dice **Usar la versión de Y**.
- **Quitar X de la lista**: saca una entrada de la lista.
- **Buscar "X" en el Workshop** y **Abrir página del Workshop**: abren el Workshop de Steam en tu navegador.
- **Crear copia compatible**: hace en `Zomboid/mods` una copia adaptada de un mod del Workshop para tu build. El resultado muestra qué cambió y qué puede fallar igual. Iniciá el juego desde el Studio para que use la copia. **Volver al original** quita la copia.
- **Guardar la versión que tenés**: guarda el mod tal como está hoy, antes de que llegue una actualización del Workshop. Podés restaurarlo desde la Lista de mods (clic derecho, Versiones guardadas).
- **Fusionar automáticamente** y **Abrir en el Fusionador**: resuelven un conflicto de archivo o de definición en tu paquete de parche, que carga último.
- **Quitar del paquete**: saca un archivo de tu paquete de parche, después de preguntar; el juego vuelve a leer el archivo del propio mod en el próximo inicio.
- **Encender el parche X del shim**: activa un parche del shim de Build 42 que necesita un problema conocido.

Una línea verde confirma cada cambio, con **Guardado en tu lista de mods.**

### Arreglar muchos a la vez

- **Arreglar N problema(s) automáticamente** activa los requisitos que faltan y sube mods en el orden donde hace falta. Nunca apaga nada y se saltea las filas "a tu criterio". Pregunta primero, y la Lista de mods puede deshacerlo.
- **Fusionar N archivos automáticamente** fusiona en tu paquete de parche todos los archivos en conflicto que no tienen cambios superpuestos. Los archivos donde los dos mods cambian las mismas líneas quedan listados para que decidas en el Fusionador.

### Reglas e índice del Studio

El Studio trae reglas de orden de carga, pares de mods que no van juntos y un índice de problemas conocidos con su arreglo. Vienen con cada versión de la app; no se descarga nada y no se envía nada. Aparecen como filas `community_*`. Un problema conocido se deja de mostrar solo cuando los archivos del mod muestran que ya no está (una actualización arregló el archivo del que hablaba), o cuando el mod cambió después de que se vio y su línea del log ya no aparece en tu sesión. **Ya no pasa** lo saca a mano.

### Lo que aprende esta PC

Además del índice que trae, cada app aprende de su propia PC. En segundo plano lee los logs de las partidas y de los servidores que el juego guarda (`Zomboid/Logs`) y anota qué mods estaban activos, en qué versión, y qué errores les atribuyó el juego (los mismos que muestra el Monitor). Lo guarda en `Zomboid/PZModStudio_Learned.json` y no envía nada.

Cuando un mod da el mismo error en 3 partidas o más, con la misma versión y en la build instalada, aparece en Diagnóstico como **Aprendido en esta PC** (`learned_issue`), con la cuenta como prueba: "en 8 de 14 partidas tuyas". Si el índice de la app ya habla de ese problema, se muestra solo el del índice. Un parche de Java que el log atribuye a varios mods a la vez no se aprende, porque no se sabe de cuál es. Si el mod se actualiza, la cuenta empieza de cero; si en sus últimas 5 partidas no volvió a salir (porque se arregló o porque un arreglo funcionó), deja de mostrarse. **Ya no pasa** lo saca a mano. La portada de Diagnóstico dice de cuántas partidas aprendió.


### El Doctor y el cerebro

La portada de Diagnóstico tiene **Preguntale al Doctor**, un chat que responde sin IA: reconoce lo que preguntás por sus palabras y arma la respuesta con lo que aprendió esta PC y con las herramientas de la app (las mismas del servidor MCP). Cada respuesta dice de qué herramienta salió. Le podés preguntar qué sabe, por qué se cerró el juego, qué cambió cuando empezó un error, qué mods fallan, si la carga está más lenta, si funcionaron los arreglos, qué hizo la última actualización del juego, o nombrar un mod para ver su historial.

Entiende errores de tipeo y otras formas de decirlo ("se me crasheó", "se tildó", "qe canbio"), fechas ("¿qué cambió desde ayer?", "esta semana", "desde el 12/11"), "¿cuál tarda más en cargar?" o "¿qué mod da más errores?", y sigue la charla: después de hablar de un mod podés decir "¿y desde cuándo?", "apagalo", "anotalo" o "mostrámelo". Si duda, pregunta "¿cuál de estos?" con botones; si no entiende y le mostrás con un botón lo que querías, se acuerda de esa frase en esta PC. Cada respuesta empieza por la conclusión, sigue con un consejo y su prueba (por ejemplo, que casi todas las caídas fueron por falta de memoria de la PC con juego y servidor abiertos), deja el detalle plegado y trae botones para ir a la Lista de mods o a la parte de Diagnóstico que corresponde.

Además:

- **¿Qué mod lo causa?** Compara las partidas con y sin cada mod: "con X salieron mal 3 de 7; sin él, 24 de 196", y dice qué tan seguro está (bastante seguro, probable o puede ser casualidad). Un mod que está en casi todas las partidas no se puede comparar y no lo nombra.
- **Buscarlo por mitades** desde el chat: apaga la mitad de tus mods (lo que necesitan los otros queda), te deja jugar la prueba y le contestás "sigue pasando" o "ya no pasa" hasta que queda uno. Tu lista se guarda y vuelve al terminar.
- **El parte**: al abrir el chat después de jugar, cuenta cómo fue la última partida (cuánto jugaste, cómo terminó, errores nuevos, arreglos que funcionaron). "¿Qué pasó esta semana?" hace lo mismo para un período.
- **Preguntas sobre la app** ("¿cómo hago un perfil?", "¿qué es una copia compatible?"): responde con el párrafo de esta guía y un botón a esa pantalla.
- **Grupos, perfiles y el Workshop**: "VP Base" cuenta cómo anda ese grupo, "perfil Knox Lives" si sus últimas partidas salieron bien; "¿hay versión nueva de X?" y "¿qué dicen en los comentarios de X?" leen su página.
- **Varias preguntas juntas** ("¿qué cambió y qué mods fallan?"), **apodos** que aprende cuando elegís entre varios mods, **"deshacé eso"** para su último cambio, y **memoria**: la próxima vez te recuerda de qué mod hablaron y cómo está ahora.

**Con el juego (Studio Link):**

- En el menú principal del juego, la tarjeta de PZ Mod Studio (arriba a la derecha) muestra lo primero que diría el Doctor: si las últimas partidas con esta lista salieron mal, si la última se cayó, qué errores se repiten. La escribe la app cada vez que revisa, así que la tarjeta dice lo último que sabía aunque la app esté cerrada.
- El Link anota en un diario qué partida cargaste y si volviste al menú (en vez de cerrar la ventana). Así cada partida dice "partida X · volvió al menú", también las que jugaste con la app cerrada.
- En el menú de pausa (Esc) hay **Marcar este momento**: si pasa algo raro, tocalo. Después preguntale al Doctor "¿qué pasó cuando marqué?" y te dice qué escribió el log del juego en ese minuto (de un minuto y medio antes a medio minuto después).

También puede cambiar cosas, siempre con tu **Hacerlo**: crear, renombrar o borrar grupos del perfil activo y meter o sacar mods ("agrupá A, B en Mapas", "sacá A del grupo Mapas", "renombrá el grupo Mapas a Mundo", "borrá el grupo Mapas"), sugerir grupos para tus mods activos sin grupo (los que comparten nombre o lo que requieren), y escribir o borrar **notas del Studio** ("nota en A: texto", "borrá la nota del Studio de A", "anotá lo aprendido"). Las notas del Studio se guardan aparte de las tuyas y se ven en violeta con su propio botón para quitarlas; tus notas nunca las escribe, cambia ni borra.

Lo que el cerebro saca de las partidas:

- **Qué cambió**: cuando un error se repite, cuándo empezó y qué era distinto respecto de la última partida que anduvo sin él (mods agregados, sacados o actualizados, su lugar en el orden, la build del juego). Lo dice como lo que cambió entonces, no como la causa. Aparece en la fila de Diagnóstico y en el historial del mod.
- **Arreglos recordados**: cada arreglo que aplicás desde Diagnóstico queda anotado, y las partidas siguientes dicen si el error del mod dejó de salir.
- **Antes de jugar**: si las últimas partidas con casi la misma lista no llegaron ni al menú o se quedaron sin memoria, la app lo avisa al apretar Jugar, con lo que es distinto desde la última que llegó a jugar, y pregunta si jugar igual.
- **Historial por mod**: en la Lista de mods, la sección **En esta PC** del panel derecho.
- **Carga más lenta** y **actualización del juego** que hizo fallar a varios mods a la vez.
- **Caídas de verdad**: cuando Java se cae deja un informe (`hs_err_pid…log`) en la carpeta del juego; el cerebro lo lee y lo une a su partida ("se cayó: la PC se quedó sin memoria" o "error fatal de Java en tal biblioteca"). Cerrar el juego con la X o Alt+F4 no deja ninguno, así que eso nunca cuenta como caída.

### Seguridad

La pestaña **Seguridad** lista lo que un mod podría hacer más allá de modificar el juego, leído de sus archivos: programas o scripts que trae, Java que puede ejecutar programas o conectarse a internet, y Lua que llama a cosas que los scripts del propio juego nunca llaman. Nada de esto es un veredicto: muchos mods hacen estas cosas por buenas razones. La mayoría de las filas son Información; un Aviso marca algo que ningún mod tiene buena razón para hacer. La fila de un mod del Workshop tiene **Abrir página del Workshop**.

### Tiempo por mod

En **Mods** > **Tiempo por mod**: qué mods frenan el juego mientras jugás. **Medir** pone un mod chico del Studio (PZ Mod Studio Profiler) primero en tu lista de mods y en la partida que jugaste por última vez; abrí el juego (reinicialo si estaba abierto) y jugá unos minutos como siempre. Cada handler de eventos de cada mod lo mide el propio juego y se suma por mod: la parte del tiempo real que se llevó, milisegundos por segundo y su llamada más larga (una larga es un congelamiento que se ve). Abrí una fila para ver los handlers que más cuestan, con su archivo y línea. **Dejar de medir** saca el medidor de la lista y de la partida; los últimos números quedan.

También cuentan las ventanas, barras y acciones con tiempo de un mod (como Clase:render, Clase:update); cada función cuenta solo su propio tiempo. El medidor le suma un poco de tiempo propio a cada handler, así que apagalo cuando termines.

## Juego

Lo que la app sabe del juego en sí, aparte de los mods. Cinco pestañas: **En vivo**, **Memoria**, **Arranque**, **Frames** y **Disco**.

### En vivo

Lo que usa el juego ahora: arriba, la memoria libre de la PC (el medidor de la barra de arriba trae a esta pestaña). Debajo, una tarjeta por cada juego abierto: memoria contra su límite de Java, CPU, en qué pantalla está (menú, conectando, en partida), la última línea de su registro, cuántos errores tiene, y botones para abrir su carpeta y su registro. Con el juego cerrado muestra lo de la última vez: el último arranque, el límite de memoria, las sesiones que se quedaron sin memoria y las veces que Windows se quedó sin memoria.

### Memoria

En **Juego** > **Memoria**: cuánta memoria puede usar el Java del juego (`-Xmx` en `ProjectZomboid64.json`), de la que tiene la PC, con una recomendación para tus mods activos y un aviso si en una sesión reciente se quedó sin memoria. Elegí un valor (de 2 GB hasta la mitad de la memoria de la PC) y **Aplicar** con el juego cerrado; se aplica en el próximo inicio, y una copia del archivo de antes del cambio queda en `PZModStudio_Backups`.

Cada perfil guarda su propio valor: al activar un perfil, el juego recibe su memoria. Un perfil sin valor propio usa el último elegido. Si una actualización del juego vuelve a poner su valor, el Studio pone el tuyo la próxima vez que abre el juego.

Si en los últimos 14 días Windows mismo se quedó sin memoria con el juego abierto, también aparece acá, con lo que ayuda (un archivo de paginación más grande, cerrar otros programas pesados). Eso no es la memoria del Java del juego, así que subir el valor de arriba no lo arregla.

### Arranque

En **Juego** > **Arranque**: cuánto tardó el juego en llegar al menú principal, etapa por etapa (archivos de los mods, scripts, Lua, modelos, etc.), según el log que deja cada inicio. Cada arranque se compara con tu arranque habitual con más o menos la misma cantidad de mods; una etapa en ámbar tardó mucho más que lo normal. Elegí un arranque anterior para verlo.

### Frames y tirones

En **Juego** > **Frames**: los tiempos de frame mientras jugás (fps, frames de más de 25, 50 y 100 ms), los chunks cargados y, por mod, los handlers que más tiempo trabaron un frame, con archivo y línea. Los cuenta el mismo medidor de **Mods** > **Tiempo por mod**, así que tocá **Medir** ahí primero y jugá unos minutos. **Volver a leer** lee los números más nuevos.

### Disco

Arriba, **Liberar espacio**: **Vaciar caché** borra lo que el Studio guarda para no volver a leer o pedir lo mismo (las revisiones de cada mod, lo que dijo Steam, las imágenes de los mapas, las traducciones); lo vuelve a armar solo y nunca toca mods, partidas, perfiles ni notas. Si hiciste un test multiplayer, **Carpeta del jugador 2** manda a la Papelera la carpeta `Zomboid2` del segundo juego (con copias de los mods del servidor); los mods en desarrollo enlazados ahí no se tocan, y el próximo test la vuelve a armar. No se puede mientras el test corre.

En **Juego** > **Disco**: cuánto ocupa cada mod instalado, contado por lo que se va junto (un ítem del Workshop con todos sus mods, una carpeta de `Zomboid/mods`). Medirlo tarda un par de segundos.

- **Nada lo usa** (ámbar): ningún perfil lo tiene, ni prendido ni apagado; ninguna partida lo nombra en su lista; ningún servidor guardado lo cargó; no está en la lista activa del juego.
- **Apagado en todos lados** (celeste): algún perfil lo guarda, pero apagado, y ninguna partida ni servidor lo usa.
- **Sobrante**: una carpeta del Workshop de un ítem al que ya no estás suscrito; Steam a veces las deja.
- **Tu subida**: un mod tuyo en `Zomboid/Workshop`. Se muestra, nunca se ofrece borrarlo.

Marcá los que quieras sacar. Un ítem del Workshop se va con **Desuscribirse de N ítems** (Steam tiene que estar abierto, y borra los archivos solo). Una carpeta local o un sobrante va con **Mandar N carpetas a la Papelera**: se puede recuperar desde la Papelera de reciclaje. Antes de cada una se pide confirmación con los nombres. Con el juego abierto no se manda nada a la Papelera.

Abajo, **Carpetas del juego y del Studio**: cuánto ocupan el juego, los mods del Workshop, los mods locales, las partidas, las copias de partidas, las versiones guardadas de mods, las copias de la lista de mods, la caché, las descargas a medio instalar y los registros, con un botón para abrir cada carpeta. Desde ahí no se borra nada.

## Servidor

El servidor que corre en esta PC (el que arrancás desde Servidores, o el de un test multiplayer), con la misma tarjeta: memoria contra su límite, CPU, si ya arrancó, la última línea de su registro y cuántos errores tiene. Durante un test multiplayer aparecen también sus jugadores. Sin ningún servidor corriendo, un botón lleva a **Servidores** para arrancar uno.

Un servidor de otra PC al que te unís no se puede medir desde acá: solo el que corre en esta PC.

## Para tener en cuenta

- Los arreglos de **Mods** se guardan en la lista de mods del perfil activo.
- Los archivos fusionados y los parches del shim se aplican la próxima vez que arranca el juego.
- "Llena la consola" se mide con el `console.txt` de la última sesión, así que se actualiza después de jugar.
- Hacer clic en la etiqueta de un problema en la Lista de mods abre esta pantalla en esa fila.
