# Browse Mods

Browse Mods is where you find and get new mods: search the Steam Workshop and Nexus Mods, look through what you already have, keep your own lists, and install mods you downloaded yourself.

## What you see

- Four tabs at the top: **Steam Workshop**, **Nexus Mods**, **My mods** (with how many you have) and **My lists**. Each tab keeps its page and filters when you switch away.
- On the right, the install buttons: **Install .zip / .7z**, **Folder** and **From a link**.
- Each mod is a card with its picture, build chips (like **Build 42**), whether it is **Installed**, and chips such as **Update waiting** or **New since last play**. Rest the pointer on a card to see more; click it to open its detail view; right-click it for a menu.

## Steam Workshop

- Type in the search box and press **Search**. You can also paste one or more Workshop links, ids or a collection link: the Studio shows exactly those items and how many you do not have yet.
- **Sort**: **Most Popular**, **Top Rated All Time**, **Most Recent** or **Last Updated**. With Most Popular you also pick the period (**Today**, **One Week**, **Three Months**, **One Year**). A text search is always sorted by relevance.
- **Game build**: **Build 42**, **Build 41** or **Any build**.
- **Mods** or **Collections**: what to list. With Mods you can also pick a category.
- **Web** opens the Workshop in your browser.

If Steam refuses the search, a box explains why. When Steam is just busy, press **Try again**. Otherwise you can paste a personal Steam Web API key and press **Check and save**; it is kept in Windows Credential Manager on this PC only, and **Remove it** deletes it. Pasting links works without a key.

## Nexus Mods

Search by words in the mod name, sort it (**Recently updated**, **Most downloaded**, **Most endorsed**, **Newest**) and pick a category. Adult mods are hidden unless you tick **Adult content**.

Nexus downloads happen on the Nexus site. In a Nexus mod's detail view, **Download on Nexus** opens its Files tab in your browser. When the download finishes, press **Install .zip / .7z** and pick the file; the Studio tags it as a Nexus mod.

## My mods

Everything you have installed: Workshop subscriptions, Nexus mods and local mods. Filter by name or id, choose a **Source**, sort by **Recently updated** or **Name**, and use the chips **New since last play** and **Update waiting in Steam** to see only those. Right-click a Workshop mod to **Unsubscribe**.

## The detail view

Clicking a card opens a large view with pictures, the description, info and requirements. **Back** returns to the previous one, and Esc or the X closes it.

- **Subscribe** / **Unsubscribe**: done through the Steam client, so Steam must be running. Steam downloads the files; the mod appears in your Mod List when it finishes. Without Steam, the button reads **Open page to subscribe**.
- **Required items**: what the mod needs on the Workshop, marked **Installed** or **Missing**. Click one to open it.
- **Community index**: what the community index knows about the mod: builds it runs on, mods it does not go with, known issues and whether the Studio fixes them, and a newer version for your build if there is one.
- The star in the header adds it to your **Favorites**.

For a collection you get **Subscribe to the N missing**, **Make a profile from this** (a profile with the mods of the collection you have installed), and the list of what you are missing.

## Subscribing to what a mod needs

When you subscribe and the mod requires other Workshop items you do not have, a window lists the whole chain. Missing ones come ticked. Choose **Subscribe to all N** to get them together, **Only X** for just the one you asked for, or **Cancel**. Items no longer on the Workshop are marked as such.

## My lists

Your own lists of mods, kept on this PC. **Favorites** is always there.

- Type a name in **New list…** and press **+** to make one.
- Add mods with the star in a detail view, or right-click any mod and choose **Favorite**, **Add to a list…** or **Add to a profile…**.
- **Subscribe to all missing** subscribes to the list's Workshop items you do not have.
- **Make a profile** creates a profile with the list's installed mods switched on.
- The pencil renames the list, the trash deletes it. The mods stay installed.
- Right-click a card and **Take out of** the list to remove it from the list only.

## Lists from the community

**From the community** (beside **My lists**) shows lists other players shared. Each one was read by a person before it was published in the Studio's repository; the Studio fetches them once a day (the arrows fetch them now). **All lists on the web** opens the community page with every list and its mods.

- Each card says who made it, how many mods it has and how many of its Workshop mods you already have.
- **Copy to my lists** makes it one of your lists, named with its author. Nothing is installed: from there, **Subscribe to all missing** and **Make a profile** work as with any list.
- **See on GitHub** opens the file.

To share one of yours, open it in **My lists** and press **Share with the community**. You are asked your name as its author and a line about it. With a GitHub token (Settings) the Studio shows what goes out and opens the pull request; without one, GitHub's new-file page opens with the list filled in, and you press **Propose changes** there. If someone already shared a list with the same name, rename yours first.

## Installing a downloaded mod

- **Install .zip / .7z**: pick an archive. The Studio finds the mod inside and puts it in `Zomboid/mods`.
- **Folder**: for a `.rar` or anything already extracted.
- **From a link**: paste a direct `.zip` link or a GitHub page. The Studio shows what it will download and asks first.

If the mod is already installed, you are asked before replacing it; the old copy is moved to `Zomboid/PZModStudio_Backups/mods`. A green line confirms the install. Then switch the mod on in the Mod List.

## Good to know

- Subscribing and unsubscribing need the Steam client running.
- After unsubscribing, Steam deletes the files later, so the mod may stay in your Mod List until Steam restarts.
- Every web page opens in your browser.
