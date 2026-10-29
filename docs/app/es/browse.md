# Explorar mods

Explorar mods es donde encontrás y conseguís mods nuevos: buscás en el Steam Workshop y en Nexus Mods, revisás lo que ya tenés, armás tus propias listas e instalás mods que descargaste por tu cuenta.

## Qué ves

- Cuatro pestañas arriba: **Steam Workshop**, **Nexus Mods**, **Mis mods** (con cuántos tenés) y **Mis listas**. Cada pestaña conserva su página y sus filtros cuando cambiás de pestaña.
- A la derecha, los botones para instalar: **Instalar .zip / .7z**, **Carpeta** y **Desde un enlace**.
- Cada mod es una tarjeta con su imagen, etiquetas de build (como **Build 42**), si está **Instalado**, y etiquetas como **Actualización pendiente** o **Nuevo desde la última partida**. Dejá el puntero sobre una tarjeta para ver más; hacé clic para abrir su vista de detalle; clic derecho para un menú.

## Steam Workshop

- Escribí en el buscador y apretá **Buscar**. También podés pegar uno o más enlaces del Workshop, ids o el enlace de una colección: el Studio muestra justo esos ítems y cuántos todavía no tenés.
- **Orden**: **Más populares**, **Mejor valorados**, **Más recientes** o **Última actualización**. Con Más populares también elegís el período (**Hoy**, **Una semana**, **Tres meses**, **Un año**). Una búsqueda por texto siempre se ordena por relevancia.
- **Build del juego**: **Build 42**, **Build 41** o **Cualquier build**.
- **Mods** o **Colecciones**: qué listar. Con Mods también podés elegir una categoría.
- **Web** abre el Workshop en tu navegador.

Si Steam rechaza la búsqueda, un recuadro explica por qué. Si Steam solo está ocupado, apretá **Probar de nuevo**. Si no, podés pegar una clave personal de la Steam Web API y apretar **Verificar y guardar**; se guarda en el Administrador de credenciales de Windows, solo en esta PC, y **Quitarla** la borra. Pegar enlaces funciona sin clave.

## Nexus Mods

Buscá por palabras del nombre del mod, ordená (**Actualizados hace poco**, **Más descargados**, **Más recomendados**, **Más nuevos**) y elegí una categoría. Los mods para adultos quedan ocultos salvo que marques **Contenido adulto**.

Las descargas de Nexus se hacen en el sitio de Nexus. En la vista de detalle de un mod de Nexus, **Descargar en Nexus** abre su pestaña Files en tu navegador. Cuando termina la descarga, apretá **Instalar .zip / .7z** y elegí el archivo; el Studio lo marca como mod de Nexus.

## Mis mods

Todo lo que tenés instalado: suscripciones del Workshop, mods de Nexus y mods locales. Filtrá por nombre o id, elegí un **Origen**, ordená por **Actualizados hace poco** o **Nombre**, y usá las etiquetas **Nuevos desde la última partida** y **Actualización pendiente en Steam** para ver solo esos. Clic derecho en un mod del Workshop para **Desuscribirse**.

## La vista de detalle

Hacer clic en una tarjeta abre una vista grande con imágenes, la descripción, información y requisitos. **Volver** regresa a la anterior, y Esc o la X la cierran.

- **Suscribirse** / **Desuscribirse**: se hace a través del cliente de Steam, así que Steam tiene que estar abierto. Steam descarga los archivos; el mod aparece en tu Lista de mods cuando termina. Sin Steam, el botón dice **Abrir la página para suscribirse**.
- **Ítems requeridos**: lo que el mod necesita del Workshop, marcado **Instalado** o **Falta**. Hacé clic en uno para abrirlo.
- **Índice comunitario**: lo que sabe el índice de la comunidad sobre el mod: builds en las que funciona, mods con los que no va, problemas conocidos y si el Studio los arregla, y una versión más nueva para tu build si existe.
- La estrella del encabezado lo agrega a tus **Favoritos**.

En una colección tenés **Suscribirse a los N que faltan**, **Crear un perfil con esto** (un perfil con los mods de la colección que tenés instalados) y la lista de lo que te falta.

## Suscribirte a lo que necesita un mod

Cuando te suscribís y el mod requiere otros ítems del Workshop que no tenés, una ventana lista toda la cadena. Los que faltan vienen marcados. Elegí **Suscribirse a los N** para traerlos juntos, **Solo X** para traer solo el que pediste, o **Cancelar**. Los ítems que ya no están en el Workshop aparecen indicados.

## Mis listas

Tus propias listas de mods, guardadas en esta PC. **Favoritos** siempre está.

- Escribí un nombre en **Lista nueva…** y apretá **+** para crear una.
- Agregá mods con la estrella de la vista de detalle, o con clic derecho en cualquier mod y **Favorito**, **Agregar a una lista…** o **Agregar a un perfil…**.
- **Suscribirse a todo lo que falta** te suscribe a los ítems del Workshop de la lista que no tenés.
- **Crear un perfil** crea un perfil con los mods instalados de la lista activos.
- El lápiz renombra la lista y el tacho la borra. Los mods quedan instalados.
- Clic derecho en una tarjeta y **Sacar de** la lista para quitarla solo de la lista.

## Listas de la comunidad

**De la comunidad** (al lado de **Mis listas**) muestra las listas que compartieron otros jugadores. Cada una la leyó una persona antes de publicarse en el repositorio del Studio; el Studio las trae una vez por día (las flechas las traen ahora). **Todas las listas en la web** abre la página de la comunidad con cada lista y sus mods.

- Cada tarjeta dice quién la hizo, cuántos mods tiene y cuántos de sus mods del Workshop ya tenés.
- **Copiar a mis listas** la hace una de tus listas, con el nombre de su autor. No se instala nada: desde ahí, **Suscribirse a todo lo que falta** y **Crear un perfil** funcionan como con cualquier lista.
- **Ver en GitHub** abre el archivo.

Para compartir una tuya, abrila en **Mis listas** y apretá **Compartir con la comunidad**. Se te pide tu nombre como autor y una línea sobre la lista. Con un token de GitHub (Ajustes) el Studio te muestra lo que sale y abre el pull request; sin token, se abre la página de archivo nuevo de GitHub con la lista completa, y ahí apretás **Propose changes**. Si alguien ya compartió una lista con el mismo nombre, renombrá la tuya antes.

## Instalar un mod que descargaste

- **Instalar .zip / .7z**: elegí un archivo comprimido. El Studio encuentra el mod adentro y lo pone en `Zomboid/mods`.
- **Carpeta**: para un `.rar` o algo ya descomprimido.
- **Desde un enlace**: pegá un enlace directo a un `.zip` o una página de GitHub. El Studio muestra qué va a descargar y pregunta antes.

Si el mod ya está instalado, te pregunta antes de reemplazarlo; la copia vieja se mueve a `Zomboid/PZModStudio_Backups/mods`. Una línea verde confirma la instalación. Después activá el mod en la Lista de mods.

## Para tener en cuenta

- Suscribirse y desuscribirse necesitan el cliente de Steam abierto.
- Después de desuscribirte, Steam borra los archivos más tarde, así que el mod puede seguir en tu Lista de mods hasta que reinicies Steam.
- Todas las páginas web se abren en tu navegador.
