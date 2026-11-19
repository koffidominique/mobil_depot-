/** React Native Web avec des exports typés pour l'aperçu navigateur. */
const RN: any = require('react-native-web');
import { mockShare } from '../mock/share';

export const ActivityIndicator = RN.ActivityIndicator;
export const Animated = RN.Animated ?? { View: RN.View, Value: class { setValue() {} }, timing: () => ({ start() {} }) };
export const AccessibilityInfo = RN.AccessibilityInfo ?? {
  isReduceMotionEnabled: async () => false,
  addEventListener: () => ({ remove() {} }),
};
export const AppState = RN.AppState ?? {
  addEventListener: () => ({ remove: () => undefined }),
};
export const Pressable = RN.Pressable;
export const ScrollView = RN.ScrollView;
export const StyleSheet = RN.StyleSheet;
export const Text = RN.Text;
export const TextInput = RN.TextInput;
export const View = RN.View;
export const Image = RN.Image;
export const Platform = RN.Platform ?? { OS: 'web', select: (values: Record<string, unknown>) => values.web };
export const Dimensions = RN.Dimensions;
export const useWindowDimensions = RN.useWindowDimensions ?? (() => Dimensions.get('window'));
export const KeyboardAvoidingView = RN.KeyboardAvoidingView;
export const Modal = RN.Modal;
export const RefreshControl = RN.RefreshControl;
export const FlatList = RN.FlatList;
export const Linking = RN.Linking;
export const useColorScheme = RN.useColorScheme ?? (() => 'light');

export type AppStateStatus = 'active' | 'background' | 'inactive';
export type StyleProp<T> = T | false | null | undefined | StyleProp<T>[];
export type ViewStyle = Record<string, unknown>;
export type TextStyle = Record<string, unknown>;
export type TurboModule = Record<string, unknown>;
export const TurboModuleRegistry = {
  get: (_name: string) => null,
  getEnforcing: <T = any>(_name: string): T => ({} as T),
};

export const Share = {
  share: async (options: { title?: string; message?: string; url?: string }) => mockShare(options),
};
