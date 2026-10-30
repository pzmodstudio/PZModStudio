// The twelve screens of the app, as cards on the home page. A card opens its module's page in place
// (index.html#module-<id>), so the address can be shared and Back closes it. The texts follow the
// app's own help pages (docs/app/en/*.md): keep them in step when a screen changes.
const MODULES = [
  {
    id: 'mod-list', name: 'Mod List', color: '#10b981',
    tagline: 'Your mods on and off, in the order the game loads them.',
    intro: 'Every installed mod, which ones are on, and the load order. Every change is saved at once into the active profile and into <code>Zomboid/mods/default.txt</code>, the list the game reads.',
    sections: [
      ['Load order, sorted like the game', [
        '<b>Auto-Sort</b> puts each mod after what it requires and its <code>loadModAfter</code> mods, before its <code>loadModBefore</code> mods, and after the mods your own rules name. Everything else keeps your order.',
        'Drag a row, use the arrows, or type a position. The <b>#</b> column is the same number the game shows.',
        '<b>Undo / Redo</b> (Ctrl+Z / Ctrl+Y) through every move, switch and sort, per profile, even after closing the app.'
      ]],
      ['Switching mods on and off', [
        'Switching on a mod that needs others lists the whole chain: turn it all on, or only that one.',
        'Ctrl+click and Shift+click pick several mods; right-click them to switch, remove or unsubscribe together.',
        'Off mods keep their place, so switching one back on puts it where it was.'
      ]],
      ['Finding your way in a big list', [
        'Search by name, mod id, Workshop id or your own notes.',
        'Filters for on, off, favourites, updated, problems, maps, libraries and mods that cannot load on your build.',
        'Group by author, category or your own groups, each with <b>All on / All off</b>. Write a note on any mod.'
      ]],
      ['Safety nets', [
        'Problem tags on each row (a missing requirement, loads too early, errors last session) with a wrench that applies the fix.',
        '<b>Earlier lists</b>: every list the Studio wrote is kept; pick one and go back to it.',
        '<b>Keep this version</b> copies a mod as it is today, so a Workshop update that breaks your save can be undone.',
        'Your own load-order rules ("this loads after that") that Auto-Sort follows and Mod Health checks.'
      ]]
    ]
  },
  {
    id: 'health', name: 'Mod Health', color: '#f87171',
    tagline: 'Every problem in your active list, each with a button to fix it.',
    intro: 'Mod Health checks your active mods with the game\'s own rules and lists every problem, from the ones that stop the game to the ones only worth knowing. It runs again by itself whenever your list changes.',
    sections: [
      ['What it finds', [
        'Mods not installed, mods your build cannot load, a broken <code>mod.info</code>.',
        'Requirements that are missing or switched off, and mods in the wrong order.',
        'Pairs of mods that say they do not work together, and the same mod listed twice.',
        'Two mods giving the same file, item, recipe or translation: the game only uses one.',
        'Workshop updates Steam has not downloaded yet, and mods that filled the console last time you played.'
      ]],
      ['Fixing', [
        'Each row has its fix: switch a mod on or off, move it, take it out of the list, open it on the Workshop.',
        '<b>Fix N problems automatically</b> turns on what is missing and moves what loads too early. It never switches anything off, and the Mod List can undo it.',
        '<b>Merge N files automatically</b> settles every file conflict without overlapping changes, in a patch package that loads last.',
        '<b>Make compatible copy</b> adapts a Workshop mod to your build in a local copy, and says what it changed.',
        '<b>Copy for an assistant</b> on every row, to ask an AI what it means in plain words.'
      ]],
      ['Performance', [
        'How much memory the game may use, with a suggestion for your mod list and a warning when a recent session ran out. Each profile keeps its own value.',
        'Game time per mod: a small profiler times every event handler of every mod while you play and adds it up per mod, with the costliest handlers and their file and line.',
        'How long each stage of the game\'s start took, compared with earlier starts, and the frame times and hitches of the session.'
      ]],
      ['Security', [
        'What mods could do beyond modding, as hints and never a verdict: programs shipped, Java that starts processes, Lua that runs commands, reaches the network or reads outside its folder.'
      ]],
      ['Disk', [
        'How much each mod takes, and which ones nothing uses: no profile, no save, no server.',
        'Unsubscribe from Workshop items or send local folders to the Recycle Bin, after asking. Leftover folders Steam forgot are found too.'
      ]],
      ['Community knowledge', [
        'Once a day the Studio downloads shared load-order rules, incompatible pairs and known issues with their fix. They show up in the check alongside your own.'
      ]]
    ]
  },
  {
    id: 'profiles', name: 'Profiles', color: '#a78bfa',
    tagline: 'Named mod sets for solo, co-op and servers, switched in one click.',
    intro: 'A profile is a named mod list: which mods belong to it, which of them are on, and their order. Keep one for each way you play and switch between them in one click.',
    sections: [
      ['Making one', [
        'Start from what is on now, everything installed, a hand-picked selection, or nothing at all.',
        'Give it a name, a description and, if you like, the build it is for (41 or 42). The Studio asks before launching it on another build.'
      ]],
      ['Switching', [
        '<b>Activate</b> writes the profile\'s mods into the list the game reads and opens the Mod List with its order.',
        'Each profile keeps its own memory setting, groups, undo history and saves.',
        'With the Studio Link mod, the game\'s own Mods screen shows the active profile, and changes made there come back into it.'
      ]],
      ['Sharing', [
        'Save a profile as a <code>.pzpack</code> file: the mods that are on, in order, with their Workshop ids. Whoever opens it can subscribe to what they are missing.',
        'A deleted profile is moved to a backups folder, not lost. Deleting one can also unsubscribe from the Workshop items no other profile uses, which you tick.'
      ]]
    ]
  },
  {
    id: 'browse', name: 'Browse Mods', color: '#38bdf8',
    tagline: 'Find and install mods from the Workshop, Nexus or a file.',
    intro: 'Search the Steam Workshop and Nexus Mods, look through what you already have, keep your own lists, and install mods you downloaded yourself.',
    sections: [
      ['Steam Workshop', [
        'Search and sort by popularity, rating or date, filtered by build and category. Paste links, ids or a collection to see exactly those items.',
        'Subscribe and unsubscribe from the Studio (through the Steam client).',
        'When a mod needs other Workshop items, the whole chain is listed and you subscribe to all of it at once.',
        'Collections: subscribe to the missing ones or make a profile from it.'
      ]],
      ['Nexus Mods and downloaded files', [
        'Search Nexus by name, category and popularity; downloads happen on the Nexus site.',
        'Install a <code>.zip</code> or <code>.7z</code>, a folder, or a direct link. The Studio finds the mod inside and puts it in <code>Zomboid/mods</code>, keeping the old copy if it replaces one.'
      ]],
      ['Your mods and your lists', [
        'Everything you have installed, with chips for <b>New since last play</b> and <b>Update waiting in Steam</b>.',
        'Your own lists (Favorites is always there): subscribe to all the missing mods of a list, or make a profile from it.',
        'Lists other players shared, each read by a person before it was published. Copy one to your lists, or share yours.'
      ]],
      ['The detail view', [
        'Pictures and videos, the description (translated into your language on request), requirements marked installed or missing, and what the community index knows: builds it runs on, mods it does not go with, known issues.',
        '<b>Support the author</b>: the Ko-fi, Patreon and similar links the author published.'
      ]]
    ]
  },
  {
    id: 'merger', name: 'Mod Merger', color: '#f59e0b',
    tagline: 'Settle files two mods both change, without touching either mod.',
    intro: 'When two mods change the same file, the game uses only one. The Merger lets you decide what the game reads, and puts your decision in a small package of its own that loads last. Your mods stay as they are.',
    sections: [
      ['Merging', [
        'Lists every file your active mods collide on: Lua, item and recipe definitions, translations. <b>No Duplicates</b> hides identical copies.',
        'For each file, see the game\'s own version, each mod\'s copy, and what each mod changes; real conflicts are marked.',
        'Choose <b>Keep both</b>, <b>Merge them</b>, one mod\'s version, or the game\'s own file, or edit the result yourself.',
        'Lua is merged on a real syntax tree, not line by line.',
        '<b>Merge all and publish</b> does every file at once.'
      ]],
      ['Patching one mod', [
        'When one mod has a bug and there is nothing to merge, open its file, fix it, and save. The game reads your copy for that file only.',
        'Mod Health and the Monitor can send you straight to the file an error named, sometimes with the fix already applied.'
      ]],
      ['Packages', [
        'A package is a mod named <code>Z_PZModStudio_…</code>, put last in the load order so its copy wins.',
        'Each package belongs to the profiles that use it, and remembers the list it was built for.',
        'Share one as a <code>.pzmerge</code> file; importing it lists the mods you are missing.',
        'Every package carries a Build 42 compatibility shim that patches calls older mods make to things Build 42 renamed or removed.'
      ]]
    ]
  },
  {
    id: 'monitor', name: 'Monitor Center', color: '#f87171',
    tagline: 'The game\'s log, live, with every error turned into a card.',
    intro: 'The Monitor follows <code>console.txt</code> as the game writes it, whether you started the game from the Studio or from Steam, and turns each error into a card that names the mod the game blamed.',
    sections: [
      ['The log', [
        'Live tabs for the game and for a dedicated server you run, plus any older log you open.',
        'Search, and filters for errors, Lua and Java lines. A spam filter hides lines about missing translations.'
      ]],
      ['Error cards', [
        'One card per problem, with how many times the game repeated it, so you fix the noisy ones first, and whether it is new or was seen in earlier sessions.',
        'The mod the game named, and the mods whose code called it, read from the stack.',
        'Fixes from the card: correct an old <code>require</code> path, patch the failing file, or switch the mod off.',
        '<b>Save report</b> writes a Markdown file with the error, your build, the mod, its code and the lines around it, ready for a mod author or an AI assistant.'
      ]],
      ['Find the mod that does it', [
        'For problems the log does not name: a freeze, a missing recipe, a save that will not open.',
        'The Studio halves your list each round (mods others need stay on), you launch, play and answer whether it still happens. About 8 launches for a hundred mods.',
        'At the end it names the mod and puts your original list back, whichever way the hunt ends.'
      ]]
    ]
  },
  {
    id: 'saves', name: 'Saves', color: '#10b981',
    tagline: 'Backups made on their own at the moments a save is most at risk.',
    intro: 'Your saves and their backups. The Studio backs a save up by itself when it is most at risk; here you see those backups, make one yourself, keep one for good or put one back.',
    sections: [
      ['Automatic backups', [
        'When you leave a save, before it loads with changed or updated mods, before its mod list is changed, and before a restore.',
        'A save that has not changed is not zipped again. The newest five automatic backups are kept; pinned ones stay for good.',
        'Every backup is read back after it is written.'
      ]],
      ['Each save at a glance', [
        'Size, mods, when it last changed, and whether its newest backup matches it.',
        'What changed in its mods since its last session: added, taken out, updated, reordered.',
        'Mods its list needs that are not installed, each with a button to find it.',
        'Map mods it does not load (a save keeps the maps it was made with), with <b>Add the maps to the save</b>.',
        'Each profile has its own saves; servers you joined show by address.'
      ]],
      ['What is inside', [
        'Each character with name, position, alive or dead and what they carry; each vehicle with how far it is and what is in it.'
      ]],
      ['Restoring', [
        'Pick a backup and restore it. The Studio checks it, backs up the save as it is now so you can undo, and only then swaps them.',
        'A bar shows the room left on the disk and warns before it runs short.'
      ]]
    ]
  },
  {
    id: 'whats-new', name: 'What\'s New', color: '#38bdf8',
    tagline: 'Which of your mods changed since you last played, before you load.',
    intro: 'Your Workshop mods updated since you last played, and the ones whose update is still waiting in Steam, each with its change notes. Read it before loading a save in progress.',
    sections: [
      ['Each updated mod', [
        'Its change notes from the Workshop, newest first, and the entries of the changelog it ships in its own folder.',
        'Which of your profiles have it on, the active one first.',
        'Time spans: since you last played, the last 7 days or the last 30.'
      ]],
      ['Heading off an update', [
        'When Steam has an update it has not downloaded yet, <b>Keep the version I have</b> copies the mod as it is now.',
        'If the update breaks your save, put the kept version back from the Mod List.'
      ]],
      ['Local mods too', [
        'Mods installed by hand and switched on appear when their changelog gets a new entry.',
        'The sidebar badge counts the mods of your list that changed.'
      ]]
    ]
  },
  {
    id: 'maps', name: 'Maps', color: '#a78bfa',
    tagline: 'Every map mod on the real world map, and where two of them clash.',
    intro: 'The game\'s world and every installed map mod on one picture: which parts of Knox Country a map replaces, and where two map mods fight over the same ground.',
    sections: [
      ['The map', [
        'The real world map, with each map mod\'s cells drawn on top in its own colour. Zoom with the wheel, drag to move.',
        'Hover a cell to see its number and which maps cover it.',
        'Cells two map mods share are drawn in red.'
      ]],
      ['Clashes', [
        'A warning for each pair of map mods on that share cells: one replaces the other there. Turn one off from the warning.',
        'Two mods using the same tile definition number, which mixes up walls, floors and furniture.',
        'A map built on another one (named in its <code>map.info</code>) is not counted as a clash.'
      ]],
      ['Other map mods', [
        'Spawn points, map lists and tile packs, which add no land of their own, each with what it needs to be on.'
      ]]
    ]
  },
  {
    id: 'servers', name: 'Servers', color: '#f59e0b',
    tagline: 'Join a server with exactly its mods, or host your own.',
    intro: 'Two sides: find a multiplayer server and play on it, or run your own dedicated server from this PC.',
    sections: [
      ['Joining', [
        'Search every public server through Steam, filtered by build, players, password and mods, and star the ones you like. Add private servers by address.',
        'See a server\'s build, whitelist, last wipe and mods, each marked installed or downloaded by the game.',
        '<b>Join</b> starts the game on the server\'s connect window with the password and your username filled in. Passwords are kept in Windows\' credential store.',
        'Make a profile with exactly the server\'s mods, in its order.'
      ]],
      ['Hosting', [
        'Create server configurations, set memory, admin password and Steam or LAN, and start, stop or restart the server.',
        '<b>Mods & Sync</b> writes your active mods into the three lines of the <code>.ini</code> that must agree (<code>Mods=</code>, <code>WorkshopItems=</code>, <code>Map=</code>), keeping every other setting.',
        'A multiplayer check that lists calls a mod makes on the client which the game routes through the server.',
        'Connected players with kick, ban and messages to everyone, and the main settings without opening the file.'
      ]],
      ['Discord', [
        'A webhook that posts when the server is up, stopped or crashed, who joins and leaves, and when mods need a restart.',
        'The game\'s own chat bridge, set up for you, with a status message that keeps itself up to date.',
        'Commands from a Discord channel for a role you choose: status, players, timed restart with warnings, save, message.'
      ]]
    ]
  },
  {
    id: 'developer', name: 'Developer', color: '#38bdf8',
    tagline: 'Make a mod, check it, reload its Lua live and publish it.',
    intro: 'For people who make mods: one tab for each tool. Shown when <b>Developer features</b> is on in the settings.',
    sections: [
      ['Starting and checking a mod', [
        '<b>New mod</b> creates the folders and files Build 42 reads, with a first Lua file, ready to switch on.',
        '<b>Check every file</b> runs every Lua file through the same parser the Merger uses, and every script file through a brace check (the game silently drops a script at its first unbalanced block). Click a problem to open the line in your editor.',
        'Check the mod against the installed build: calls to functions, methods and events the game no longer has, with the nearest name it does have.',
        'Start the game with only your mod and what it needs; your list comes back when the game closes.'
      ]],
      ['Translations and the Workshop', [
        'Translation drafts: the texts your mod is missing in other languages, drafted for you to review.',
        'Publish to the Workshop from the Studio: title, visibility, tags, description and change note, a look at the files before sending, and a <code>.workshopignore</code> to keep files out.'
      ]],
      ['Reloading while the game runs', [
        'Save a Lua file and it is checked and sent into the running game within moments: on the main menu, in game or paused.',
        'A file that does not parse is not sent, so the game never runs broken code.',
        'Event handlers and global tables are kept across reloads, so open windows run the new code with their state.'
      ]],
      ['More tools', [
        'A multiplayer check of your mod\'s client code, with the file, line and the game\'s server handler for each call.',
        'Start the official map editors (TileZed, WorldEd) once the Studio finds them.',
        'A built-in Model Context Protocol server lets AI assistants such as Claude or Cursor read logs, check mods and manage the load order.'
      ]]
    ]
  },
  {
    id: 'settings', name: 'App Settings', color: '#94a3b8',
    tagline: 'Folders, language, updates and how the Studio behaves.',
    intro: 'Where the Studio finds your game and mods, and how it behaves. Every change is saved as soon as you make it.',
    sections: [
      ['Folders', [
        '<b>Auto-Detect Paths</b> finds the game, the Workshop folder and your <code>Zomboid</code> folder by itself. Steam, GOG and manual installs all work.'
      ]],
      ['Behaviour', [
        'English or Spanish, the start screen, the window as you left it, and closing to the tray so the Studio keeps watching the game.',
        'Remember how the game starts, and what the game\'s Mods screen does when another mod manager is on.'
      ]],
      ['Updates and community', [
        'Checks for new versions; <b>Update now</b> downloads, checks the signature and replaces the app, or changes nothing if anything does not match.',
        'The daily download of community rules and the mod index can be switched off. Nothing is ever sent.',
        'Optionally connect GitHub to propose rules and lists to the community in one click.'
      ]],
      ['Translation', [
        'Translate mod descriptions and change notes into your language, with your own DeepL key or a free fallback.'
      ]],
      ['Build 42 fixes and diagnostics', [
        'What the Build 42 compatibility shim covers, with a switch for each mod it patches.',
        '<b>Copy diagnostics</b> collects what a bug report needs, with your user and PC names hidden.'
      ]]
    ]
  }
];

// Screenshots for the module pages, in site/img (captured from the app, v1.7.13).
const SHOTS = {
  'mod-list': ['mod-list', 'Mod List'], health: ['health', 'Mod Health: compatible copies the Studio made'], profiles: ['profiles', 'Profiles'],
  browse: ['browse', 'Browse Mods: the Workshop inside the app'], merger: ['merger', 'Mod Merger: the packages'],
  monitor: ['monitor', 'Monitor Center: an error as a card'], saves: ['saves', 'Saves'], 'whats-new': ['whats-new', "What's new"],
  maps: ['maps', 'Maps: map mods on the map of Knox Country'], servers: ['servers', 'Servers: public servers and what a server loads'],
  developer: ['developer', 'Developer'], settings: ['settings', 'App Settings']
};

(() => {
  const grid = document.getElementById('modules');
  const view = document.getElementById('moduleView');
  const card = (m, i) => '<a class="screen" href="#module-' + m.id + '" style="--c:' + m.color + '">'
    + '<span class="num">' + String(i + 1).padStart(2, '0') + '</span><div><h3>' + m.name + '</h3><p>' + m.tagline + '</p></div>'
    + '<span class="go" aria-hidden="true">&rarr;</span></a>';
  grid.innerHTML = MODULES.map(card).join('');

  let lastScroll = 0;
  const show = () => {
    const m = MODULES.find((x) => '#module-' + x.id === location.hash);
    if (!m) {
      if (!view.hidden) { view.hidden = true; document.body.classList.remove('noscroll'); }
      return;
    }
    const i = MODULES.indexOf(m);
    const prev = MODULES[(i + MODULES.length - 1) % MODULES.length], next = MODULES[(i + 1) % MODULES.length];
    view.innerHTML = '<div class="sheet" style="--c:' + m.color + '" role="dialog" aria-modal="true" aria-labelledby="mvTitle">'
      + '<div class="sheetTop"><a href="#features" class="back">&larr; All modules</a><a href="#features" class="x" aria-label="Close">&times;</a></div>'
      + '<span class="num">Module ' + String(i + 1).padStart(2, '0') + ' of ' + MODULES.length + '</span>'
      + '<h2 id="mvTitle">' + m.name + '</h2><p class="lead">' + m.intro + '</p>'
      + (SHOTS[m.id] ? '<figure class="shot"><img src="img/' + SHOTS[m.id][0] + '.webp" alt="' + m.name + ' in PZ Mod Studio"><figcaption>' + SHOTS[m.id][1] + '</figcaption></figure>' : '')
      + m.sections.map(([h, items]) => '<h3>' + h + '</h3><ul>' + items.map((t) => '<li>' + t + '</li>').join('') + '</ul>').join('')
      + '<div class="pager"><a href="#module-' + prev.id + '">&larr; ' + prev.name + '</a><a href="#module-' + next.id + '">' + next.name + ' &rarr;</a></div></div>';
    if (view.hidden) lastScroll = window.scrollY;
    view.hidden = false;
    document.body.classList.add('noscroll');
    view.scrollTop = 0;
    view.querySelector('.back').focus({ preventScroll: true });
  };
  window.addEventListener('hashchange', () => {
    const wasOpen = !view.hidden;
    show();
    if (wasOpen && view.hidden) window.scrollTo(0, lastScroll);
  });
  view.addEventListener('click', (e) => { if (e.target === view) location.hash = 'features'; });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !view.hidden) location.hash = 'features'; });
  show();
})();
