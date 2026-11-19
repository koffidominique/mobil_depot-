import React, { useEffect } from 'react';
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
import { AnimatedPressable } from './AnimatedPressable';
import { useResponsive } from '../hooks/useResponsive';
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
  const responsive = useResponsive();
  return (
    <ScrollView
      style={{ backgroundColor: c.background }}
      contentContainerStyle={[styles.screen, { padding: responsive.horizontalPadding }]}
      keyboardShouldPersistTaps="handled"
      keyboardDismissMode="on-drag"
    >
      <View style={styles.headerRow}>
        <View style={{ flex: 1 }}>
          <Text accessibilityRole="header" style={[styles.title, { color: c.text }]}>{title}</Text>
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
  const responsive = useResponsive();
  const spoken = `${label} : ${value}${hint ? `. ${hint}` : ''}`;
  return (
    <View
      accessible
      accessibilityLabel={spoken}
      style={[
        styles.metric,
        { backgroundColor: c.surface, borderColor: c.border, minWidth: responsive.isCompact ? '100%' : 150 },
      ]}
    >
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
    <AnimatedPressable
      accessibilityLabel={label}
      disabled={disabled || loading}
      onPress={onPress}
      testID={`button-${label}`}
      style={[
        styles.button,
        { backgroundColor: disabled || loading ? c.subtitle : c.primary, minHeight: 48 },
      ]}
    >
      {loading ? (
        <ActivityIndicator color={c.primaryText} accessibilityLabel={`${label} en cours`} />
      ) : (
        <Text style={[styles.buttonText, { color: c.primaryText }]}>{label}</Text>
      )}
    </AnimatedPressable>
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
        accessibilityLabel={label}
        returnKeyType="done"
        blurOnSubmit
        style={[styles.input, { color: c.text, backgroundColor: c.surface, borderColor: c.border, minHeight: 48 }]}
      />
    </View>
  );
}

/** Messages compréhensibles, immune à la seule couleur grâce au pictogramme. */
export function Banner({ message, tone = 'error' }: { message: string | null; tone?: 'error' | 'info' | 'success' }) {
  const c = useColors();
  if (!message) return null;
  const icon = tone === 'error' ? '!' : tone === 'success' ? '✓' : 'i';
  const color = tone === 'error' ? c.danger : tone === 'success' ? c.primary : c.gold;
  return (
    <View
      accessibilityRole="alert"
      accessibilityLiveRegion="polite"
      accessibilityLabel={`${tone === 'error' ? 'Erreur' : tone === 'success' ? 'Confirmation' : 'Information'} : ${message}`}
      style={[styles.banner, { borderColor: color }]}
    >
      <Text style={{ color, fontWeight: '700' }}>{`${icon}  ${message}`}</Text>
    </View>
  );
}

export function Row({ label, value }: { label: string; value: string }) {
  const c = useColors();
  return (
    <View accessible accessibilityLabel={`${label} : ${value}`} style={styles.row}>
      <Text style={{ color: c.subtitle, flex: 1 }}>{label}</Text>
      <Text style={{ color: c.text, fontWeight: '600' }}>{value}</Text>
    </View>
  );
}

export function LoadingState({ label = 'Chargement…' }: { label?: string }) {
  const c = useColors();
  return (
    <View accessible accessibilityRole="text" accessibilityLabel={label} style={styles.state}>
      <ActivityIndicator color={c.primary} />
      <Text style={{ color: c.subtitle, marginTop: 8 }}>{label}</Text>
    </View>
  );
}

export function EmptyState({ title, message }: { title: string; message: string }) {
  const c = useColors();
  return (
    <View accessible accessibilityLabel={`${title}. ${message}`} style={styles.state}>
      <Text style={{ color: c.text, fontWeight: '700', fontSize: 16 }}>{title}</Text>
      <Text style={{ color: c.subtitle, marginTop: 4, textAlign: 'center' }}>{message}</Text>
    </View>
  );
}

/** Retour visuel global après chaque action, même hors de l'écran courant. */
export function FeedbackToast() {
  const message = useAppStore((state) => state.lastMessage);
  const notify = useAppStore((state) => state.notify);
  const c = useColors();

  useEffect(() => {
    if (!message) return undefined;
    const timer = setTimeout(() => notify(null), 3200);
    return () => clearTimeout(timer);
  }, [message, notify]);

  if (!message) return null;
  return (
    <View
      accessible
      accessibilityLiveRegion="polite"
      accessibilityRole="alert"
      pointerEvents="none"
      style={[styles.toast, { backgroundColor: c.primary, borderColor: c.primaryText }]}
    >
      <Text style={{ color: c.primaryText, fontWeight: '700' }}>✓  {message}</Text>
    </View>
  );
}

/** Cible tactile générique pour les listes d'actions (menus, filtres). */
export function ActionRow({ label, detail, onPress, danger }: {
  label: string; detail?: string; onPress: () => void; danger?: boolean;
}) {
  const c = useColors();
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${label}${detail ? `, ${detail}` : ''}`}
      onPress={onPress}
      style={({ pressed }) => [styles.actionRow, { opacity: pressed ? 0.7 : 1 }]}
    >
      <Text style={{ color: danger ? c.danger : c.primary, fontWeight: '700' }}>{`${label} →`}</Text>
      {detail ? <Text style={{ color: c.subtitle, fontSize: 12 }}>{detail}</Text> : null}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  screen: { gap: 14, paddingBottom: 48 },
  headerRow: { flexDirection: 'row', alignItems: 'center', gap: 12 },
  title: { fontSize: 26, fontWeight: '800' },
  subtitle: { fontSize: 13, marginTop: 2 },
  card: { borderWidth: 1, borderRadius: 18, padding: 16, gap: 10 },
  metric: { flex: 1, borderWidth: 1, borderRadius: 16, padding: 14 },
  metricLabel: { fontSize: 12, textTransform: 'uppercase', letterSpacing: 0.6 },
  metricValue: { fontSize: 19, fontWeight: '800', marginTop: 6 },
  metricHint: { fontSize: 11, marginTop: 4 },
  button: { borderRadius: 14, paddingHorizontal: 18, alignItems: 'center', justifyContent: 'center' },
  buttonText: { fontWeight: '700', fontSize: 15 },
  field: { gap: 6 },
  fieldLabel: { fontSize: 12, fontWeight: '600' },
  input: { borderWidth: 1, borderRadius: 12, paddingHorizontal: 14, paddingVertical: 12, fontSize: 16 },
  banner: { borderWidth: 1, borderRadius: 12, padding: 12 },
  row: { flexDirection: 'row', alignItems: 'center', gap: 8, minHeight: 44 },
  state: { alignItems: 'center', justifyContent: 'center', paddingVertical: 28 },
  toast: {
    position: 'absolute', left: 12, right: 12, bottom: 76,
    borderWidth: 1, borderRadius: 14, padding: 12, zIndex: 20,
  },
  actionRow: { paddingVertical: 12, minHeight: 44, justifyContent: 'center', gap: 2 },
});
