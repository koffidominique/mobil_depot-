import { NavigationContainer, DarkTheme, DefaultTheme } from '@react-navigation/native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React, { useEffect, useState } from 'react';
import { ActivityIndicator, Text, View } from 'react-native';
import RNFS from 'react-native-fs';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { depot } from './core/client';
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
import { SyncScreen } from './screens/SyncScreen';

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

function Bootstrap() {
  const [ready, setReady] = useState(false);
  const [startupError, setStartupError] = useState<string | null>(null);
  const session = useAppStore((state) => state.session);
  const setSession = useAppStore((state) => state.setSession);
  const themeMode = useAppStore((state) => state.themeMode);

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
      <Stack.Navigator screenOptions={{ headerShown: false }}>
        {session ? (
          <>
            <Stack.Screen name="Main" component={MainTabs} />
            <Stack.Screen
              name="Sync"
              component={SyncScreen}
              options={{ headerShown: true, title: 'Synchronisation & paramètres' }}
            />
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
        <Bootstrap />
      </QueryClientProvider>
    </SafeAreaProvider>
  );
}
