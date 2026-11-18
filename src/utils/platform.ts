/**
 * The operating system the app runs on, from the web view's user agent: WebView2 on Windows says
 * "Windows NT", WebKitGTK on Linux says "X11; Linux". Used to hide what only works on Windows
 * (the update in place, downloading SteamCMD, the Windows memory log).
 */
export const isWindows: boolean = typeof navigator === 'undefined' || /Windows/i.test(navigator.userAgent);

export const isLinux: boolean = !isWindows && /Linux/i.test(typeof navigator === 'undefined' ? '' : navigator.userAgent);
