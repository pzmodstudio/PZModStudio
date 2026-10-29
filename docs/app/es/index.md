# PZ Mod Studio

PZ Mod Studio es un administrador de mods gratuito para Project Zomboid en Windows. Guarda tus mods en perfiles, hace copias de tus partidas, revisa tu lista de mods como la lee el juego y te ayuda a arreglar lo que se rompe. Tocá "?" en cualquier pantalla para ver su página de ayuda.

La primera vez que abrís el Studio aparece una **Bienvenida**: una tarjeta abajo a la derecha que recorre cada pantalla con la pantalla real de fondo y su botón resaltado en la barra lateral, con lo que el Studio ya sabe de tu instalación (las carpetas que encontró, tus perfiles, los problemas de tu lista). Avanzá con **Siguiente** o las flechas del teclado; la X la cierra. Para verla de nuevo, abrí la ayuda ("?") y tocá **Ver la bienvenida otra vez**.

## Las pantallas

La barra lateral lista las pantallas bajo **Módulos del Studio**. Las pantallas con candado necesitan antes un perfil activo: creá o activá uno en Perfiles.

- **Perfiles**: listas de mods con nombre; crealas, cambiá entre ellas con un clic, compartilas como archivos `.pzpack`.
- **Partidas**: tus partidas y sus copias de seguridad; hacé una, guardá una o restaurá una.
- **Centro de monitoreo**: iniciá el juego y leé su registro, con una tarjeta por error y el mod del que vino.
- **Lista de mods**: prendé y apagá mods y fijá el orden de carga, con filtros, grupos, notas y deshacer.
- **Novedades**: tus mods del Workshop actualizados desde que jugaste, con sus notas de cambios y un aviso cuando una actualización puede romper partidas.
- **Salud de mods**: revisa la lista activa como lo hace el juego: requisitos faltantes o apagados, orden incorrecto, mods incompatibles, mods que tu build no carga, con arreglos.
- **Explorar mods**: buscá en el Steam Workshop, suscribite, armá tus propias listas e instalá mods bajados de otros lados.
- **Mapas**: las celdas de cada mod de mapa sobre el mundo del juego, y dónde dos mods de mapa se pelean por las mismas celdas.
- **Fusionador de mods**: cuando dos mods cambian el mismo archivo, combinalos en un paquete de parche que carga último.
- **Desarrollador**: para quienes hacen mods: revisá los archivos Lua y de scripts de un mod antes que el juego, e iniciá el juego con solo ese mod prendido. Aparece solo con **Funciones para desarrolladores** prendido en Configuración.
- **Servidores**: alojá y administrá un servidor dedicado y mantené sus mods iguales a los tuyos.
- **Configuración**: dónde están tu juego y tus mods, qué avisos ves, y otras opciones.

## Iniciar el juego

El botón verde de arriba a la derecha inicia Project Zomboid con tu lista de mods. La flecha de al lado abre el menú:

- **Modos de inicio**: con depuración (`-debug`, el Centro de monitoreo sigue el registro en vivo) o el juego normal, cada uno en pantalla completa o en ventana.
- **Adónde**: **Menú principal**, como siempre, o **Personalizado…**, que abre **Inicio personalizado** para ir directo a un lugar:
  - **Una partida**: tus partidas para un jugador, las del perfil activo primero. El juego abre y, en el menú principal, el mod Studio Link carga esa partida, con los mismos avisos y copia de seguridad que **Continuar**. Después queda la pantalla negra de "clic para empezar" del juego.
  - **Un servidor**: tus servidores guardados. Entra como **Entrar** en Servidores, con la contraseña guardada y tu usuario; el juego carga los mods del servidor.
  - **Cómo** elige el modo de inicio. **Elegir** lo deja como destino del botón; **Elegir e iniciar** además arranca el juego. **Volver al menú principal** quita el destino.

Con un destino elegido, el botón lo nombra (**Jugar: (partida)** o **Entrar: (servidor)**) y lo recuerda, igual que el modo, mientras **Recordar el inicio del juego** esté prendido en Configuración.

## El mod Studio Link en el juego

PZ Mod Studio Link (`Z_StudioLink`) es un mod chico para la Build 42 que el Studio instala y deja prendido en todos los perfiles. Es la parte de la app dentro del juego. Una parte funciona sola; el resto necesita la app.

### Su propia pantalla de Mods

El botón **MODS** del menú principal abre la pantalla de Mods del Link en vez de la del juego. Por sí sola ofrece:

- todos los mods instalados con un buscador (nombre, ID del mod, ID del Workshop, autor) y filtros (**Todos**, **Activos**, **Desactivados**, **Favoritos**, **Actualizados**, **Problemas**, **Mapas**);
- los datos de cada mod: qué requiere y qué lo necesita, sus builds, su página del Workshop;
- prender y apagar mods junto con lo que requieren, varios a la vez con Ctrl o Shift;
- el orden de carga: ordenado por él, subiendo y bajando, o arrastrando, nunca por encima de lo que un mod requiere;
- grupos por autor o categoría, las opciones de cada mod (**Opciones del mod**), el **Orden de mapas**, y **Deshacer** / **Rehacer**;
- **Aceptar** guarda en la lista que lee el juego, igual que la pantalla original. **Pantalla del juego** abre la original.

Con la app, la misma pantalla además muestra solo los mods del perfil (**Mods del perfil** / **Todos los instalados**), los avisos de Salud de mods en cada mod, los mods actualizados desde tu última sesión, tus notas y grupos, y un botón para agregar al perfil los mods nuevos que no están en ninguno. **Sincronizar con PZ Mod Studio** trae lo que cambió la app, y **Abrir PZ Mod Studio** pone la app al frente.

Si está prendido otro administrador de mods que maneja la pantalla de Mods, el Link te pregunta cuál pantalla querés, y si elegís la otra se la deja.

### La tarjeta del menú principal

Si la app ya se ejecutó en esta PC, una tarjeta en el menú principal muestra si la app está abierta o cerrada, el perfil activo y su cantidad de mods, los errores y avisos de Salud de mods, y **Continuar: (partida)** con lo que cambió en sus mods desde su última sesión. Si la app nunca se instaló, la tarjeta en cambio invita a conseguirla, con **Descargar** (copia la dirección de la página de descarga) y **Ocultar**.

### El aviso de errores

Cuando el juego registra un error de Lua, un aviso nombra el mod del que vino (y qué mod lo llamó), con el último mensaje. **Copiar reporte** pone el error en el portapapeles para mandárselo al autor del mod, **Ver en Mods** abre la pantalla de Mods en ese mod, y **Ver en el Studio** abre el Centro de monitoreo en su tarjeta. El aviso no aparece si Error Magnifier está prendido.

### Con y sin la app

- Sin la app: la pantalla de Mods del Link, el aviso de errores (sin **Ver en el Studio**) y la tarjeta de invitación.
- Con un perfil activo, aun con la app cerrada: la pantalla de Mods muestra el perfil como lo dejó la app; en la pantalla de carga cada partida dice si sus mods coinciden con el perfil; una partida que usa mods fuera del perfil no carga; una partida con la lista de mods vacía pregunta antes.
- Solo con la app abierta: las copias de una partida antes de cargarla con mods cambiados y al salir de ella, jugar una partida en una copia, agregar mods nuevos al perfil, **Sincronizar**, **Abrir PZ Mod Studio** y **Ver en el Studio**.
- Las notas, los grupos, los perfiles y Salud de mods se hacen en la app; el juego solo los muestra.
