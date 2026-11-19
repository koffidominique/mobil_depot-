import React from 'react';
import {
  ActivityIndicator,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
  type StyleProp,
  type ViewStyle,
} from 'react-native';
import { useAppStore } from '../store/useAppStore';
import { colorsFor } from '../theme';

export function useColors() {
  const mode = useAppStore((state) => state.themeMode);
  return colorsFor(mode);
}

export function Screen({
  children,
  title,
  subtitle,
  action,
}: {
  children: React.ReactNode;
  title: string;
  subtitle?: string;
  action?: React.ReactNode;
}) {
  const c = useColors();
  return (
    <ScrollView style={{ backgroundColor: c.background }} contentContainerStyle={styles.screen}>
      <View style={styles.headerRow}>
        <View style={{ flex: 1 }}>
          <Text style={[styles.title, { color: c.text }]}>{title}</Text>
          {subtitle ? <Text style={[styles.subtitle, { color: c.subtitle }]}>{subtitle}</Text> : null}
        </View>
        {action}
      </View>
      {children}
    </ScrollView>
  );
}

export function Card({
  children,
  style,
}: {
  children: React.ReactNode;
  style?: StyleProp<ViewStyle>;
}) {
  const c = useColors();
  return (
    <View style={[styles.card, { backgroundColor: c.surface, borderColor: c.border }, style]}>
      {children}
    </View>
  );
}

export function Metric({ label, value, hint }: { label: string; value: string; hint?: string }) {
  const c = useColors();
  return (
    <View style={[styles.metric, { backgroundColor: c.surface, borderColor: c.border }]}>
      <Text style={[styles.metricLabel, { color: c.subtitle }]}>{label}</Text>
      <Text style={[styles.metricValue, { color: c.primary }]}>{value}</Text>
      {hint ? <Text style={[styles.metricHint, { color: c.subtitle }]}>{hint}</Text> : null}
    </View>
  );
}

export function PrimaryButton({
  label,
  onPress,
  disabled,
  loading,
}: {
  label: string;
  onPress: () => void;
  disabled?: boolean;
  loading?: boolean;
}) {
  const c = useColors();
  return (
    <Pressable
      accessibilityRole="button"
      disabled={disabled || loading}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        { backgroundColor: disabled || loading ? c.subtitle : c.primary, opacity: pressed ? 0.85 : 1 },
      ]}
    >
      {loading ? (
        <ActivityIndicator color={c.primaryText} />
      ) : (
        <Text style={[styles.buttonText, { color: c.primaryText }]}>{label}</Text>
      )}
    </Pressable>
  );
}

export function Field({
  label,
  value,
  onChangeText,
  placeholder,
  secureTextEntry,
  keyboardType = 'default',
}: {
  label: string;
  value: string;
  onChangeText: (value: string) => void;
  placeholder?: string;
  secureTextEntry?: boolean;
  keyboardType?: 'default' | 'numeric' | 'number-pad';
}) {
  const c = useColors();
  return (
    <View style={styles.field}>
      <Text style={[styles.fieldLabel, { color: c.subtitle }]}>{label}</Text>
      <TextInput
        value={value}
        onChangeText={onChangeText}
        placeholder={placeholder}
        placeholderTextColor={c.subtitle}
        secureTextEntry={secureTextEntry}
        keyboardType={keyboardType}
        style={[styles.input, { color: c.text, backgroundColor: c.surface, borderColor: c.border }]}
      />
    </View>
  );
}

export function Banner({ message, tone = 'error' }: { message: string | null; tone?: 'error' | 'info' }) {
  const c = useColors();
  if (!message) return null;
  return (
    <View
      style={[
        styles.banner,
        { borderColor: tone === 'error' ? c.danger : c.gold },
      ]}
    >
      <Text style={{ color: tone === 'error' ? c.danger : c.text }}>{message}</Text>
    </View>
  );
}

export function Row({ label, value }: { label: string; value: string }) {
  const c = useColors();
  return (
    <View style={styles.row}>
      <Text style={{ color: c.subtitle, flex: 1 }}>{label}</Text>
      <Text style={{ color: c.text, fontWeight: '600' }}>{value}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  screen: { padding: 18, gap: 14, paddingBottom: 48 },
  headerRow: { flexDirection: 'row', alignItems: 'center', gap: 12 },
  title: { fontSize: 26, fontWeight: '800' },
  subtitle: { fontSize: 13, marginTop: 2 },
  card: { borderWidth: 1, borderRadius: 18, padding: 16, gap: 10 },
  metric: { flex: 1, minWidth: 150, borderWidth: 1, borderRadius: 16, padding: 14 },
  metricLabel: { fontSize: 12, textTransform: 'uppercase', letterSpacing: 0.6 },
  metricValue: { fontSize: 19, fontWeight: '800', marginTop: 6 },
  metricHint: { fontSize: 11, marginTop: 4 },
  button: { borderRadius: 14, paddingVertical: 15, alignItems: 'center' },
  buttonText: { fontWeight: '700', fontSize: 15 },
  field: { gap: 6 },
  fieldLabel: { fontSize: 12, fontWeight: '600' },
  input: { borderWidth: 1, borderRadius: 12, paddingHorizontal: 14, paddingVertical: 12, fontSize: 16 },
  banner: { borderWidth: 1, borderRadius: 12, padding: 12 },
  row: { flexDirection: 'row', alignItems: 'center', gap: 8, minHeight: 30 },
});
