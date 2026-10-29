# Fusionador de mods

El Fusionador de mods arregla archivos que cambian dos o más de tus mods activos, y te deja parchear un bug en un archivo propio de un mod. Tus decisiones van a un paquete: un mod chiquito propio que carga último, así que el juego lee tu versión de esos archivos.

## Qué es un paquete

- Un paquete es una carpeta de mod en `Zomboid/mods` cuyo nombre empieza con `Z_PZModStudio_` (el de siempre es `Z_PZModStudio_MergedPatch`).
- Cuando lo publicás, el Studio escribe tus archivos fusionados adentro (para Build 41 y en su carpeta `42` para Build 42), agrega un archivo chico de compatibilidad y pone el paquete **último** en la lista de mods del juego. El mod que carga último es aquel cuya copia de un archivo usa el juego, así que gana tu copia fusionada.
- Un paquete tiene dos estados. **🟢 EMPAQUETADO / CERRADO** quiere decir que está publicado y el juego lo lee. **🟡 ABIERTO (borrador)** quiere decir que lo estás editando; mientras es borrador se saca de la lista de mods del juego, así que el juego no lo lee hasta que publiques de nuevo.
- Cada paquete pertenece a los perfiles que lo tienen entre sus mods. Un paquete que creás con un perfil activo se agrega a ese perfil.
- Un paquete recuerda qué perfil estaba activo la última vez que se publicó (**armado con** en su tarjeta, en ámbar si ahora hay otro perfil activo): sus merges resuelven la lista de ese perfil. Con un perfil que no tiene paquete propio, **Paquete para "nombre"** en la barra de arriba crea uno con su nombre.

## Qué ves

Cuando no hay ningún paquete abierto, ves la lista **Paquetes de fusión**. Cada tarjeta muestra el nombre del paquete, sus archivos, su estado, los perfiles a los que pertenece y estos botones:

- **🔍 Abrir / Entrar al paquete** lo abre en el espacio de trabajo y escanea tus mods activos buscando conflictos.
- **Visible en la Lista de mods** / **Oculto en la Lista de mods** prende o apaga el paquete en la lista de mods del juego (`default.txt`).
- El lápiz edita el nombre y la descripción; el tacho lo borra del disco (primero te pregunta). Los perfiles que lo tenían se quedan con el nombre nuevo después de renombrarlo.
- El ícono de descarga lo guarda como archivo `.pzmerge` para compartirlo; el ícono de carpeta abre su carpeta en el Explorador de Windows.
- **Paquetes de otros perfiles** muestra los paquetes de otros perfiles; **Solo "nombre"** vuelve a los del perfil activo.

La barra de arriba tiene **Cargar paquete (.pzmerge)** para importar un paquete que te mandaron, y **➕ Paquete nuevo** para crear uno (un nombre, una descripción opcional y después **Crear paquete**). Los mods propios del Studio (el Puente IPC en vivo y el Studio Link) aparecen marcados como **Sistema**: podés ver sus archivos pero no editarlos, renombrarlos ni borrarlos.

## Trabajar dentro de un paquete

La barra de arriba muestra **← Volver a la lista**, un selector **Paquete:** para saltar a otro paquete, el estado y la visibilidad, y estos botones:

- **Parchear un archivo de un mod**: abre un archivo propio de un mod para arreglarlo (ver más abajo).
- **⚡ Fusionar todo y publicar**: fusiona todos los archivos en conflicto con el motor automático y publica el paquete de una vez: se escribe en `Zomboid/mods` y queda último en tu lista de mods, y el juego lo lee la próxima vez que arranca. Después revisá los archivos.
- **↩️ Desfusionar todo**: descarta todas las fusiones guardadas en este paquete. Queda una copia en `Zomboid/PZModStudio_Backups`, y los archivos ya publicados siguen hasta que publiques de nuevo.
- **Todos** / **Sin duplicados**: oculta los archivos donde los mods traen copias idénticas, así solo se listan los choques reales.
- Íconos de carpeta, exportar y reescanear.
- **📦 Empaquetar y publicar**: el paso final. Escribe la decisión actual de cada archivo en el paquete y pone el paquete último en la lista de mods del juego.

A la izquierda, **Conflictos del VFS** lista cada archivo que choca con su tipo (Lua, **DEFINICIÓN** para scripts de ítems y recetas, **TEXTOS** para traducciones) y su estado: **Requiere revisión**, **Autofusionado** o **Resuelto**.

## Decidir qué lee el juego para un archivo

1. Hacé clic en un archivo de la lista. La mitad de arriba muestra el archivo vanilla, la copia de cada mod y un resumen de lo que cambia cada mod respecto del juego base; los conflictos reales (las mismas líneas cambiadas de forma distinta) quedan marcados. Hacé clic en un cambio para llevar todos los paneles hasta ahí.
2. En **¿Qué debería leer el juego?**, elegí una respuesta:
   - **Conservar ambos** (para Lua y otros archivos): los cambios de todos los mods juntos. Donde dos cambian las mismas líneas, gana el que carga último y el otro queda en un comentario.
   - **Fusionarlos** (para definiciones y traducciones): lo que admite ambos valores conserva ambos; donde solo entra uno, queda lo que el juego hace hoy y el otro queda en un comentario.
   - **Solo (nombre del mod)**: la versión de ese mod. En una definición, lo que definen los otros mods y este no toca se sigue aplicando.
   - **Vanilla**: el archivo propio del juego, como si ningún mod lo hubiera tocado (no aparece cuando el juego base no tiene ese archivo).
3. O editá el código en el editor de abajo; entonces muestra **Tu propia edición**. **Agregar** y **Quitar** en el panel de un mod suman o sacan la copia entera de ese mod, y **Cargar base** pone el archivo vanilla en el editor.
4. Apretá **✓ Guardar la fusión de este archivo**. El archivo pasa a **Resuelto**.
5. Cuando terminaste con todos los archivos, apretá **📦 Empaquetar y publicar**.

**Restaurar fusión AST** vuelve a poner la fusión automática en el editor, y **Desfusionar este archivo** olvida la decisión guardada para ese archivo. Arrastrá la barra entre los paneles para cambiarles el tamaño.

## Parchear un bug en un mod

Usá **Parchear un archivo de un mod** cuando un mod tiene un bug y no hay nada que fusionar. Elegí el mod, después el archivo Lua o de script, editalo y apretá **✓ Guardar la fusión de este archivo**. Tu copia va al paquete y el juego la lee en vez de la del mod, solo para ese archivo; el mod sigue activo y el resto queda intacto. Guardar una copia sin cambios se rechaza, porque solo congelaría el archivo en la versión de hoy.

El Centro de monitoreo y Salud de mods te pueden mandar acá directo: se abre el paquete correcto y el archivo que nombró el error se abre en el editor, a veces con el arreglo ya aplicado.

## Paquetes empaquetados y publicar en el lugar

Un paquete empaquetado es de solo lectura: sus archivos quedan ocultos detrás de un resumen hasta que apretás **🔓 Desfusionar / Abrir para editar** (o **Desempaquetar para editar**) y confirmás. Desempaquetar saca los archivos publicados del juego hasta que publiques de nuevo.

Un archivo propio de un mod abierto para parchear es la excepción: en un paquete empaquetado, guardarlo lo **publica en el lugar**. Todo lo que tiene el paquete se publica de nuevo con tu archivo al lado, el paquete sigue empaquetado y el juego lee el archivo nuevo la próxima vez que arranca. Cuando Salud de mods o el Centro de monitoreo te mandan a un archivo de un paquete empaquetado, ese archivo se muestra sin desempaquetar.

## Compartir paquetes

**Cargar paquete (.pzmerge)** importa un archivo de paquete. Si se hizo con mods que no tenés, o con otras copias de ellos, una ventana **Antes de publicar** te los lista, con **Suscribirse a todos** para los mods del Workshop que faltan. Lo más seguro es que los dos tengan las copias más nuevas; después reescaneá y volvé a fusionar antes de publicar.

## Bueno saber

- El juego lee un paquete cuando arranca: después de publicar, reiniciá el juego si está abierto.
- Si no se encuentra la carpeta del juego, la pantalla te pide que primero configures las rutas en **Configuración**.
- Si dejás un archivo con ediciones sin guardar, te pregunta si querés descartarlas.
- Borrar un paquete lo saca del disco; los mods que fusionaba no se tocan.
