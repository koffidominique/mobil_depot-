const state = { isConnected: true, isInternetReachable: true, type: 'wifi' as const };

const NetInfo = {
  fetch: async () => state,
  addEventListener: (_listener: (value: typeof state) => void) => () => undefined,
  useNetInfo: () => state,
};

export default NetInfo;
