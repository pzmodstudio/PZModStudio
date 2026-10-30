# Lista de mods

Tus mods instalados: cuáles están prendidos y en qué orden los carga el juego. Cada cambio que hacés acá se guarda al instante en el perfil activo y en `Zomboid/mods/default.txt`, la lista que lee el juego.

## Qué ves

- El encabezado cuenta ítems del Workshop, mods, mods **activos**, y mods de mapa, librerías y mods de script. Cuando hay un problema aparecen contadores rojos y ámbar (**Dependencias faltantes**, **Dependencias apagadas**, **Incompatibilidad**, **Orden de la lista**); hacé clic en uno para recorrer esos mods.
- La barra de herramientas: buscador, filtros, grupos y los botones **Activar todo** / **Apagar todo**.
- La lista. La barra verde de arriba (**CARGA PRIMERO**) es donde van las librerías; la barra ámbar de abajo (**CARGA ÚLTIMO**) es donde un mod gana cualquier archivo que compartan dos mods.
- A la derecha, el inspector del mod elegido: sus datos, lo que requiere, y avisos con botones (**Mover después de ...**, **Activar la librería**, **Desactivar**).

## Apoyar al autor

El inspector muestra **Apoyar al autor** con los links que su autor puso en el changelog del mod o en su descripción del Workshop (Ko-fi, Patreon, Buy Me a Coffee...), que se abren en tu navegador, y, cuando el mod trae un changelog, **Lo último de su changelog**, que despliega su entrada más nueva. Los mismos botones están en la página de un mod del Workshop en Explorar mods, instalado o no.

## La columna #

El número de cada fila es el lugar del mod entre los mods **prendidos**: el orden de carga que lee el juego, y el mismo número que muestra el juego. Un mod apagado muestra **—** y la etiqueta **APAGADO**, porque no tiene lugar en el orden de carga. Los mods apagados conservan su lugar en la lista, así que al prenderlos normalmente vuelven a donde estaban.

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
- Debajo: **Orden: de carga**, **Orden: nombre**, **Orden: instalados más nuevos** u **Orden: instalados más viejos** (por cuándo se instaló cada mod: cuando se creó su carpeta; para uno del Workshop, la primera descarga, que las actualizaciones no cambian; cada fila muestra la fecha y los que no están instalados van al final), el menú de **Grupos**, uno de tus grupos, el origen (**Steam Workshop**, **Tus subidas al Workshop**, **Instalados a mano**, **Copias compatibles**...), una etiqueta (errores o avisos de la última sesión, carga lenta, pesado para el juego, un problema de Salud de mods) y **Mods del perfil** / **Todos los instalados**.
- La pantalla de Mods del juego (mod Studio Link) tiene los mismos filtros en el mismo orden. Actualizados, Librerías, el grupo, el origen y la etiqueta necesitan PZ Mod Studio; sin la app, el juego lo dice donde irían.
- Arrastrar funciona también ordenado por nombre: el mod va al lugar del mod sobre el que lo soltás.
- **quitar filtros** vuelve a mostrar todos los mods.

## Grupos y notas

- El menú de grupos arma la lista en secciones plegables: **Grupos: ninguno**, **Grupos: autor**, **Grupos: categoría** o **Grupos: míos**. Cada encabezado muestra cuántos están prendidos, con **Activar todos** / **Desactivar todos**. En **Grupos: míos**, un botón de nota edita la nota del grupo.
- Clic derecho en un mod y elegí **Grupos…** para marcar en qué grupos está, crear uno nuevo (**Agregar**), renombrar o borrar uno. Un grupo que queda vacío desaparece. Los grupos nunca cambian el orden de carga.
- Al elegir uno de tus grupos aparecen **Activar el grupo** (con lo que requieren sus mods) y **Desactivar el grupo**.
- Clic derecho y elegí **Agregar una nota…** para escribirle una nota a un mod. Su fila muestra un ícono de nota.
- Cada perfil tiene sus propios grupos: un grupo es del perfil donde lo creaste. Las notas son sobre un mod y son las mismas en todos los perfiles.
- Las notas y los grupos son solo tuyos. El juego los muestra (los grupos del perfil activo) en la pantalla de Mods del mod Studio Link.

## Los mods del perfil

Con un perfil activo, la lista muestra los mods de ese perfil. El botón **Mods del perfil** cambia a **Todos los instalados**, que lista todos los mods instalados y marca en qué perfiles está cada uno. Los mods instalados que no están en ningún perfil (suscriptos desde la web de Steam, por ejemplo) muestran una barra azul con **Agregar al perfil** (entran apagados), **Mostrarlos**, o la X para ocultarla.

## El menú del clic derecho

**Activar/Desactivar**, **Agregar al perfil** / **Sacar del perfil** (sacarlo también lo apaga), **Mover al principio**, **Mover al final**, **Abrir en Explorar mods** (**Buscarlo en Explorar mods** para un mod que no está instalado), **Abrir su carpeta**, **Agregar una nota…** (**Editar la nota…** cuando ya tiene una), **Grupos…**, **Reglas de orden de carga…**, **Guardar esta versión**, **Versiones guardadas…**, **Favorito**, **Agregar a una lista…**, **Agregar a un perfil…**, **Compartir lo que el Studio vio de este mod…**, **Copiar el id del mod** y **Desuscribirse de su ítem del Workshop** (Steam borra sus archivos; los otros mods del mismo ítem se van también).

- **Reglas de orden de carga…**: decí que un mod debe cargar después de otro (lo que descubriste jugando). Ordenar sigue la regla y Salud de mods avisa cuando la lista la rompe. **Compartir esta regla** la ofrece a la comunidad en GitHub.
- **Guardar esta versión** copia el mod tal como está hoy en `Zomboid/PZModStudio_Snapshots`. **Versiones guardadas…** lista las copias: **Usar esta versión** repone una como copia local que el juego lee en vez de la del Workshop.

## Etiquetas de las filas

Las etiquetas de una fila vienen de la última sesión y de Salud de mods: errores o avisos que el juego registró del mod, cuánto tardó en cargar (desde 250 ms), su parte del tiempo del juego cuando lo mediste en Salud de mods (desde 1 %, ámbar desde 5 %), y problemas como **Necesita un mod apagado** o **Carga demasiado temprano**. Hacé clic en una etiqueta para ir al mod relacionado o a Salud de mods; el botón de la llave al lado aplica el arreglo.

## Listas anteriores y presets

- **Listas anteriores** es deshacer para la lista. Cada vez que el Studio la escribió, guardó la anterior. Elegí una y tocá **Volver a esta**; tu lista de ahora también queda guardada.
- El botón **Presets** guarda la lista como preset con nombre, carga uno, o exporta e importa archivos `.pzpack`. Un pack importado muestra los mods que te faltan, con **Suscribirse a todos**.

## Bueno saber

- **Actualizar** vuelve a leer las carpetas de mods después de suscribirte o desuscribirte.
- El mod PZ Mod Studio Link queda prendido en todos los perfiles y no aparece en la lista mientras hay un perfil activo.
- Con el juego abierto, lo que cambiás acá llega al juego en el menú principal (a través del mod Studio Link), o con **Sincronizar con PZ Mod Studio** en su pantalla de Mods.
