export const palette = {
  ink: '#0D1B2A',
  forest: '#0F5132',
  gold: '#D9A441',
  cream: '#F7F3EA',
  danger: '#B42318',
  muted: '#6B7280',
} as const;

export interface AppColors {
  background: string;
  surface: string;
  text: string;
  subtitle: string;
  border: string;
  primary: string;
  primaryText: string;
  danger: string;
  gold: string;
}

export const lightColors: AppColors = {
  background: '#F5F7F4',
  surface: '#FFFFFF',
  text: palette.ink,
  subtitle: '#56616F',
  border: '#E2E7E1',
  primary: palette.forest,
  primaryText: '#FFFFFF',
  danger: palette.danger,
  gold: palette.gold,
};

export const darkColors: AppColors = {
  background: '#0B1220',
  surface: '#162033',
  text: '#F7F3EA',
  subtitle: '#AAB4C4',
  border: '#26354D',
  primary: '#2DBE7A',
  primaryText: '#07130C',
  danger: '#FF8A80',
  gold: '#F2C66D',
};

export function colorsFor(mode: 'light' | 'dark'): AppColors {
  return mode === 'dark' ? darkColors : lightColors;
}
