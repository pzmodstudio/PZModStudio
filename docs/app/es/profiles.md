# Perfiles

Un perfil es una lista de mods con nombre: qué mods le pertenecen, cuáles están prendidos y en qué orden cargan. Acá creás perfiles, cambiás entre ellos con un clic, los compartís y los borrás. La mayoría de las otras pantallas quedan bloqueadas hasta que haya un perfil activo.

## Qué ves

- A la izquierda, el formulario **Crear perfil nuevo**.
- A la derecha, una tarjeta por cada perfil guardado. El activo tiene un marco verde y la etiqueta **Activo**; los demás dicen **Inactivo**.
- Cada tarjeta muestra su descripción, cuántos mods están prendidos (**mods activos**), el **Total** de mods de su orden y la build: **Build 41/42** si la fijaste a mano, o **Jugado en ...** con la build en la que se jugó por última vez desde el Studio.
- El contador de arriba a la derecha dice cuántos perfiles tenés guardados.

## Crear un perfil

1. Escribí un **Nombre del perfil** (obligatorio) y, si querés, una **Descripción (opcional)**.
2. En **Qué mods**, elegí con qué arranca el perfil:
   - **Los que están activos ahora**: los mods prendidos en este momento.
   - **Todo lo instalado**: todos los mods que encontró el Studio.
   - **Elegirlos**: una lista con buscador y botones **Todos** / **Ninguno**; marcá los mods que quieras.
   - **Ninguno todavía**: un perfil vacío. El Studio te pide confirmar, porque cambiar a él apaga todos los mods. Un botón **Buscar mods en Explorar mods** te lleva ahí para suscribirte antes.
3. Tocá **Crear perfil nuevo**.

El perfil nuevo se crea y se activa en el momento, y conserva el orden de carga que ya tiene tu lista. Si el Studio todavía no encontró ningún mod (carpeta del juego o de Zomboid sin fijar en Configuración), no lo deja crear, para que un perfil no apague todos tus mods por error.

## Cambiar de perfil

Tocá **Activar (1 clic)** en una tarjeta. El Studio entonces:

- escribe los mods de ese perfil en la lista que lee el juego (`Zomboid/mods/default.txt`);
- lo marca como activo y abre la Lista de mods con su orden;
- instala el mod PZ Mod Studio Link y lo deja prendido en todos los perfiles, para que la pantalla de Mods del juego muestre el perfil.

Si un mod del perfil necesita un agente Java en el archivo de inicio del juego (Zombie Buddy, por ejemplo), primero aparece una ventana con lo que sale o vuelve a ese archivo. Tocá **Activar ...** para seguir. Si el juego está abierto, esa parte espera a que se cierre. Antes se guarda una copia del archivo de inicio.

## Editar, compartir y borrar

- El botón del lápiz (**Renombrar este perfil**) abre **Editar perfil**: cambiá el nombre, la descripción y la **Build del juego**. Dejá la build en **La última jugada** o fijá **Build 41** / **Build 42**. Con una build fijada, el Studio pregunta antes de iniciar este perfil en otra build, aun antes de haberlo jugado. Tocá **Guardar cambios**.
- El botón de compartir guarda el perfil en un archivo `.pzpack` que podés mandarle a quien quieras. Lleva solo los mods prendidos, en orden, con sus IDs del Workshop. Quien lo abra puede suscribirse a lo que le falte.
- El botón de la papelera borra el perfil después de confirmar. Si el perfil usa paquetes de parche del Fusionador (`Z_PZModStudio_...`), una segunda pregunta ofrece borrarlos también; **Cancelar** los conserva.

## Bueno saber

- Un perfil borrado no se pierde: su archivo se mueve a `Zomboid/PZModStudio_Backups/profiles`.
- Si borrás el perfil activo, no queda ningún perfil activo y la pantalla de Mods del juego deja de mostrar un perfil.
- Lo que cambiás en la Lista de mods (prender mods, reordenar) se guarda en el perfil activo al instante.
- Lo que aceptás en la pantalla de Mods del juego también vuelve al perfil activo.
- Un mod que prendés pasa a ser uno de los mods del perfil. Los mods que no están en ningún perfil se ofrecen en la Lista de mods con **Agregar al perfil**.
- Las partidas nunca pasan de una build mayor a otra (41 y 42); por eso un perfil recuerda su build.
