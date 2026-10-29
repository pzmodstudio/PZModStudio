"""pzctl — drive a running Project Zomboid from outside, without taking the user's screen.

  python -I pzctl.py lua "return CA.state()"      run Lua in the game (menus or in game), print result
  python -I pzctl.py luaf file.lua                  same, code from a file
  python -I pzctl.py ui [filter]                    list visible UI elements
  python -I pzctl.py click "Text"                   click a button / menu entry by its text
  python -I pzctl.py shot out.png [maxw]            screenshot of the game window (works when covered)
  python -I pzctl.py win park|show|min|info         park = small window behind everything, no focus
  python -I pzctl.py log [n]                        tail of claude_log.txt

Talks to the Z_ClaudeAgent mod through <user>/Zomboid/Lua/claude_cmd.lua and claude_out.txt.
"""
import ctypes
import ctypes.wintypes as wt
import os
import re
import sys
import time

ZOMBOID = os.path.join(os.path.expanduser("~"), "Zomboid")
LUA_DIR = os.path.join(ZOMBOID, "Lua")
CMD = os.path.join(LUA_DIR, "claude_cmd.lua")
OUT = os.path.join(LUA_DIR, "claude_out.txt")
LOG = os.path.join(LUA_DIR, "claude_log.txt")

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32")


# ------------------------------------------------------------------ lua bridge
def lua(code, timeout=15.0):
    os.makedirs(LUA_DIR, exist_ok=True)
    cid = str(int(time.time() * 1000))
    with open(CMD, "w", encoding="utf-8", newline="\n") as f:
        f.write(f"--id:{cid}\n{code}\n")
    end = time.time() + timeout
    while time.time() < end:
        try:
            with open(OUT, encoding="utf-8", errors="replace") as f:
                txt = f.read()
            if txt.startswith(f"id:{cid}\n"):
                _, status, *rest = txt.split("\n", 2) + [""]
                return status, (rest[0] if rest else "")
        except OSError:
            pass
        time.sleep(0.1)
    return "timeout", "no answer (game not running, loading, or mod not active)"


def lua_str(s):
    return "[==[" + s + "]==]"


# ------------------------------------------------------------------ window
def find_window():
    """Main window of ProjectZomboid64.exe (largest visible top-level window of that process)."""
    import subprocess
    out = subprocess.run(["tasklist", "/FI", "IMAGENAME eq ProjectZomboid64.exe", "/FO", "CSV", "/NH"],
                         capture_output=True, text=True).stdout
    pids = set()
    for line in out.splitlines():
        parts = [p.strip('"') for p in line.split('","')]
        if len(parts) > 1 and parts[1].isdigit():
            pids.add(int(parts[1]))
    if not pids:
        return None
    found = []

    @ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def cb(hwnd, _):
        pid = wt.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        if pid.value in pids and user32.IsWindowVisible(hwnd):
            r = wt.RECT()
            user32.GetWindowRect(hwnd, ctypes.byref(r))
            found.append(((r.right - r.left) * (r.bottom - r.top), hwnd))
        return True

    user32.EnumWindows(cb, 0)
    return max(found)[1] if found else None


SWP_NOACTIVATE, SWP_NOZORDER, SWP_NOSIZE, SWP_NOMOVE = 0x10, 0x4, 0x1, 0x2
HWND_BOTTOM = 1


def win(cmd, w=1280, h=720):
    hwnd = find_window()
    if not hwnd:
        return "no game window"
    r = wt.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    if cmd == "info":
        fg = user32.GetForegroundWindow()
        return f"hwnd={hwnd} rect=({r.left},{r.top},{r.right},{r.bottom}) foreground={fg == hwnd} iconic={bool(user32.IsIconic(hwnd))}"
    if cmd == "park":
        # restore if minimized (minimized GL windows stop presenting), then send to bottom of z-order
        if user32.IsIconic(hwnd):
            user32.ShowWindow(hwnd, 4)  # SW_SHOWNOACTIVATE
        user32.SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, w, h, SWP_NOACTIVATE)
        return "parked behind other windows at 0,0 %dx%d" % (w, h)
    if cmd == "min":
        user32.ShowWindow(hwnd, 7)  # SW_SHOWMINNOACTIVE
        return "minimized"
    if cmd == "show":
        user32.ShowWindow(hwnd, 9)
        user32.SetForegroundWindow(hwnd)
        return "shown"
    return "unknown win cmd"


def key(name, hold=0.08):
    """Post a key press to the game window without focusing it (e.g. 'O', 'F9', 'SHIFT+O')."""
    hwnd = find_window()
    if not hwnd:
        return "no game window"
    vk_names = {"SHIFT": 0x10, "CTRL": 0x11, "ALT": 0x12, "ESC": 0x1B, "SPACE": 0x20, "TAB": 0x09, "ENTER": 0x0D,
                "DELETE": 0x2E, "INSERT": 0x2D, "SEMICOLON": 0xBA, "APOSTROPHE": 0xDE}
    vk_names.update({f"F{i}": 0x6F + i for i in range(1, 13)})
    keys = []
    for part in name.upper().split("+"):
        keys.append(vk_names.get(part, ord(part[0]) if len(part) == 1 else None))
    if None in keys:
        return "unknown key " + name
    WM_KEYDOWN, WM_KEYUP = 0x100, 0x101
    for vk in keys:
        sc = user32.MapVirtualKeyW(vk, 0)
        user32.PostMessageW(hwnd, WM_KEYDOWN, vk, 1 | (sc << 16))
        time.sleep(0.02)
    time.sleep(hold)
    for vk in reversed(keys):
        sc = user32.MapVirtualKeyW(vk, 0)
        user32.PostMessageW(hwnd, WM_KEYUP, vk, 1 | (sc << 16) | (1 << 30) | (1 << 31))
        time.sleep(0.02)
    return "posted " + name


def click_window(fx=0.5, fy=0.5):
    """Post a left click at a fraction of the client area, without focusing the window."""
    hwnd = find_window()
    if not hwnd:
        return "no game window"
    r = wt.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(r))
    x, y = int(r.right * fx), int(r.bottom * fy)
    lp = (y << 16) | (x & 0xFFFF)
    WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP = 0x200, 0x201, 0x202
    user32.PostMessageW(hwnd, WM_MOUSEMOVE, 0, lp)
    user32.PostMessageW(hwnd, WM_LBUTTONDOWN, 1, lp)
    time.sleep(0.08)
    user32.PostMessageW(hwnd, WM_LBUTTONUP, 0, lp)
    return f"clicked {x},{y}"


def enter_game(timeout=300):
    """After Continue/Play: wait for the world to load and get past the 'click to start'
    screen. GameLoadingState suppresses Lua, so the mod cannot answer it; a posted click
    can. Clicks are sent only while the mod is silent, so nothing in game gets clicked."""
    end = time.time() + timeout
    clicks = 0
    silent_since = None
    while time.time() < end:
        st, res = lua("return CA.state()", 2)
        if st == "ok" and '"ingame":true' in res:
            return f"in game after {clicks} clicks: {res}"
        if st == "timeout":
            silent_since = silent_since or time.time()
            # loading takes a while; start clicking once the mod has been silent for 20 s
            if time.time() - silent_since > 20:
                click_window()
                clicks += 1
        else:
            silent_since = None
        time.sleep(3)
    return f"not in game after {timeout}s ({clicks} clicks)"


def find_pz_dir():
    """The game folder: $PZ_DIR, else wherever Steam says it installed the game."""
    if os.environ.get("PZ_DIR"):
        return os.environ["PZ_DIR"]
    roots = []
    try:
        import winreg

        for hive, key, name in [
            (winreg.HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
            (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        ]:
            try:
                with winreg.OpenKey(hive, key) as k:
                    roots.append(winreg.QueryValueEx(k, name)[0])
            except OSError:
                pass
    except ImportError:
        pass
    # Other drives: Steam lists every library it uses.
    for root in list(roots):
        try:
            with open(os.path.join(root, "steamapps", "libraryfolders.vdf"), encoding="utf-8") as f:
                roots += re.findall(r'"path"\s+"([^"]+)"', f.read())
        except OSError:
            pass
    for root in roots:
        pz = os.path.join(root.replace("\\\\", "\\"), "steamapps", "common", "ProjectZomboid")
        if os.path.isdir(pz):
            return pz
    return ""


PZ_DIR = find_pz_dir()


def launch(extra=(), w=1280, h=720):
    """Start the game windowed, park its window behind everything and give focus back."""
    import subprocess
    if find_window():
        return "already running"
    prev = user32.GetForegroundWindow()
    if not PZ_DIR:
        return "Project Zomboid not found; set PZ_DIR to the game folder"
    exe = os.path.join(PZ_DIR, "ProjectZomboid64.exe")
    subprocess.Popen([exe, "-windowed", *extra], cwd=os.path.dirname(exe),
                     creationflags=0x00000008 | 0x00000200)  # DETACHED_PROCESS | NEW_PROCESS_GROUP
    hwnd = None
    for _ in range(240):
        hwnd = find_window()
        if hwnd:
            break
        time.sleep(0.25)
    if not hwnd:
        return "started, window not found yet"
    # the game grabs focus a few times while it boots; keep handing it back for a while
    for _ in range(40):
        user32.SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, w, h, SWP_NOACTIVATE)
        if prev and user32.GetForegroundWindow() == hwnd:
            # a synthetic ALT tap lifts the foreground lock so the switch is allowed
            user32.keybd_event(0x12, 0, 0, 0)
            user32.keybd_event(0x12, 0, 2, 0)
            user32.SetForegroundWindow(prev)
        time.sleep(0.5)
    return f"launched hwnd={hwnd}, parked, focus returned={user32.GetForegroundWindow() == prev}"


def shot(path, maxw=1280):
    hwnd = find_window()
    if not hwnd:
        return "no game window"
    r = wt.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(r))
    w, h = r.right, r.bottom
    if w <= 0 or h <= 0:
        return "window has no client area (minimized?)"
    hdc = user32.GetDC(hwnd)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mdc, bmp)
    ok = user32.PrintWindow(hwnd, mdc, 3)  # PW_CLIENTONLY | PW_RENDERFULLCONTENT

    class BIH(ctypes.Structure):
        _fields_ = [("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG), ("biPlanes", wt.WORD),
                    ("biBitCount", wt.WORD), ("biCompression", wt.DWORD), ("biSizeImage", wt.DWORD),
                    ("biXPelsPerMeter", wt.LONG), ("biYPelsPerMeter", wt.LONG), ("biClrUsed", wt.DWORD),
                    ("biClrImportant", wt.DWORD)]

    bih = BIH(ctypes.sizeof(BIH), w, -h, 1, 32, 0, 0, 0, 0, 0, 0)
    buf = ctypes.create_string_buffer(w * h * 4)
    gdi32.GetDIBits(mdc, bmp, 0, h, buf, ctypes.byref(bih), 0)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mdc)
    user32.ReleaseDC(hwnd, hdc)
    from PIL import Image
    img = Image.frombuffer("RGBA", (w, h), buf, "raw", "BGRA", 0, 1).convert("RGB")
    if w > maxw:
        img = img.resize((maxw, int(h * maxw / w)))
    img.save(path)
    return f"saved {path} ({w}x{h}, printwindow={ok})"


# ------------------------------------------------------------------ main
def main(argv):
    if not argv:
        print(__doc__)
        return
    c = argv[0]
    if c == "lua":
        st, res = lua(argv[1], float(argv[2]) if len(argv) > 2 else 15.0)
        print(st)
        print(res)
    elif c == "luaf":
        with open(argv[1], encoding="utf-8") as f:
            st, res = lua(f.read(), float(argv[2]) if len(argv) > 2 else 15.0)
        print(st)
        print(res)
    elif c == "ui":
        flt = lua_str(argv[1]) if len(argv) > 1 else "nil"
        st, res = lua(f"return CA.ui({flt})")
        print(st)
        print(res)
    elif c == "click":
        # screens build over a few frames: retry while the target is not there yet
        end = time.time() + (float(argv[2]) if len(argv) > 2 else 20)
        while True:
            st, res = lua(f"return CA.click({lua_str(argv[1])})")
            if "not found" not in res or time.time() > end:
                break
            time.sleep(1)
        print(st, res)
    elif c == "shot":
        print(shot(argv[1] if len(argv) > 1 else "pz.png", int(argv[2]) if len(argv) > 2 else 1280))
    elif c == "idle":
        # wait until the player's action queue is empty, then run optional Lua and print it
        end = time.time() + (float(argv[1]) if len(argv) > 1 else 60)
        while time.time() < end:
            st, res = lua("return CA.busy().actions", 5)
            if st == "ok" and res.strip() == "0":
                break
            time.sleep(0.5)
        if len(argv) > 2:
            st, res = lua(argv[2])
            print(st)
            print(res)
        else:
            print("idle" if time.time() < end else "still busy")
    elif c == "enter":
        print(enter_game(float(argv[1]) if len(argv) > 1 else 300))
    elif c == "clickwin":
        print(click_window(float(argv[1]) if len(argv) > 1 else 0.5, float(argv[2]) if len(argv) > 2 else 0.5))
    elif c == "key":
        print(key(argv[1]))
    elif c == "launch":
        print(launch(argv[1:]))
    elif c == "win":
        print(win(argv[1] if len(argv) > 1 else "info"))
    elif c == "log":
        n = int(argv[1]) if len(argv) > 1 else 40
        try:
            with open(LOG, encoding="utf-8", errors="replace") as f:
                print("".join(f.readlines()[-n:]))
        except OSError as e:
            print(e)
    else:
        print(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
