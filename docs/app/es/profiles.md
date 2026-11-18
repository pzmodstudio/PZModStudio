# Perfiles

Un perfil es una lista de mods con nombre: qué mods le pertenecen, cuáles están prendidos y en qué orden cargan. Acá creás perfiles, cambiás entre ellos con un clic, los compartís y los borrás. La mayoría de las otras pantallas quedan bloqueadas hasta que haya un perfil activo.

## Qué ves

- Arriba, cuántos perfiles tenés guardados y el botón **Nuevo perfil**, que abre una ventana que te guía en tres pasos para crear uno.
- Una tarjeta por cada perfil guardado, el activo primero. El activo tiene un marco verde, la etiqueta **Activo** y el botón **Ver sus mods**, que abre la Lista de mods; los demás dicen **Inactivo** y tienen **Activar (1 clic)**.
- Cada tarjeta muestra su descripción, cuántos mods están prendidos (**mods activos**), el **Total** de mods de su orden y la build: **Build 41/42** si la fijaste a mano, o **Jugado en ...** con la build en la que se jugó por última vez desde el Studio.

## Crear un perfil

Tocá **Nuevo perfil**: se abre una ventana con tres pasos (**Nombre**, **Qué mods**, **Revisar**), cada uno con una explicación arriba. **Siguiente** y **Atrás** pasan de uno a otro; Esc o la X la cierran sin crear nada.

1. Escribí un **Nombre del perfil** (obligatorio) y, si querés, una **Descripción (opcional)**.
2. En **Qué mods**, elegí con qué arranca el perfil:
   - **Los que están activos ahora**: los mods prendidos en este momento.
   - **Todo lo instalado**: todos los mods que encontró el Studio.
   - **Elegirlos**: una lista con buscador y botones **Todos** / **Ninguno**; marcá los mods que quieras.
   - **Ninguno todavía**: un perfil vacío. El Studio te pide confirmar, porque cambiar a él apaga todos los mods. Mientras hay un perfil activo, un botón **Buscar mods en Explorar mods** te lleva ahí para suscribirte antes.
3. En **Revisar** ves el nombre y cuántos mods arrancan prendidos. Tocá **Crear y activar**.

El perfil nuevo se crea y se activa en el momento, y conserva el orden de carga que ya tiene tu lista. Si el Studio todavía no encontró ningún mod (carpeta del juego o de Zomboid sin fijar en Configuración), no lo deja crear, para que un perfil no apague todos tus mods por error.

## Cambiar de perfil

Tocá **Activar (1 clic)** en una tarjeta. El Studio entonces:

- escribe los mods de ese perfil en la lista que lee el juego (`Zomboid/mods/default.txt`);
- lo marca como activo y abre la Lista de mods con su orden;
- instala el mod PZ Mod Studio Link y lo deja prendido en todos los perfiles, para que la pantalla de Mods del juego muestre el perfil.

Si un mod del perfil necesita un agente Java en el archivo de inicio del juego (Zombie Buddy, por ejemplo), primero aparece una ventana con lo que sale o vuelve a ese archivo. Tocá **Activar ...** para seguir. Si el juego está abierto, esa parte espera a que se cierre. Antes se guarda una copia del archivo de inicio.

## Aislar este perfil

**Aislar este perfil** deja aparte, para ese perfil:

- sus **partidas** de un jugador;
- las **opciones del juego**: gráficos, sonido y teclas;
- la **configuración de los mods**: lo que cada mod guarda en la carpeta Zomboid (sus opciones, sus ajustes, sus datos).

Con el perfil activo el juego usa solo lo suyo, y con otro perfil, lo de ese (o lo compartido, si el otro no está aislado). Al aislarlo elegís cuáles de las partidas compartidas se lleva; el resto queda compartido. Las opciones y la configuración de los mods empiezan con una **copia** de las que tenés ahora, así que el perfil arranca igual que antes y los otros perfiles conservan las suyas.

Los **mods en sí son los mismos para todos los perfiles**: no se copian ni se mueven, y no hay que instalar nada de nuevo. Que un mod (o una parte de un mod) esté prendido en un perfil y apagado en otro funciona como siempre. Tampoco se separan las partidas de servidores (`Saves/Multiplayer`), los registros del juego, los servidores, ni los archivos de la app y del Link.

Lo que pesa mucho y no es un ajuste no se copia al aislar un perfil: un archivo de más de 20 MB o una carpeta de más de 50 MB (la caché de un mod, grabaciones, volcados) queda **compartido**, igual para todos los perfiles desde ese momento, así aislar nunca llena el disco. Las opciones y las teclas del juego se copian siempre, pesen lo que pesen. Lo que quedó compartido se anota en `Zomboid/PZModStudio_ProfileSavesShared.json`.

Al cambiar de perfil la app pone cada cosa en su lugar con un renombre, sin copiar nada. Solo lo hace con Project Zomboid cerrado: el juego, y también cualquier servidor de Project Zomboid en esta PC (el servidor del Studio de la pantalla Servidores, o uno abierto a mano). Si hay uno abierto, la app dice cuál y te pide que lo cierres primero. Lo de los otros perfiles espera en `Zomboid/PZModStudio_ProfileSaves`. Si la PC se apaga a mitad de un cambio, la app lo termina la próxima vez que abre.

Cada partida y cada ajuste que la app mueve, copia o guarda en los respaldos queda anotado en el registro del Studio (`Zomboid/PZModStudio_Logs/studio.log`, las líneas con `[Profiles]`), de dónde a dónde, así que si parece faltar una partida podés ver adónde fue.

**Dejar de aislar** junta sus partidas con las compartidas y vuelve a poner las opciones y la configuración compartidas; las del perfil se guardan en `Zomboid/PZModStudio_Backups/ProfileSettings`, sin pisar nada. Borrar el perfil hace lo mismo, así que nunca se pierde una partida ni un ajuste.

## Editar, compartir y borrar

- El botón del lápiz (**Renombrar este perfil**) abre **Editar perfil**: cambiá el nombre, la descripción y la **Build del juego**. Dejá la build en **La última jugada** o fijá **Build 41** / **Build 42**. Con una build fijada, el Studio pregunta antes de iniciar este perfil en otra build, aun antes de haberlo jugado. Tocá **Guardar cambios**.
- El botón de compartir guarda el perfil en un archivo `.pzpack` que podés mandarle a quien quieras. Lleva solo los mods prendidos, en orden, con sus IDs del Workshop. Quien lo abra puede suscribirse a lo que le falte.
- El botón de la papelera borra el perfil después de confirmar. Si algunos de sus ítems del Workshop no los usa ningún otro perfil, la confirmación es una ventana que los lista (**Solo los usa este perfil**), marcados, con **Marcar todos** / **Desmarcar todos**. **Borrar y desuscribir** borra el perfil y te desuscribe de los ítems marcados (Steam tiene que estar abierto); **Borrar solo el perfil** los conserva. Los ítems que usa otro perfil, y los mods locales, nunca se tocan. Si el perfil usa paquetes de parche del Fusionador (`Z_PZModStudio_...`), una segunda pregunta ofrece borrarlos también; **Cancelar** los conserva.

## Bueno saber

- Un perfil borrado no se pierde: su archivo se mueve a `Zomboid/PZModStudio_Backups/profiles`.
- Si borrás el perfil activo, no queda ningún perfil activo y la pantalla de Mods del juego deja de mostrar un perfil.
- Lo que cambiás en la Lista de mods (prender mods, reordenar) se guarda en el perfil activo al instante.
- Lo que aceptás en la pantalla de Mods del juego también vuelve al perfil activo.
- Un mod que prendés pasa a ser uno de los mods del perfil. Los mods que no están en ningún perfil se ofrecen en la Lista de mods con **Agregar al perfil**.
- Las partidas nunca pasan de una build mayor a otra (41 y 42); por eso un perfil recuerda su build.
