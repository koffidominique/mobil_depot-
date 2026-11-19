/** Adaptations aux petits/grands écrans et à l'orientation. */
import { useWindowDimensions } from 'react-native';
import { layoutForWidth } from '../core/design';

export type ResponsiveMetrics = ReturnType<typeof layoutForWidth>;

export function useResponsive(): ResponsiveMetrics {
  const { width, height } = useWindowDimensions();
  return layoutForWidth(width, height);
}
