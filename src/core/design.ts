/** Fonctions pures de design mobile : responsive et contraste WCAG. */
export function layoutForWidth(width: number, height: number) {
  const isCompact = width < 360;
  const isTablet = width >= 600;
  const isLandscape = width > height;
  return {
    width,
    height,
    isCompact,
    isTablet,
    isLandscape,
    columns: isCompact || (isLandscape && width < 700) ? 1 : isTablet ? 3 : 2,
    horizontalPadding: isCompact ? 12 : 18,
    minTouchTarget: 44,
  };
}

function channel(value: number): number {
  const normalized = value / 255;
  return normalized <= 0.03928 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
}

export function relativeLuminance(hex: string): number {
  const cleaned = hex.replace('#', '');
  if (!/^[0-9a-fA-F]{6}$/.test(cleaned)) throw new Error(`Couleur invalide : ${hex}`);
  const r = parseInt(cleaned.slice(0, 2), 16);
  const g = parseInt(cleaned.slice(2, 4), 16);
  const b = parseInt(cleaned.slice(4, 6), 16);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

export function contrastRatio(foreground: string, background: string): number {
  const first = relativeLuminance(foreground);
  const second = relativeLuminance(background);
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}

export function meetsAa(foreground: string, background: string, largeText = false): boolean {
  return contrastRatio(foreground, background) >= (largeText ? 3 : 4.5);
}
