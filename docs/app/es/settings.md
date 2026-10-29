# Configuración

En Configuración le decís al Studio dónde están tu juego y tus mods, y elegís cómo se comporta: idioma, ventana, avisos, cómo arranca y algunos extras opcionales. Cada cambio se guarda apenas lo hacés.

## Qué ves

La pantalla tiene pestañas arriba a la derecha:

- **Carpetas**: las carpetas y todas las preferencias generales (casi toda esta página).
- **Arreglos para Build 42**: qué cubre el shim de compatibilidad para Build 42, con un interruptor por cada mod que parchea.
- **Servidor MCP para agentes**: solo aparece con **Funciones para desarrolladores** encendido.

## Carpetas

Apretá **Detectar carpetas** para que el Studio encuentre todo solo. Para fijar una carpeta a mano, apretá **Elegir...** al lado. Un aviso **Guardado.** confirma el cambio.

- **Carpeta de instalación de Project Zomboid**: la carpeta con `ProjectZomboid64.exe`. Muestra **Juego encontrado** cuando la carpeta es correcta, y **El Studio no encuentra el juego acá** cuando no. Sirve para Steam, GOG o instalaciones manuales.
- **Carpeta del Steam Workshop (opcional si no usás Steam)**: donde Steam descarga los mods del Workshop (`workshop/content/108600`). Dejala vacía si jugás en GOG o ponés los mods a mano en `Zomboid/mods`.
- **Carpeta Zomboid del usuario (datos y mods locales)**: tu carpeta `C:\Users\<Usuario>\Zomboid`, con registros, partidas, perfiles y mods locales. Es obligatoria.
- **Archivo de la lista de mods (Zomboid/mods/default.txt)**: el archivo que el juego lee para saber qué mods están activos y en qué orden. Se encuentra solo; **Elegir...** acá elige un archivo, no una carpeta.

Sin una carpeta del juego válida, pantallas como el Fusionador de mods te piden que primero vengas acá.

## Avisos

En **Avisos y notificaciones** podés silenciar o volver a mostrar dos mensajes:

- **Aviso al iniciar el juego sin un paquete de fusión activo**: el aviso antes de iniciar cuando no hay ningún paquete de fusión activo.
- **Mensaje explicativo al ordenar dependencias**: la explicación que aparece al ordenar dependencias en la Lista de mods.

Cada uno tiene **Silenciar** / **Volver a mostrar**.

## Cómo arranca el Studio y su ventana

- **¿Qué se abre cuando arranca el Studio?**: **Pantalla de perfiles (inicio)** (recomendado) muestra el administrador de perfiles; **Abrir el último perfil directamente** carga el último perfil activo y abre la Lista de mods.
- **Abrir la ventana como la dejé**: mismo tamaño, lugar y monitor, maximizada o en pantalla completa si lo estaba. Si ese monitor ya no está, abre en el principal.
- **El botón de cerrar (X)**: **Cierra el Studio** lo cierra (el juego sigue si está abierto). **Lo deja en la bandeja, al lado del reloj** oculta la ventana y el Studio sigue atento al juego (copias al salir, el perfil en la pantalla de Mods del juego). Hacé clic en el ícono de la bandeja para volver a abrirlo; su menú tiene **Salir**.

## Idioma

Elegí **English** o **Español** en **Idioma**. Si a un idioma le falta una línea, se muestra en inglés. Los mensajes del propio juego (errores, avisos, las líneas del registro en el Monitor) quedan como el juego los escribe.

## Inicio del juego y la pantalla de Mods del juego

- **Recordar el inicio del juego**: encendido, el botón de inicio arranca en el modo que usaste la última vez (depuración, ventana, normal...). Apagado, siempre arranca en Modo depuración (pantalla completa), el recomendado.
- **La pantalla de Mods del juego, con otro administrador de mods activo**: solo un administrador de mods puede dibujar la pantalla de Mods del juego. Cuando hay otro activo (DMM, PZMM...), elegí **Preguntarme en el juego**, **La de PZ Mod Studio (vuelve a preguntar si activás el otro)** o **La del otro administrador**. Las copias de partidas y el aviso antes de cargar siguen siendo de PZ Mod Studio en los dos casos.

## Archivos de la comunidad y token de GitHub

- **Descargar las reglas y el índice de mods de la comunidad**: una vez al día el Studio baja de su propio repositorio de GitHub reglas de orden de carga, pares incompatibles y un registro por mod. Los usan Ordenar, Salud de mods, la Lista de mods y Explorar mods. Apagado, se conserva la última copia y no se descarga nada. Nunca se envía nada. Sus dos botones abren la página web del índice y sus listas de la comunidad.
- **Buscar versiones nuevas de PZ Mod Studio**: al abrir y cada 6 horas mientras está abierto, el Studio le pregunta a GitHub cuál es la última versión publicada. Si hay una más nueva, una barra debajo del encabezado lo avisa. **Actualizar ahora** baja la versión nueva, comprueba que la firmó el proyecto (la firma de `SHA256SUMS.txt` y el SHA-256 del zip), la pone en lugar de la que tenés y reinicia el Studio; si algo no coincide, no cambia nada. **Descargar** abre la página de la versión en el navegador, y **Ahora no** oculta el aviso hasta la próxima versión. Si el Studio está instalado en una carpeta donde no puede escribir (Archivos de programa, para todos los usuarios), actualizalo con el instalador de la página. Nada se instala sin que lo pidas, y no se envía nada. **Buscar ahora** pregunta en el momento. Apagado, no se consulta GitHub.
- **GitHub, para "Compartir" en un clic**: opcional. Conectado, los botones "Compartir con la comunidad" (reglas, el registro de un mod, una lista) abren el pull request por vos. Sin conectar, abren la página de GitHub con todo ya completo. Apretá **Conectar con GitHub**: aparece un código (ya copiado) y se abre la página de GitHub; escribí el código ahí y aprobá **PZ Mod Studio**, y el Studio queda conectado. O apretá **Crear un token**, crealo en GitHub, pegalo y apretá **Verificar y guardar**. El token se verifica con GitHub y se guarda en el Administrador de credenciales de Windows, solo en esta PC; **Quitar** lo borra.

## Diagnóstico

**Diagnóstico** junta lo que necesita un reporte de problema: versiones del Studio y de Windows, las carpetas, la versión del juego, el perfil activo, el Studio Link, Zombie Buddy, si Steam está limitando los pedidos y el final del registro del Studio. **Copiar diagnóstico** lo copia; tu nombre de usuario de Windows y el nombre de la PC se reemplazan por `<user>` y `<pc>`. Revisalo antes de pegarlo en algún lado. **Abrir carpeta de registros** abre la carpeta con los registros propios del Studio.

## Funciones para desarrolladores

**Funciones para desarrolladores** muestra la pantalla Desarrollador en la barra lateral, la pestaña **Servidor MCP para agentes** (que deja a un asistente de IA manejar el Studio) y el mod acompañante que ejecuta comandos dentro del juego. Un jugador no lo necesita; dejalo apagado salvo que estés desarrollando con eso. La elección se recuerda en esta PC.

## Arreglos para Build 42

Esta pestaña explica el shim de compatibilidad para Build 42: un archivo que el Studio escribe en cada paquete de fusión que arma, y que parchea las llamadas que hacen los mods de Build 41 a cosas que la Build 42 renombró o quitó. Las partes 1 a 6 se aplican siempre que haya un paquete de fusión activo. La parte 7 parchea mods concretos, cada uno con un interruptor **SÍ** / **NO**. Un interruptor se escribe de una vez en cada paquete del Studio en disco; no hace falta publicar, y el juego lo lee en el próximo inicio.

## Servidor MCP para agentes

Solo con **Funciones para desarrolladores** encendido. Elegí tu cliente de IA, apretá **Copiar JSON de MCP** y pegalo en el archivo de configuración de ese cliente; abajo se muestran los pasos para cada cliente. No ejecutes `pz-mcp-server.exe` con doble clic: que quede una ventana negra en blanco es normal, y tu cliente de IA lo abre y lo cierra solo.

## Bueno saber

- Las carpetas, el idioma, la ventana y los avisos se aplican en el momento; nada necesita reiniciar salvo los interruptores del shim, que el juego lee en el próximo inicio.
- El token de GitHub no sale de esta PC salvo para hablar con GitHub.
