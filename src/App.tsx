import { NavigationContainer, DarkTheme, DefaultTheme } from '@react-navigation/native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React, { useEffect, useRef, useState } from 'react';
import { ActivityIndicator, AppState, Text, View, type AppStateStatus } from 'react-native';
import NetInfo from '@react-native-community/netinfo';
import RNFS from 'react-native-fs';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { FeedbackToast } from './components/ui';
import { depot } from './core/client';
import { DEFAULT_AUTO_LOCK_MS, shouldAutoLock } from './core/security';
import { useSyncStatus, useSynchronization } from './hooks/useCore';
import { restoreSession } from './core/session';
import { useAppStore } from './store/useAppStore';
import { colorsFor } from './theme';
import { darkColors } from './theme';
import type { MainTabParamList, RootStackParamList } from './navigation/types';
import { HomeScreen } from './screens/HomeScreen';
import { PosScreen } from './screens/PosScreen';
import { StockScreen } from './screens/StockScreen';
import { CustomersScreen } from './screens/CustomersScreen';
import { MenuScreen } from './screens/MenuScreen';
import { LoginScreen } from './screens/LoginScreen';
import { UnlockScreen } from './screens/UnlockScreen';
import { SyncScreen } from './screens/SyncScreen';
import { DesktopScreen } from './screens/DesktopScreen';
import { PurchasesScreen } from './screens/PurchasesScreen';
import { SuppliersScreen } from './screens/SuppliersScreen';
import { CashScreen } from './screens/CashScreen';
import { ReportsScreen } from './screens/ReportsScreen';
import { CatalogScreen } from './screens/CatalogScreen';
import { ReceiptsScreen } from './screens/ReceiptsScreen';
import { StatementScreen } from './screens/StatementScreen';

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, staleTime: 5000 } },
});

const Stack = createNativeStackNavigator<RootStackParamList>();
const Tabs = createBottomTabNavigator<MainTabParamList>();

function MainTabs() {
  const c = useColorsFromStore();
  return (
    <Tabs.Navigator
      screenOptions={{
        headerStyle: { backgroundColor: c.surface },
        headerTintColor: c.text,
        tabBarStyle: { backgroundColor: c.surface, borderTopColor: c.border },
        tabBarActiveTintColor: c.primary,
        tabBarInactiveTintColor: c.subtitle,
      }}
    >
      <Tabs.Screen name="Home" component={HomeScreen} options={{ title: 'Accueil' }} />
      <Tabs.Screen name="Sales" component={PosScreen} options={{ title: 'Ventes' }} />
      <Tabs.Screen name="Stock" component={StockScreen} options={{ title: 'Stock' }} />
      <Tabs.Screen name="Customers" component={CustomersScreen} options={{ title: 'Clients' }} />
      <Tabs.Screen name="Menu" component={MenuScreen} options={{ title: 'Menu' }} />
    </Tabs.Navigator>
  );
}

function useColorsFromStore() {
  const mode = useAppStore((state) => state.themeMode);
  return colorsFor(mode);
}

/** Déclencheurs autorisés : démarrage, retour au premier plan, retour du réseau. */
function AutoSync() {
  const status = useSyncStatus();
  const sync = useSynchronization();
  const configured = Boolean(status.data?.server_url);
  const wasConnected = useRef<boolean | null>(null);

  useEffect(() => {
    if (!configured) return;
    sync.mutate();
    // Le déclenchement au démarrage ne doit se produire qu'une fois par session.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [configured]);

  useEffect(() => {
    const unsubscribe = NetInfo.addEventListener((state) => {
      const connected = Boolean(state.isConnected);
      if (configured && connected && wasConnected.current === false) sync.mutate();
      wasConnected.current = connected;
    });
    const onForeground = (next: AppStateStatus) => {
      if (configured && next === 'active') sync.mutate();
    };
    const subscription = AppState.addEventListener('change', onForeground);
    return () => {
      unsubscribe();
      subscription.remove();
    };
  }, [configured, sync]);

  return null;
}

/** Verrouille la session après un passage en arrière-plan trop long. */
function AutoLock() {
  const session = useAppStore((state) => state.session);
  const setLocked = useAppStore((state) => state.setLocked);
  const backgroundAt = useRef<number | null>(null);

  useEffect(() => {
    let autoLockMs = DEFAULT_AUTO_LOCK_MS;
    void depot.getSecurityPolicy().then((policy) => {
      autoLockMs = policy.auto_lock_ms || DEFAULT_AUTO_LOCK_MS;
    }).catch(() => undefined);
    const onForeground = (next: AppStateStatus) => {
      if (next === 'background' || next === 'inactive') {
        backgroundAt.current = Date.now();
        return;
      }
      if (next === 'active' && backgroundAt.current != null) {
        if (shouldAutoLock(backgroundAt.current, autoLockMs)) {
          if (session?.token) void depot.lockSession(session.token).catch(() => undefined);
          setLocked(true);
        }
        backgroundAt.current = null;
      }
    };
    const subscription = AppState.addEventListener('change', onForeground);
    return () => subscription.remove();
  }, [session?.token, setLocked]);

  return null;
}

function Bootstrap() {
  const [ready, setReady] = useState(false);
  const [startupError, setStartupError] = useState<string | null>(null);
  const session = useAppStore((state) => state.session);
  const setSession = useAppStore((state) => state.setSession);
  const themeMode = useAppStore((state) => state.themeMode);
  const locked = useAppStore((state) => state.locked);

  useEffect(() => {
    let mounted = true;
    const start = async () => {
      try {
        const dbPath = `${RNFS.DocumentDirectoryPath}/depotpro_local.db`;
        await depot.initializeDatabase(dbPath);
        const restored = await restoreSession();
        if (mounted) setSession(restored);
      } catch (error) {
        if (mounted) setStartupError(error instanceof Error ? error.message : 'Erreur de démarrage.');
      } finally {
        if (mounted) setReady(true);
      }
    };
    void start();
    return () => {
      mounted = false;
    };
  }, [setSession]);

  if (!ready) {
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center', backgroundColor: darkColors.background }}>
        <ActivityIndicator color={darkColors.primary} />
        <Text style={{ color: darkColors.text, marginTop: 12 }}>Ouverture du moteur Rust…</Text>
      </View>
    );
  }

  if (startupError) {
    return (
      <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center', padding: 24, backgroundColor: darkColors.background }}>
        <Text style={{ color: darkColors.danger, fontWeight: '700' }}>Démarrage impossible</Text>
        <Text style={{ color: darkColors.text, marginTop: 8, textAlign: 'center' }}>{startupError}</Text>
      </View>
    );
  }

  return (
    <NavigationContainer theme={themeMode === 'dark' ? DarkTheme : DefaultTheme}>
      {session ? <><AutoSync /><AutoLock /></> : null}
      <Stack.Navigator screenOptions={{ headerShown: false }}>
        {locked ? (
          <Stack.Screen name="Unlock" component={UnlockScreen} />
        ) : session ? (
          <>
            <Stack.Screen name="Main" component={MainTabs} />
            <Stack.Screen
              name="Sync"
              component={SyncScreen}
              options={{ headerShown: true, title: 'Synchronisation & paramètres' }}
            />
            <Stack.Screen name="Purchases" component={PurchasesScreen} options={{ headerShown: true, title: 'Achats & réceptions' }} />
            <Stack.Screen name="Suppliers" component={SuppliersScreen} options={{ headerShown: true, title: 'Fournisseurs' }} />
            <Stack.Screen name="Cash" component={CashScreen} options={{ headerShown: true, title: 'Caisse' }} />
            <Stack.Screen name="Reports" component={ReportsScreen} options={{ headerShown: true, title: 'Rapports' }} />
            <Stack.Screen name="Catalog" component={CatalogScreen} options={{ headerShown: true, title: 'Catalogue' }} />
            <Stack.Screen name="Receipts" component={ReceiptsScreen} options={{ headerShown: true, title: 'Reçus' }} />
            <Stack.Screen name="Statement" component={StatementScreen} options={{ headerShown: true, title: 'Bordereaux' }} />
            <Stack.Screen name="Desktop" component={DesktopScreen} options={{ headerShown: true, title: 'Parité Desktop' }} />
          </>
        ) : (
          <Stack.Screen name="Login" component={LoginScreen} />
        )}
      </Stack.Navigator>
    </NavigationContainer>
  );
}

export default function App() {
  return (
    <SafeAreaProvider>
      <QueryClientProvider client={queryClient}>
        <FeedbackToast />
        <Bootstrap />
      </QueryClientProvider>
    </SafeAreaProvider>
  );
}
