# Servidores

La pantalla Servidores tiene dos lados: **Unirse a un servidor**, para encontrar un servidor multijugador y jugar ahí, y **Alojar un servidor**, para correr tu propio servidor dedicado desde esta PC. Pasá de uno a otro con los dos botones de arriba.

## Unirse a un servidor: encontrar uno

En **Servidores públicos**, apretá **Buscar servidores**. El Studio le pide a Steam todos los servidores de Project Zomboid, como el buscador del propio juego, a través del cliente de Steam abierto en esta PC (no hace falta clave; mientras pregunta, Steam te muestra jugando a Project Zomboid). Los servidores aparecen a medida que responden; todos tardan alrededor de un minuto y medio. **Detener** corta la búsqueda, **Buscar de nuevo** empieza otra vez.

Achicá la lista con el cuadro (**Nombre o mapa**) y los filtros: **Solo mi versión**, **Con jugadores**, **No llenos**, **Sin contraseña**, **Con y sin mods** / **Con mods** / **Sin mods**, y ordená por **Más jugadores**, **Menor ping** o **Nombre**. Hacé clic derecho en un servidor para ver un menú: **Ver lo que dice**, **Entrar**, **Marcar con estrella**, **Copiar la dirección** y crear un perfil con sus mods.

**Mis servidores** guarda los servidores que marcaste con estrella (**Con estrella**) y aquellos a los que entraste desde acá (**Entraste desde acá**). Para agregar un servidor privado o de un amigo, escribí su dirección en **Agregar un servidor por su dirección** (`ip:puerto`; 16261 si no ponés el puerto), un nombre opcional y la contraseña, y apretá **Agregar**. La contraseña se guarda en el almacén de credenciales de Windows, nunca en un archivo.

## Unirse a un servidor: entrar

1. Hacé clic en un servidor. El panel de la derecha muestra su versión, si está abierto a todos o solo con lista blanca, su último reinicio y sus mods. Cada mod tiene un color: **Instalado y activo**, **Instalado** o **Lo descarga el juego**.
2. Si el servidor usa otra versión que la tuya, un aviso lo dice: el juego no te deja entrar hasta que coincidan (rama beta de Steam).
3. Escribí la contraseña del servidor si tiene, y marcá **Recordar la contraseña del servidor** para guardarla. Una guardada se muestra como una línea verde (**Contraseña del servidor guardada en Windows**) en vez del campo: el Studio la pone sola al entrar. **Cambiar** abre el campo para una nueva y **Olvidar** la borra. La contraseña de tu cuenta la escribís en la ventana de conexión del juego.
4. Cerrá el juego si está abierto y apretá **Entrar**.

El juego abre en la ventana de conexión de este servidor con la contraseña del servidor ya puesta, y tu usuario también si lo escribiste en **Tu usuario en este servidor** (el Studio lo guarda; el mod Studio Link lo completa). Ahí escribís la contraseña de tu cuenta y tocás **Conectar**, que no se activa sin usuario. Un usuario nuevo crea una cuenta nueva en ese servidor. El juego descarga los mods del servidor y los usa solo ahí: tu lista de mods queda como está; el perfil con el que entrás los recibe (abajo).

Con el mod Studio Link, el servidor también queda en los **Favoritos** del juego (pestaña Favorites de su pantalla Multijugador), con tu usuario debajo, igual que si lo hubieras agregado con **Add Server** y **Add New Account**. Así, si te desconectás, lo tenés ahí para volver a entrar sin cerrar el juego. No se agrega dos veces, y la contraseña de tu cuenta no se guarda; la del servidor sí, como la guarda el juego.

## Crear un perfil para un servidor

Un servidor solo anuncia sus primeros mods (Steam corta la lista). Después de entrar una vez desde acá, el Studio conoce la lista completa y el botón pasa a ser **Crear un perfil con estos N mods**. Antes de eso, **Crear un perfil con los N mods que anuncia** usa la lista parcial. El perfil se llama "Servidor: (nombre)" y respeta el orden del servidor. Antes de crearlo te avisa qué mods no están instalados ahora.

Para jugar con él, activalo en Perfiles antes de entrar. Cada vez que entrás desde acá, cuando el juego ya cargó los mods del servidor, se agregan al perfil activo con el que entraste: activos, en el orden del servidor, con los que el juego te hizo descargar. Sus otros mods quedan en él, apagados, salvo los que corren solo de tu lado (ZombieBuddy y sus mods de Java para el cliente, como Viewpoint): siguen como los tenías. El panel del servidor dice qué perfil es.

## Alojar un servidor: la lista de servidores

A la izquierda, **Configuraciones de servidor (.ini)** lista las configuraciones de servidor de `Zomboid/Server`, con la cantidad de mods y de ítems del Workshop, y **En marcha** para el que está arriba. Escribí un nombre y apretá **Crear** para hacer una nueva, o **⚡ Crear la configuración por defecto (servertest.ini)** cuando no hay ninguna. El tacho borra una configuración del disco (primero te pregunta). Elegí una para ver sus pestañas. El cartel de arriba muestra **EN LÍNEA** o **DESCONECTADO**, **Compartir invitación** (copia un texto de invitación con el nombre, el puerto, los jugadores máximos, la contraseña y la cantidad de mods; vos completás tu IP pública y abrís el puerto en el router) y un botón para recargar los archivos de servidor.

## Host y lanzador

- **Memoria (-Xmx)**: 4, 8, 12 o 16 GB para el servidor.
- **Contraseña de admin**: fija la contraseña de la cuenta `admin` cuando arranca el servidor.
- **Autenticación y red**: **Steam activado** o **NoSteam (LAN)**.
- **Iniciar servidor dedicado (nombre)** lo arranca en su propia ventana de consola. Mientras corre, **Detener servidor dedicado** detiene el servidor que inició el Studio (nunca otros programas Java), y **Reiniciar** lo detiene y lo vuelve a arrancar.

## Mods y sincronización

Esto compara tus mods activos con lo que tiene el `.ini` del servidor. Un servidor no lee tu lista de mods: necesita tres líneas en su `.ini`, `Mods=`, `WorkshopItems=` y `Map=`, y las tres tienen que coincidir o el servidor juega en silencio sin los mods. El Studio las revisa y lista lo que está mal (un mod que nunca se descarga, un ítem que nunca se carga, un mapa ignorado).

**Escribir mi lista en este servidor (Mods, WorkshopItems y Map)** escribe las tres líneas a partir de tus mods activos, en el orden de carga, y deja todo lo demás como estaba. El archivo anterior queda en `Zomboid/PZModStudio_Backups`.

Un paquete de PZ Mod Studio (`Z_PZModStudio_*`) activo también entra en `Mods=`, pero es un mod local, no un ítem del Workshop: el servidor en esta PC lo tiene, los jugadores no lo reciben al descargar, y el juego no los deja entrar sin él. Compartiles el paquete, o apagalo antes de escribir la lista.

**Chequeo multijugador de los mods activos** (**Chequear**) lista llamadas que el código de cliente de un mod hace directamente donde el juego las pasa por el servidor; en multijugador cambian solo el mundo de ese jugador, o se rechazan.

## Jugadores

**Jugadores conectados** lista quién está en línea, leído del log del propio servidor, con rol, ping, salud y posición. Mientras el servidor está en marcha, el Studio habla con él por RCON (el `RCONPassword` y el `RCONPort` de su `.ini`; un servidor que el Studio inicia sin contraseña recibe una):

- **Expulsar** en la fila de un jugador lo desconecta; puede volver a entrar. Podés dar un motivo, que el jugador ve.
- **Banear** lo banea de este servidor, con un motivo opcional. Para quitar un baneo, usá la consola del servidor: `unbanuser "nombre"`.
- El cuadro de arriba manda un mensaje a todos en el juego (**Enviar a todos**, o Enter); aparece en su pantalla.

La respuesta del servidor aparece debajo del cuadro.

## .ini rápido

Editá la configuración principal y apretá **Guardar la configuración en el .ini**: **Nombre público del servidor**, **Contraseña del servidor (para entrar)** (vacía para un servidor abierto), **Contraseña de admin (-adminpassword)**, **Jugadores máximos**, **Puerto por defecto**, **Descripción pública**, **Combate PVP** y **Pausar cuando está vacío**.

## Discord

Un webhook de Discord deja que el servidor publique en un canal: cuando arranca, se detiene, se cae o vuelve después de reiniciarse, quién entra y quién sale, y cuando sus mods del Workshop se actualizaron y necesita reiniciarse. No menciona a nadie.

1. En Discord, abrí la configuración del canal, Integraciones, Webhooks, Nuevo webhook, y copiá su URL.
2. Pegala en **Dirección del webhook** y apretá **Guardar**. Se guarda en el Administrador de credenciales de Windows y no se vuelve a mostrar.
3. Apretá **Mandar una prueba** y mirá el canal.
4. Marcá **Publicar las novedades de este servidor en el canal** y elegí los eventos: **Arranca, se detiene y se cae**, **Jugadores que entran y salen**, **Reinicios**, **Mods del Workshop que esperan un reinicio**.

**Olvidarlo** quita la dirección de esta PC. Cada servidor tiene su propio webhook.

### Chat con el juego: el bot propio del servidor

El servidor dedicado de Project Zomboid tiene un bot de Discord propio. El Studio lo configura en el `.ini` del servidor; vive dentro del servidor, así que funciona siempre que el servidor corre, aunque el Studio esté cerrado.

1. En el Developer Portal de Discord: **New Application**, después **Bot**: **Reset Token** y copialo, y activá **Message Content Intent**.
2. En **Chat con el juego**, pegá el **Token del bot**, escribí el **Canal del chat** (su nombre como está en Discord, sin `#`) y, si querés, el **Canal del registro**, marcá **Activar el bot** y apretá **Guardar en el .ini**.
3. Apretá **Invitar el bot a tu Discord**, elegí tu servidor de Discord, e iniciá (o reiniciá) el servidor del juego.

Desde ahí el chat general del juego va a ese canal, y lo que se escribe ahí entra al juego. El juego lee el token del `.ini`, así que se escribe ahí: quien tenga ese archivo puede usar el bot. Si el `.ini` tiene un canal de comandos, la pantalla te avisa: lo que cualquiera escriba ahí corre como un comando de la consola de admin; **Borrarlo** salvo que ese canal sea solo tuyo.

Un **mensaje de estado que se edita solo** (activado de entrada) deja un solo mensaje en el canal que dice si el servidor está en línea, desde cuándo y quién está, en vez de publicar uno nuevo cada vez.

### Comandos desde Discord (con el Studio abierto)

Apagados salvo que los actives. Con el bot de arriba configurado, escribí un **Canal de comandos** y el **Rol que puede usarlos** (los nombres como están en Discord) y marcá **Activar los comandos**. El Studio lee ese canal cada 5 segundos y contesta ahí; a quien no tiene el rol se le dice que no puede.

- `!estado` y `!jugadores`: si el servidor está en línea y quién está.
- `!reiniciar [minutos]` (de 1 a 60, 5 si no decís): los jugadores reciben el aviso en el juego en ese momento, y 5 y 1 minuto antes; después el servidor se detiene como siempre (guarda) y vuelve a arrancar. `!reiniciar cancelar` lo detiene.
- `!guardar` guarda el mundo; `!mensaje <texto>` le dice algo a todos en el juego.
- `!ayuda` los lista. También funcionan en inglés (`!status`, `!players`, `!restart`, `!save`, `!msg`).

Le hablan al servidor por RCON: un servidor que creó el juego no tiene contraseña RCON, así que el Studio le pone una al azar la próxima vez que lo inicia (antes guarda una copia del `.ini`); eso también hace que **Detener** guarde antes de cerrar. Solo funcionan mientras PZ Mod Studio está abierto, y nunca corren un comando de consola escrito a mano. Usá un canal que solo vean los admins: el canal del chat también entra al juego.

## Bueno saber

- Discord solo recibe mensajes mientras PZ Mod Studio está abierto y alojando el servidor.
- Para iniciar un servidor hace falta la carpeta del juego configurada en **Configuración**.
- Entrar a un servidor abre el juego: cerralo antes.
