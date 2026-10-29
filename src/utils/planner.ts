import { ModInfo } from '../types';
import { findInstalledDependencyInMods } from '../components/load_order/modLookup';

/** One mod the plan changes besides the one clicked, and why. */
export interface PlanStep {
  mod_id: string;
  name: string;
  /** The mod that makes this one change: the one that needs it, or the one it needs. */
  because: string;
}

/**
 * What switching one mod on or off brings with it. On: the mods it requires (all the way down)
 * that are installed and off, and requirements that load after a mod needing them, moved before
 * it. Off: the mods that are on and need it (all the way up), which would fail without it.
 */
export interface TogglePlan {
  target: ModInfo;
  enable: boolean;
  turnOn: PlanStep[];
  turnOff: PlanStep[];
  moved: PlanStep[];
  /** Required, and not installed: nothing the Studio can switch on. */
  missing: { id: string; because: string }[];
  /** Required and installed, but the installed build cannot load it. */
  blocked: PlanStep[];
}

const installed = (mods: ModInfo[]) => mods.filter((m) => !m.not_installed);

/** The mod a `require=` entry means among the installed ones. */
const resolve = (req: string, mods: ModInfo[]) => findInstalledDependencyInMods(req, installed(mods));

export function planToggle(mods: ModInfo[], modId: string): TogglePlan | null {
  const target = mods.find((m) => m.mod_id === modId);
  if (!target) return null;
  const plan: TogglePlan = { target, enable: !target.enabled, turnOn: [], turnOff: [], moved: [], missing: [], blocked: [] };

  if (plan.enable) {
    // Everything it needs, breadth first, each mod once.
    const seen = new Set<string>([target.mod_id]);
    const queue: ModInfo[] = [target];
    while (queue.length > 0) {
      const mod = queue.shift()!;
      for (const req of mod.dependencies ?? []) {
        const dep = resolve(req, mods);
        if (!dep) {
          if (!plan.missing.some((m) => m.id === req)) plan.missing.push({ id: req, because: mod.name });
          continue;
        }
        if (seen.has(dep.mod_id)) continue;
        seen.add(dep.mod_id);
        if (dep.loadable_on_installed_build === false) {
          plan.blocked.push({ mod_id: dep.mod_id, name: dep.name, because: mod.name });
          continue;
        }
        if (!dep.enabled) plan.turnOn.push({ mod_id: dep.mod_id, name: dep.name, because: mod.name });
        queue.push(dep);
      }
    }
    // Requirements load before what needs them: the ones that do not are moved.
    const order = applyPlan(mods, plan, true, false);
    const index = new Map(order.map((m, i) => [m.mod_id, i]));
    const on = new Set(order.filter((m) => m.enabled).map((m) => m.mod_id));
    for (const mod of order) {
      if (!on.has(mod.mod_id) || !(seen.has(mod.mod_id))) continue;
      for (const req of mod.dependencies ?? []) {
        const dep = resolve(req, order);
        if (dep && on.has(dep.mod_id) && (index.get(dep.mod_id) ?? 0) > (index.get(mod.mod_id) ?? 0)
          && !plan.moved.some((m) => m.mod_id === dep.mod_id) && !plan.turnOn.some((m) => m.mod_id === dep.mod_id)) {
          plan.moved.push({ mod_id: dep.mod_id, name: dep.name, because: mod.name });
        }
      }
    }
  } else {
    // Everything on that needs it, directly or through another mod being switched off.
    const off = new Set<string>([target.mod_id]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const mod of mods) {
        if (!mod.enabled || off.has(mod.mod_id)) continue;
        const need = (mod.dependencies ?? []).map((r) => resolve(r, mods)).find((d) => d && off.has(d.mod_id));
        if (need) {
          off.add(mod.mod_id);
          plan.turnOff.push({ mod_id: mod.mod_id, name: mod.name, because: need.name });
          grew = true;
        }
      }
    }
  }
  return plan;
}

/**
 * Switches these mods on with what they need, each requirement before what needs it: for mods
 * that just arrived from a subscription the player made with their requirements.
 */
export function enableWithRequirements(mods: ModInfo[], ids: string[]): ModInfo[] {
  let out = mods;
  for (const id of ids) {
    const mod = out.find((m) => m.mod_id === id);
    if (!mod || mod.enabled || mod.not_installed) continue;
    const plan = planToggle(out, id);
    if (plan) out = applyPlan(out, plan, true);
  }
  return out;
}

/** One installed mod that needs a mod about to go, and which one. */
export interface Dependent {
  mod_id: string;
  name: string;
  enabled: boolean;
  because: string;
}

/**
 * The installed mods whose `require=` names one of these mods (not counting the mods going
 * themselves), the ones that are on first: what unsubscribing or deleting them breaks.
 */
export function dependentsOf(mods: ModInfo[], leaving: string[]): Dependent[] {
  const going = new Set(leaving);
  const out: Dependent[] = [];
  for (const mod of installed(mods)) {
    if (going.has(mod.mod_id)) continue;
    const need = (mod.dependencies ?? []).map((r) => resolve(r, mods)).find((d) => d && going.has(d.mod_id));
    if (need) out.push({ mod_id: mod.mod_id, name: mod.name, enabled: mod.enabled, because: need.name });
  }
  return out.sort((a, b) => Number(b.enabled) - Number(a.enabled));
}

/** Nothing besides the mod clicked changes and nothing is in the way: no need to ask. */
export const planIsSimple = (plan: TogglePlan) =>
  plan.turnOn.length === 0 && plan.turnOff.length === 0 && plan.moved.length === 0 && plan.missing.length === 0 && plan.blocked.length === 0;

/**
 * The mod list after the plan. `withExtras` false: only the mod clicked changes (what the
 * player chose with "Only this mod"). `reorder`: requirements moved before what needs them.
 */
export function applyPlan(mods: ModInfo[], plan: TogglePlan, withExtras: boolean, reorder = true): ModInfo[] {
  const on = new Set(withExtras && plan.enable ? plan.turnOn.map((s) => s.mod_id) : []);
  const off = new Set(withExtras && !plan.enable ? plan.turnOff.map((s) => s.mod_id) : []);
  let out = mods.map((m) => {
    if (m.mod_id === plan.target.mod_id) return { ...m, enabled: plan.enable };
    if (on.has(m.mod_id)) return { ...m, enabled: true };
    if (off.has(m.mod_id)) return { ...m, enabled: false };
    return m;
  });
  if (!withExtras || !reorder || !plan.enable) return out;
  // Each requirement goes right before the first mod on that needs it, until nothing moves.
  for (let pass = 0; pass < 50; pass++) {
    let moved = false;
    for (let i = 0; i < out.length && !moved; i++) {
      const mod = out[i];
      if (!mod.enabled) continue;
      for (const req of mod.dependencies ?? []) {
        const dep = resolve(req, out);
        if (!dep || !dep.enabled) continue;
        const j = out.findIndex((m) => m.mod_id === dep.mod_id);
        if (j > i) {
          const [taken] = out.splice(j, 1);
          out.splice(i, 0, taken);
          moved = true;
          break;
        }
      }
    }
    if (!moved) break;
  }
  return out;
}
