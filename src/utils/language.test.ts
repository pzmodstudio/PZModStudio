import { describe, expect, it } from 'vitest';
import { scriptLanguage } from './language';

describe('scriptLanguage', () => {
  it('goes by the script with most letters', () => {
    expect(scriptLanguage('Плавные объёмные модели для вида от первого лица, автор 中文')).toBe('ru');
    expect(scriptLanguage('这个模组添加了新物品')).toBe('zh');
    expect(scriptLanguage('このMODは新しいアイテムを追加します')).toBe('ja');
    expect(scriptLanguage('새로운 아이템을 추가합니다')).toBe('ko');
    expect(scriptLanguage('Adds new items to the game')).toBeNull();
  });
});
