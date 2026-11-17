# Lista de mods

Tus mods instalados: cuáles están prendidos y en qué orden los carga el juego. Cada cambio que hacés acá se guarda al instante en el perfil activo y en `Zomboid/mods/default.txt`, la lista que lee el juego.

## Qué ves

- El encabezado cuenta ítems del Workshop, mods, mods **activos**, y mods de mapa, librerías y mods de script. Cuando hay un problema aparecen contadores rojos y ámbar (**Dependencias faltantes**, **Dependencias apagadas**, **Incompatibilidad**, **Orden de la lista**); hacé clic en uno para recorrer esos mods.
- La barra de herramientas: buscador, filtros, grupos y los botones **Activar todo** / **Apagar todo**.
- La lista. La barra verde de arriba (**CARGA PRIMERO**) es donde van las librerías; la barra ámbar de abajo (**CARGA ÚLTIMO**) es donde un mod gana cualquier archivo que compartan dos mods.
- A la derecha, el panel del mod que elegiste (vacío hasta que hacés clic en uno). Más abajo, en **El panel de la derecha**.

## El panel de la derecha

Arriba, siempre a la vista: la imagen del mod (clic para verla grande), si es de mapa, librería o de script, de dónde viene (Steam, copia compatible, local...), el nombre, el ID, el autor y la versión, y los botones **Apoyar al autor** con los links que su autor puso en el changelog o en el Workshop (Ko-fi, Patreon...), que se abren en tu navegador. Debajo, un botón para prenderlo o apagarlo que dice en qué lugar carga, y **Ver en Explorar mods**.

Después, tres pestañas (la que elijas queda puesta al cambiar de mod):

- **Studio**: lo que la app sabe de este mod en tu lista.
  - **Notas**: tu nota completa (**Editar nota**, o **Agregar una nota**), tus grupos y en qué perfiles está, con el activo resaltado.
  - **Qué hacer**: cada aviso de la revisión con todo lo que dice, si hay que hacer algo, conviene o es solo para saber, y su botón de arreglo. Si no hay nada, lo dice en verde.
  - **Última sesión**: los errores o avisos que registró el juego (con la última línea), cuánto tardó en cargar y su parte del tiempo del juego.
  - **Dependencias**: qué necesita y qué mods lo necesitan; clic en uno para ir a su fila.
  - **Datos del mod** (lo que dice su `mod.info`, con **Lo último de su changelog**) y **Clases que cambia su Java**, plegados al final.
  - **Qué hacer** y **Última sesión** se pliegan con un clic en su título, y quedan como los dejaste.
- **Página del Workshop**: la página del mod sin salir de la app: imágenes y videos, suscriptores, favoritos, tamaño, fechas, autor, categorías, los ítems que requiere (verde si lo tenés: clic para ir a su fila; ámbar si no: se abre en Explorar mods) y la descripción completa con **Traducir**. Un mod que no está en el Workshop muestra acá su descripción.
- **Comentarios**: los comentarios de su página del Workshop, los más nuevos primero, con **Cargar más**. Si el perfil del autor es privado, Steam no los muestra y el panel lo dice.

## La columna #

El número de cada fila es el lugar del mod entre los mods **prendidos**: el orden de carga que lee el juego, y el mismo número que muestra el juego. Un mod apagado muestra **—** y se ve en gris, porque no tiene lugar en el orden de carga. Los mods apagados conservan su lugar en la lista, así que al prenderlos normalmente vuelven a donde estaban.

## Prender y apagar mods

- Hacé clic en el cuadrado de la fila, o clic derecho y elegí **Activar** / **Desactivar**.
- Cuando un mod necesita otros, una ventana muestra lo que se prende con él (y lo que se apaga con él). Elegí **Prender los N**, o **Solo ...** para ese mod solo.
- **Activar todo** y **Apagar todo** actúan solo sobre las filas que se ven ahora (después del buscador y los filtros), y preguntan antes.
- **Deshacer** / **Rehacer** (las flechas al lado, o Ctrl+Z / Ctrl+Y) van para atrás y para adelante por los cambios hechos acá: movimientos, arrastres, activar o apagar, ordenar. Cada perfil tiene los suyos, también después de cerrar la app.
- Ctrl+clic elige varios mods, Shift+clic elige un rango, Esc los suelta. Clic derecho sobre los elegidos: activarlos o apagarlos, sacarlos del perfil, o desuscribirte de sus ítems del Workshop.
- Un paquete abierto en borrador en el Fusionador no se puede prender hasta empaquetarlo.

## Cambiar el orden de carga

- Arrastrá una fila desde su número y soltala arriba o abajo de otra. Las filas ocultas en el medio se respetan.
- Los botones de flechas a la derecha: mover al principio, subir uno, bajar uno, mover al final. Subir y bajar intercambian con el mod de al lado en la lista completa, que puede ser uno apagado.
- **Pos #**: escribí un número y tocá **Ir** o Enter. El mod va a ese lugar entre los mods prendidos.
- **Ordenar dependencias**: pone cada mod después de lo que requiere y de sus `loadModAfter`, antes de sus `loadModBefore`, y después de los mods que digan tus propias reglas. Todo lo demás conserva tu orden; los paquetes de PZ Mod Studio van al final. Una ventana dice cuántos mods se movieron, con **Deshacer**.

## Encontrar mods

- El buscador busca en el nombre, el ID del mod, el ID del Workshop y tus notas.
- Los botones de filtro: **Todos**, **Activos**, **Desactivados**, **Favoritos**, **Actualizados**, **Problemas**, **Mapas**, **Librerías**, **No cargan**. Cada uno muestra su cantidad.
- Debajo: **Orden: de carga**, **Orden: nombre**, **Orden: instalados más nuevos** u **Orden: instalados más viejos** (por cuándo se instaló cada mod: cuando se creó su carpeta; para uno del Workshop, la primera descarga, que las actualizaciones no cambian; cada fila muestra la fecha y los que no están instalados van al final), el menú de **Grupos**, uno de tus grupos, el origen (**Steam Workshop**, **Tus subidas al Workshop**, **Instalados a mano**, **Copias compatibles**...), una etiqueta (errores o avisos de la última sesión, carga lenta, pesado para el juego, un problema de Diagnóstico) y **Mods del perfil** / **Todos los instalados**.
- La pantalla de Mods del juego (mod Studio Link) tiene los mismos filtros en el mismo orden. Actualizados, Librerías, el grupo, el origen y la etiqueta necesitan PZ Mod Studio; sin la app, el juego lo dice donde irían.
- Arrastrar funciona también ordenado por nombre: el mod va al lugar del mod sobre el que lo soltás.
- **quitar filtros** vuelve a mostrar todos los mods.

## Grupos y notas

- El menú de grupos arma la lista en secciones plegables: **Grupos: ninguno**, **Grupos: autor**, **Grupos: categoría** o **Grupos: míos**. Cada encabezado muestra cuántos están prendidos, con **Activar todos** / **Desactivar todos**. En **Grupos: míos**, un botón de nota edita la nota del grupo.
- Clic derecho en un mod y elegí **Grupos…** para marcar en qué grupos está, crear uno nuevo (**Agregar**), renombrar o borrar uno. Un grupo que queda vacío desaparece. Los grupos nunca cambian el orden de carga.
- Al elegir uno de tus grupos aparecen **Activar el grupo** (con lo que requieren sus mods) y **Desactivar el grupo**.
- Clic derecho y elegí **Agregar una nota…** para escribirle una nota a un mod (también desde el panel de la derecha). Su fila muestra el comienzo de la nota al lado del nombre: pasá el mouse para leerla entera, o hacé clic para editarla.
- Cada perfil tiene sus propios grupos: un grupo es del perfil donde lo creaste. Las notas son sobre un mod y son las mismas en todos los perfiles.
- Las notas y los grupos son solo tuyos. El juego los muestra (los grupos del perfil activo) en la pantalla de Mods del mod Studio Link.

## Los mods del perfil

Con un perfil activo, la lista muestra los mods de ese perfil. El botón **Mods del perfil** cambia a **Todos los instalados**, que lista todos los mods instalados y marca en qué perfiles está cada uno. Los mods instalados que no están en ningún perfil (suscriptos desde la web de Steam, por ejemplo) muestran una barra azul con **Agregar al perfil** (entran apagados), **Mostrarlos**, o la X para ocultarla.

## El menú del clic derecho

**Activar/Desactivar**, **Agregar al perfil** / **Sacar del perfil** (sacarlo también lo apaga), **Mover al principio**, **Mover al final**, **Abrir en Explorar mods** (**Buscarlo en Explorar mods** para un mod que no está instalado), **Abrir su carpeta**, **Agregar una nota…** (**Editar la nota…** cuando ya tiene una), **Grupos…**, **Reglas de orden de carga…**, **Guardar esta versión**, **Versiones guardadas…**, **Favorito**, **Agregar a una lista…**, **Agregar a un perfil…**, **Copiar el id del mod** y **Desuscribirse de su ítem del Workshop** (Steam borra sus archivos; los otros mods del mismo ítem se van también).

- **Reglas de orden de carga…**: decí que un mod debe cargar después de otro (lo que descubriste jugando). Ordenar sigue la regla y Diagnóstico avisa cuando la lista la rompe. Para pasarle tu orden a otra persona, exportá un preset: lleva la lista en el orden que le dieron tus reglas.
- **Guardar esta versión** copia el mod tal como está hoy en `Zomboid/PZModStudio_Snapshots`. **Versiones guardadas…** lista las copias: **Usar esta versión** repone una como copia local que el juego lee en vez de la del Workshop.

## Etiquetas de las filas

Una fila muestra solo lo que pide algo, para que la lista se pueda leer:

- Los avisos de Diagnóstico que piden hacer algo o conviene arreglar (como **Necesita un mod apagado** o **Carga demasiado temprano**), con el botón de la llave al lado que aplica el arreglo. Clic en la etiqueta para ir al mod relacionado o a Diagnóstico.
- Los que son solo para saber se juntan en una etiqueta chica con una **i** y cuántos son; el panel de la derecha los muestra todos.
- Errores o avisos que el juego registró del mod la última vez, una carga de más de 1 segundo, y su parte del tiempo del juego desde el 5 % (medida en Diagnóstico > Mods > Tiempo por mod).
- El origen solo cuando no es Steam (**Local**, **Copia compat.**...).
- Con **Todos los instalados**, un ícono de perfiles con un número: **✓** si está en el perfil activo (y **+N** si está en otros), o en cuántos perfiles está. Pasá el mouse para ver los nombres.

Los tiempos de carga más cortos, el origen Steam y todo lo demás están en el panel de la derecha.

## Listas anteriores y presets

- **Listas anteriores** es deshacer para la lista. Cada vez que el Studio la escribió, guardó la anterior. Elegí una y tocá **Volver a esta**; tu lista de ahora también queda guardada.
- El botón **Presets** guarda la lista como preset con nombre, carga uno, o exporta e importa archivos `.pzpack`. Un pack importado muestra los mods que te faltan, con **Suscribirse a todos**.

## Bueno saber

- **Actualizar** vuelve a leer las carpetas de mods después de suscribirte o desuscribirte.
- El mod PZ Mod Studio Link queda prendido en todos los perfiles y no aparece en la lista mientras hay un perfil activo.
- Con el juego abierto, lo que cambiás acá llega al juego en el menú principal (a través del mod Studio Link), o con **Sincronizar con PZ Mod Studio** en su pantalla de Mods.
