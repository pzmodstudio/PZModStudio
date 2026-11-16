import type { VfsConflict } from '../types';

/**
 * What a publish puts in the package. Before, every conflict went in with its merged text, so a
 * file two mods change on the same lines shipped the automatic merge nobody had looked at: both
 * changes "firing at once", which is not what either author wrote. Now a conflict goes in when:
 * - the player decided it (saved in the Merger: status RESOLVED), whatever they chose;
 * - or its merge needs no decision (the mods change different lines, a list that adds up, the
 *   same file twice): `auto_mergeable`, or identical copies.
 * The rest wait for a choice and are left out: the game then does what it does without a
 * package, the last loaded mod's copy wins, and the player is told which files wait.
 */
export interface PublishPlan {
  ship: { relative_path: string; content: string }[];
  /** Conflicts left out until the player chooses. */
  waiting: VfsConflict[];
}

export const needsChoice = (c: VfsConflict) => c.status !== 'RESOLVED' && c.status !== 'AUTO_MERGED' && !c.is_identical_noise && !c.auto_mergeable;

/**
 * `refresh` (Update patch, after mods changed): a merge that needs no decision is made again
 * from the mods as they are now, not the copy kept from the last publish; a decision whose file a
 * mod changed since (`stalePaths`, from Mod Health) waits to be made again instead of shipping
 * the old one silently.
 */
/** The kept content is one mod's file or the game's own: the player chose it ("Only A", "Vanilla"). */
const isAChoice = (c: VfsConflict) => {
  const same = (a: string, b: string) => a.replace(/\r\n/g, '\n').trim() === b.replace(/\r\n/g, '\n').trim();
  const kept = c.merged_output ?? '';
  return c.status === 'RESOLVED' && (c.competing_mods.some((m) => same(m.content, kept)) || same(c.base_content, kept));
};

export const publishPlan = (conflicts: VfsConflict[], refresh?: { stalePaths: Set<string> }): PublishPlan => {
  const ship: PublishPlan['ship'] = [];
  const waiting: VfsConflict[] = [];
  for (const c of conflicts) {
    if (refresh && c.auto_mergeable && !isAChoice(c)) {
      ship.push({ relative_path: c.relative_path, content: c.auto_ast_output || c.merged_output || c.base_content });
    } else if (needsChoice(c) || (refresh && refresh.stalePaths.has(c.relative_path))) {
      waiting.push(c);
    } else {
      ship.push({ relative_path: c.relative_path, content: c.merged_output || c.base_content });
    }
  }
  return { ship, waiting };
};
