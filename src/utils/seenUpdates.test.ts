import { describe, expect, it } from 'vitest';
import { updateKey } from './seenUpdates';

describe('an update marked as seen', () => {
  it('is that update only: the next one of the same mod, or one waiting in Steam, is new again', () => {
    const installed = { workshop_id: '1', installed_time: 100, latest_time: 100, pending: false };
    expect(updateKey(installed)).toBe(updateKey({ ...installed }));
    expect(updateKey(installed)).not.toBe(updateKey({ ...installed, installed_time: 200, latest_time: 200 }));
    expect(updateKey(installed)).not.toBe(updateKey({ ...installed, latest_time: 300, pending: true }));
  });
});
