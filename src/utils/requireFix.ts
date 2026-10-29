/** A `require` path the Studio already knows the answer to: the module moved. */
export interface RequireFix {
  from: string;
  to: string;
}

/**
 * `require("Vehicle/ISVehiclePartMenu")` -> `require("Vehicles/ISUI/ISVehiclePartMenu")`, in any
 * quoting the file uses (`require "X"` without parentheses is common). Returns the new text and
 * the first line changed, or null if the file does not contain that require.
 */
export function applyRequireFix(content: string, fix: RequireFix): { content: string; line: number } | null {
  const escaped = fix.from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`(require\\s*\\(?\\s*)(["'])${escaped}\\2`, 'g');
  if (!re.test(content)) return null;
  re.lastIndex = 0;
  let firstAt = -1;
  const out = content.replace(re, (_whole, head: string, quote: string, offset: number) => {
    if (firstAt < 0) firstAt = offset;
    return `${head}${quote}${fix.to}${quote}`;
  });
  const line = content.slice(0, firstAt).split('\n').length;
  return { content: out, line };
}

/**
 * The package a patch goes into when nobody chose one: the one the game is reading, else the
 * first that is not the companion bridge, else the default name.
 */
export function packageForPatch(
  packages: Array<{ folder_name: string; is_companion_bridge: boolean; is_active_in_modlist: boolean; is_packaged: boolean }>
): string {
  const target =
    packages.find((p) => !p.is_companion_bridge && p.is_active_in_modlist && p.is_packaged) ??
    packages.find((p) => !p.is_companion_bridge);
  return target?.folder_name ?? 'Z_PZModStudio_MergedPatch';
}
