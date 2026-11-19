import React from 'react';
import { View } from 'react-native-web';

export function SafeAreaProvider({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}

export function SafeAreaView({ children, style }: { children?: React.ReactNode; style?: unknown }) {
  return <View style={style}>{children}</View>;
}

export function useSafeAreaInsets() {
  return { top: 0, right: 0, bottom: 0, left: 0 };
}
