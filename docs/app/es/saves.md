# Partidas

Tus partidas de Project Zomboid y sus copias de seguridad. El Studio hace copias solo en los momentos en que una partida corre más riesgo; acá ves esas copias, hacés una vos, guardás una para siempre o volvés a una anterior.

## Las partidas de cada perfil

Cada perfil tiene sus propias partidas. Una partida pertenece al perfil que estaba activo la última vez que la jugaste con el Studio abierto; una partida jugada sin él pertenece a un perfil solo si los mods de ese perfil son exactamente los de la partida. Una partida nunca se pasa a otro perfil.

- Esta pantalla muestra las partidas del perfil activo. **Ver todas** agrega las de otros perfiles y las que no tienen perfil, cada una con la etiqueta **Perfil: <nombre>** o **Sin perfil**.
- La partida de un servidor se muestra con su dirección (`ip:puerto`) y la etiqueta **Online**, y pertenece al perfil con el que entraste a ese servidor desde **Servidores** (el Link no corre en un servidor, así que el Studio se guía por su propia lista de servidores).
- En el juego, con el mod Studio Link: la pantalla **Cargar** lista las partidas del perfil activo, con **Ver todas** al lado de la búsqueda, y **Continuar** carga la partida más reciente del perfil activo (no aparece si el perfil no tiene ninguna). Sin el Studio, el juego lista todas las partidas como siempre.

## Qué ves

- Arriba, **Abrir carpeta de copias** abre la carpeta donde van las copias (`Zomboid/PZModStudio_SaveBackups`), y la flecha circular actualiza la pantalla.
- Una barra con el espacio del disco: cuánto queda libre, el tamaño del disco y cuánto ocupan las copias. Se pone ámbar con **Queda poco espacio** cuando tu partida más grande puede no entrar dos veces más.
- Una fila por partida: el nombre del mundo, y abajo el modo de juego, el tamaño, la cantidad de archivos, la cantidad de mods y el último cambio.
- Si los mods de la partida cambiaron desde su última sesión, una línea ámbar lo dice: **Desde su última sesión (...)**, con cuántos mods se agregaron, se sacaron, se actualizaron, o si cambió el orden. Pasá el mouse por encima para ver los nombres.
- Si la lista de mods de la partida nombra mods que no están instalados, una línea roja lo dice: **No carga: N mods de su lista no están instalados**, con un botón por mod. Tocalo para abrirlo en **Explorar mods** (el ítem del Workshop si la lista lo nombra, si no, una búsqueda con su id). Las partidas de servidor no tienen lista propia: el juego baja los mods del servidor al entrar.
- A la derecha de cada fila, su estado:
  - **Copia al día**: la copia más nueva coincide con la partida.
  - **Cambió desde la copia del ...**: la partida cambió después de su última copia.
  - **Sin copia**: todavía no tiene ninguna.
  - **Haciendo copia...** mientras se hace una.

## Cuándo hace copias el Studio

- **Al salir de ella**: cuando volvés al menú principal, o el juego se cierra con la partida abierta.
- **Antes de cargarla con mods cambiados**: el mod Studio Link la pide cuando cargás una partida cuyos mods no son los de su última sesión, o se actualizaron desde entonces.
- **Antes de cambiar su lista de mods**: cuando la lista de mods de la partida está por cambiarse en el juego.
- **Antes de una restauración**: la partida como estaba, justo antes de restaurar una copia vieja.

Si una partida no cambió desde su copia más nueva, no se vuelve a comprimir.

## Hacer una copia vos

Tocá **Hacer copia ahora** en la fila. La copia es un zip que se vuelve a leer después de escribirlo; las copias comprobadas muestran **verificada**. Si la partida no cambió, no se hace un zip nuevo: la copia más nueva queda marcada como guardada. El botón aparece gris cuando el disco no tiene lugar para la partida más un margen.

## Las copias de una partida

Hacé clic en la flecha a la izquierda de una fila para abrir sus copias. Cada una muestra cuándo se hizo, por qué (**hecha por vos**, **al salir de ella**, ...), el tamaño del zip, el de la partida, archivos y mods. Para cada copia:

- El alfiler la guarda: una copia guardada nunca se borra para hacer lugar. Tocalo de nuevo para que vuelva a ser automática.
- **Restaurar** vuelve la partida a esa copia, después de confirmar.
- La papelera (**Borrar esta copia**) la borra, después de confirmar. No se puede deshacer.

## Restaurar una partida

1. Cerrá Project Zomboid. Con el juego abierto, la restauración se rechaza.
2. Abrí las copias de la partida y tocá **Restaurar** en la que quieras.
3. Confirmá. El Studio revisa la copia, hace una copia de la partida como está ahora (para que puedas deshacer), descomprime la copia al lado de la partida y recién entonces las intercambia.

## Bueno saber

- Se guardan las 5 copias automáticas más nuevas de cada partida; las más viejas se borran cuando se hace una nueva. Las que hacés o fijás quedan hasta que las borres.
- Las copias automáticas necesitan PZ Mod Studio abierto mientras jugás, y el mod Studio Link prendido en el juego (es el que le avisa a la app cuando entrás y salís de una partida).
- La partida de un servidor se copia cuando el juego se cierra con el Studio abierto, si cambió durante esa sesión.
- Con la app cerrada, el mod Link avisa antes de cargar que la partida no se puede copiar, y te deja seguir sin copia.
- El mod Link también puede jugar una partida en una copia (`<mundo>_copy`), que hace la app; la original queda como está.
