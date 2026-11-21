/**
 * Navigation minimaliste pour l'aperçu web : mêmes noms d'écrans que l'app,
 * aucun module natif. Ce shim n'est jamais utilisé dans le bundle mobile.
 */
import React, { Children, createContext, isValidElement, useContext, useEffect, useState } from 'react';
import { Pressable, ScrollView, Text, View } from 'react-native-web';

interface StackFrame {
  name: string;
  /** Paramètres d'ouverture d'écran (ex. filtre client des commandes). */
  params?: Record<string, unknown>;
}

interface StackContextValue {
  route: string;
  params?: Record<string, unknown>;
  stack: StackFrame[];
  navigate: (name: string, params?: Record<string, unknown>) => void;
  goBack: () => void;
  replace: (name: string) => void;
}

const StackContext = createContext<StackContextValue | null>(null);

export const DefaultTheme = { dark: false, colors: { primary: '#1B7F5A', background: '#F5F7F5', text: '#0B2118', card: '#FFFFFF', border: '#DDE5DF', notification: '#B7791F' } };
export const DarkTheme = { dark: true, colors: { primary: '#4ED8A0', background: '#0B1C16', text: '#E8F3ED', card: '#12281F', border: '#254535', notification: '#E0A83E' } };

export function NavigationContainer({ children, theme }: { children: React.ReactNode; theme?: unknown }) {
  void theme;
  const [stack, setStack] = useState<StackFrame[]>([{ name: 'Main' }]);
  const current = stack[stack.length - 1] ?? { name: 'Main' };
  const value: StackContextValue = {
    route: current.name,
    params: current.params,
    stack,
    navigate: (name, params) => setStack((frames) => [...frames, { name, params }]),
    goBack: () => setStack((frames) => (frames.length > 1 ? frames.slice(0, -1) : frames)),
    replace: (name) => setStack([{ name }]),
  };
  return <StackContext.Provider value={value}>{children}</StackContext.Provider>;
}

export interface WebNavigation<T = unknown> {
  navigate: (name: string, params?: Record<string, unknown>) => void;
  goBack: () => void;
  setOptions: (options?: T) => void;
  canGoBack: () => boolean;
}

export function useNavigation<T = unknown>(): WebNavigation<T> {
  const context = useContext(StackContext);
  return {
    navigate: (name: string, params?: Record<string, unknown>) => context?.navigate(name, params),
    goBack: () => context?.goBack(),
    setOptions: (_options?: T) => undefined,
    canGoBack: () => Boolean(context && context.stack.length > 1),
  };
}

export type NativeStackNavigationProp<T = unknown> = WebNavigation<T>;

export function useRoute() {
  const context = useContext(StackContext);
  return { name: context?.route ?? '', params: context?.params };
}

function screensOf(children: React.ReactNode): Array<{ name: string; Component?: React.ComponentType<any>; options: any }> {
  const result: Array<{ name: string; Component?: React.ComponentType<any>; options: any }> = [];
  const visit = (nodes: React.ReactNode) => {
    Children.toArray(nodes).forEach((node) => {
      if (!isValidElement(node)) return;
      if (node.type === React.Fragment) {
        visit((node.props as { children?: React.ReactNode }).children);
        return;
      }
      const props = node.props as { name?: string; component?: React.ComponentType<any>; options?: any };
      if (props.name) result.push({ name: props.name, Component: props.component, options: props.options ?? {} });
    });
  };
  visit(children);
  return result;
}

function StackNavigator({ children, screenOptions }: { children: React.ReactNode; screenOptions?: any }) {
  const context = useContext(StackContext);
  const screens = screensOf(children);
  const available = screens.map((screen) => screen.name);
  const route = available.includes(context?.route ?? '') ? (context?.route as string) : (available[0] ?? '');
  const current = screens.find((screen) => screen.name === route);

  useEffect(() => {
    if (route && context && context.route !== route) context.replace(route);
  }, [route, context?.route]); // eslint-disable-line react-hooks/exhaustive-deps

  const headerShown = (current?.options?.headerShown as boolean | undefined)
    ?? (screenOptions?.headerShown as boolean | undefined)
    ?? true;
  const title = (current?.options?.title as string | undefined) ?? route;
  const Component = current?.Component;

  return (
    <View style={{ flex: 1 }}>
      {headerShown ? (
        <View style={{ paddingTop: 26, paddingBottom: 10, paddingHorizontal: 14, backgroundColor: '#FFFFFF', borderBottomWidth: 1, borderBottomColor: '#E3EAE5', flexDirection: 'row', alignItems: 'center', gap: 10 }}>
          {context && context.stack.length > 1 ? (
            <Pressable onPress={context.goBack} accessibilityRole="button" hitSlop={8}>
              <Text style={{ color: '#1B7F5A', fontWeight: '700', fontSize: 16 }}>←</Text>
            </Pressable>
          ) : null}
          <Text style={{ color: '#0B2118', fontWeight: '800', fontSize: 16 }}>{title}</Text>
        </View>
      ) : null}
      <View style={{ flex: 1 }}>
        {Component ? <Component /> : null}
      </View>
    </View>
  );
}

function TabsNavigator({ children, screenOptions }: { children: React.ReactNode; screenOptions?: any }) {
  void screenOptions;
  const screens = screensOf(children);
  const [active, setActive] = useState(screens[0]?.name ?? '');
  const available = screens.map((screen) => screen.name);
  const route = available.includes(active) ? active : (available[0] ?? '');
  const current = screens.find((screen) => screen.name === route);
  const Component = current?.Component;

  return (
    <View style={{ flex: 1 }}>
      <View style={{ flex: 1 }}>
        {Component ? <Component /> : null}
      </View>
      <View style={{ flexDirection: 'row', borderTopWidth: 1, borderTopColor: '#E3EAE5', backgroundColor: '#FFFFFF' }}>
        {screens.map((screen) => (
          <Pressable
            key={screen.name}
            onPress={() => setActive(screen.name)}
            accessibilityRole="tab"
            style={{ flex: 1, paddingVertical: 11, paddingHorizontal: 2, alignItems: 'center', backgroundColor: screen.name === route ? '#EAF5EF' : '#FFFFFF' }}
          >
            <Text numberOfLines={1} style={{ fontSize: 10, color: screen.name === route ? '#1B7F5A' : '#60756A', fontWeight: screen.name === route ? '800' : '600' }}>
              {(screen.options?.title as string | undefined) ?? screen.name}
            </Text>
          </Pressable>
        ))}
      </View>
    </View>
  );
}

export interface WebNavigator<T = unknown> {
  Navigator: React.ComponentType<any>;
  Screen: (props: { name: string; component?: React.ComponentType<any>; options?: { title?: string; headerShown?: boolean; tabBarIcon?: (props: any) => T } }) => null;
}

export function createNativeStackNavigator<T = unknown>(): WebNavigator<T> {
  return { Navigator: StackNavigator, Screen: () => null };
}

export function createBottomTabNavigator<T = unknown>(): WebNavigator<T> {
  return { Navigator: TabsNavigator, Screen: () => null };
}

export function useScrollToTop() {}
export { ScrollView as StackScrollView };
