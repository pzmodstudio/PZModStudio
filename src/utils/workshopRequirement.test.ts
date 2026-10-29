import { describe, expect, it, vi } from 'vitest';
import { findRequiredItem } from './workshopRequirement';
import { TauriService } from '../services/tauri';
import { ModInfo, WorkshopItemDetails } from '../types';

const mod = (mod_id: string, workshop_id?: string): ModInfo => ({ mod_id, name: mod_id, dependencies: [], enabled: true, workshop_id, load_order_index: 0 });
const details = (required: [string, string][], description = ''): WorkshopItemDetails =>
  ({ description, required_items: required.map(([workshop_id, title]) => ({ workshop_id, title })) }) as unknown as WorkshopItemDetails;

describe('the Workshop item a missing requirement comes from', () => {
  // As on Basile's 3D Doors page: ZombieBuddy and Viewpoint installed, Visible Door Damage not.
  const mods = [mod('BasileDoors3D', '3818859807'), mod('ZombieBuddy', '3619862853'), mod('Viewpoint', '3809306528')];

  it('is the one required item not installed', async () => {
    vi.spyOn(TauriService, 'workshopItemDetails').mockResolvedValue(
      details([['3619862853', 'ZombieBuddy'], ['3809306528', 'Project Viewpoint'], ['3799596085', 'Visible Door Damage [B42 MP]']])
    );
    expect(await findRequiredItem('BasileDoors3D', 'VisibleDoorDamagebyYurik', mods)).toBe('3799596085');
  });

  it('with several left, the one whose description names the mod id', async () => {
    vi.spyOn(TauriService, 'workshopItemDetails').mockImplementation(async (id: string) => {
      if (id === '3818859807') return details([['1', 'Other'], ['2', 'Visible Door Damage']]);
      return details([], id === '2' ? 'Mod ID: VisibleDoorDamagebyYurik' : 'Mod ID: SomethingElse');
    });
    expect(await findRequiredItem('BasileDoors3D', 'VisibleDoorDamagebyYurik', mods)).toBe('2');
  });

  it('is not guessed when nothing names it, or the mod has no Workshop page', async () => {
    vi.spyOn(TauriService, 'workshopItemDetails').mockImplementation(async (id: string) =>
      id === '3818859807' ? details([['1', 'A'], ['2', 'B']]) : details([], 'nothing here')
    );
    expect(await findRequiredItem('BasileDoors3D', 'VisibleDoorDamagebyYurik', mods)).toBeNull();
    expect(await findRequiredItem('LocalMod', 'X', [...mods, mod('LocalMod')])).toBeNull();
  });
});
