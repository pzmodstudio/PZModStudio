# Centro de monitoreo

El Centro de monitoreo muestra el log del juego a medida que se escribe y convierte cada error en una tarjeta que dice a qué mod culpó el juego y qué podés hacer. Sigue `console.txt` tanto si abriste el juego desde el Studio como desde Steam.

## Qué ves

- A la izquierda, el visor de logs, con una pestaña por log.
- A la derecha, **DIAGNÓSTICO**: las tarjetas de error y, arriba de ellas, el panel **Encontrar el mod que lo causa**.
- Arriba, **📁 Carpeta de logs** abre en el Explorador de Windows la carpeta con `console.txt` y la subcarpeta `Logs`.
- Al lado, el botón del filtro de spam. Mientras dice **Filtro de spam: ACTIVO** oculta las líneas de traducciones faltantes; apretalo para pasar a **Filtro: MOSTRAR TODO**.

Si todavía no configuraste la carpeta del juego, la pantalla solo muestra **Configurar las rutas en Configuración**.

## El visor de logs

Siempre hay dos pestañas:

- **console.txt (Cliente)**: el log del juego en vivo. El Studio empieza a leer desde el final del archivo, así que ves lo que pasa desde que el Studio está abierto. Cuando empieza una sesión nueva del juego, el archivo arranca de cero y la pestaña también.
- **Servidor dedicado (en vivo)**: el log del servidor dedicado, mientras está corriendo uno que iniciaste en **Servidores**.

Para leer otros logs, apretá **Cargar log**. El menú lista los logs que encuentra en tu carpeta Zomboid (con su tamaño); elegí uno para abrirlo en una pestaña nueva. **📂 Buscar otro archivo (.txt)...** abre cualquier archivo de texto. Estas pestañas se cierran con su X; las dos pestañas en vivo no.

Debajo de las pestañas:

- El buscador deja solo las líneas que contienen lo que escribís.
- Los botones **Todo**, **🚨 Errores**, **🌙 Lua** y **☕ Java** dejan solo las líneas de ese tipo. **⚡ Puente** aparece solo con las funciones de Desarrollador activadas.
- El pie muestra cuántas líneas se ven sobre el total.

Los botones chicos a la derecha de las pestañas:

- El del disquete guarda lo que tiene la pestaña como un archivo nuevo en `Zomboid/Logs/`.
- El de copiar copia al portapapeles las líneas que ves (después de los filtros).
- El del tacho vacía el visor de esta pestaña. No toca el archivo en el disco.

Las líneas de error se ven en rojo y los avisos en ámbar.

## Las tarjetas de error

Aparece una tarjeta cuando el juego lanza un error, o cuando escribe un aviso y nombra un mod. Todo lo demás queda en el log de la izquierda. Primero van las tarjetas de errores (rojas) y después los avisos (ámbar, marcados **aviso · el juego siguió**). El mismo problema aparece una sola vez, aunque el juego lo repita muchas veces; el número tipo **600×** dice cuántas veces está en el log, y los que se repiten son los que conviene arreglar primero.

Una tarjeta puede mostrar:

- **El nombre del mod**, cuando el juego lo nombró en la línea del error o en el stack trace de abajo.
- **llamado desde ...**: los mods cuyo código llamó al que falló, leídos de la pila del propio juego. La falla puede estar en cualquiera de los dos.
- **Fragmento del log**: la línea en sí.
- Para un `require` que falló, dónde está ahora ese módulo, cuando el Studio lo encuentra.
- Qué hacer, o un aviso de que el shim de Build 42 ya lo cubre.

Cuando empieza una sesión nueva del juego, se borran las tarjetas de la anterior. **Descartar** las borra a mano.

## Arreglar desde una tarjeta

- **Arreglar la ruta** (o **Arreglar la ruta en Archivo.lua**): aparece cuando un mod pide un módulo con una ruta vieja y el Studio conoce la nueva. Reescribe el `require` en todos los archivos de los mods activos que lo piden y los publica en tu paquete del Studio. El mod no se modifica y sigue activo. Una línea verde dice qué se arregló; relanzá el juego.
- **Parchear Archivo.lua** o **Parchear un archivo suyo**: abre los archivos de ese mod en el Fusionador de mods para que corrijas uno. Tu copia va a un paquete que carga último, así que el juego la lee en lugar de la del mod. Los avisos sin un arreglo conocido no tienen este botón.
- **Quitar la copia de Archivo.lua**: aparece cuando el error está en un archivo que tu paquete del Studio ya parcheó. Saca esa copia del paquete (después de preguntarte) y el juego vuelve a leer el archivo propio del mod. Cualquier edición que hayas hecho en la copia se pierde.
- **Apagarlo**: pide confirmación, apaga el mod y te lleva a la Lista de mods.
- **Un paquete de merge puede arreglar esto — abrir el Merger**: para errores que resuelve el shim de Build 42. El shim viaja dentro de un paquete de merge publicado; la tarjeta te dice si el tuyo ya está activo o sigue como borrador.

## Informes

- **Guardar informe** escribe un archivo Markdown con el error, tu build del juego, el mod y su código, más las líneas del log de alrededor. Queda listo para pasárselo al autor del mod o a un asistente de IA.
- **Copiar informe** pone el mismo informe en el portapapeles.
- **🔍 Ver en los logs** pone el archivo o la palabra clave del error en el buscador de la izquierda.
- **Copiar la línea** copia solo la línea del error.

## Encontrar el mod que lo causa

Para problemas que el log no nombra: un congelamiento, una receta que nunca aparece, una partida que no abre.

1. Abrí el panel, apretá **Empezar a partir la lista** y confirmá.
2. Queda activa la mitad de los sospechosos. Los mods que otro mod requiere siguen activos en todas las rondas, así nada se rompe por ese motivo.
3. Apretá **Lanzar el juego**, jugá como cuando pasó y cerralo.
4. Respondé **Sigue pasando** o **Ya no pasa**. La lista se vuelve a partir. La etiqueta muestra la ronda y cuántos inicios quedan (unos 8 para cien mods).
5. Cuando queda un solo mod, jugá una vez más para confirmarlo.

Al final el panel nombra el mod, muestra las líneas que el log tenga sobre él y ofrece **Parchear un archivo suyo**, **Apagarlo** o **Dejarlo activo**. Tu lista original se restaura termine como termine la búsqueda. **Parar y restaurar mi lista** la corta antes.

Si ningún mod solo tiene la culpa, el panel explica por qué: pueden ser dos mods juntos, uno de los mods requeridos, o algo que no es un mod.

## Para tener en cuenta

- La búsqueda escribe tu Lista de mods por su cuenta. No actives ni apagues mods en la Lista de mods mientras corre, o la siguiente respuesta no sirve.
- La búsqueda sobrevive a cerrar el Studio; se guarda en `Zomboid/PZModStudio_Bisect.json`.
- **Lanzar el juego** en la búsqueda abre el juego normal, sin modo debug.
- Los arreglos publicados en un paquete se aplican la próxima vez que arranca el juego.
