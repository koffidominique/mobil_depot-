/** PARTIE 10 — responsive, contrastes et cibles tactiles. */
import { contrastRatio, layoutForWidth, meetsAa } from '../src/core/design';
import { darkColors, lightColors } from '../src/theme';

test('la mise en page s adapte aux petits écrans et à l orientation', () => {
  expect(layoutForWidth(320, 700).columns).toBe(1);
  expect(layoutForWidth(320, 700).horizontalPadding).toBe(12);
  expect(layoutForWidth(390, 844).columns).toBe(2);
  expect(layoutForWidth(768, 1024).columns).toBe(3);
  expect(layoutForWidth(812, 375).columns).toBe(3);
  expect(layoutForWidth(667, 375).columns).toBe(1);
  expect(layoutForWidth(320, 700).minTouchTarget).toBe(44);
});

test('les palettes clair et sombre respectent AA', () => {
  for (const colors of [lightColors, darkColors]) {
    expect(meetsAa(colors.text, colors.background)).toBe(true);
    expect(meetsAa(colors.subtitle, colors.background)).toBe(true);
    expect(meetsAa(colors.text, colors.surface)).toBe(true);
    expect(meetsAa(colors.primaryText, colors.primary)).toBe(true);
    expect(meetsAa(colors.danger, colors.surface)).toBe(true);
    expect(contrastRatio(colors.text, colors.background)).toBeGreaterThan(4.5);
  }
});
