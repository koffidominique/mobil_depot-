/** Pressable avec retour discret, désactivé si l'utilisateur préfère moins de mouvement. */
import React, { useEffect, useRef } from 'react';
import { Animated, Pressable, type StyleProp, type ViewStyle } from 'react-native';
import { useReducedMotion } from '../hooks/useAccessibility';

export function AnimatedPressable({
  children,
  onPress,
  disabled,
  accessibilityLabel,
  accessibilityRole = 'button',
  style,
  testID,
}: {
  children: React.ReactNode;
  onPress: () => void;
  disabled?: boolean;
  accessibilityLabel?: string;
  accessibilityRole?: 'button' | 'tab';
  style?: StyleProp<ViewStyle>;
  testID?: string;
}) {
  const scale = useRef(new Animated.Value(1)).current;
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    if (reducedMotion) scale.setValue(1);
  }, [reducedMotion, scale]);

  const animate = (to: number) => {
    if (reducedMotion) return;
    Animated.timing(scale, { toValue: to, duration: 90, useNativeDriver: true }).start();
  };

  return (
    <Animated.View style={[{ transform: [{ scale }] }, style]}>
      <Pressable
        accessibilityLabel={accessibilityLabel}
        accessibilityRole={accessibilityRole}
        accessibilityState={{ disabled: Boolean(disabled) }}
        disabled={disabled}
        onPress={onPress}
        testID={testID}
        onPressIn={() => animate(0.98)}
        onPressOut={() => animate(1)}
      >
        {children}
      </Pressable>
    </Animated.View>
  );
}

