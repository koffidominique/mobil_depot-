import { create } from 'zustand';
import type { AuthSession, CartLineInput, PaymentInput, MixedCaseInput } from '../core/types';

export type ThemeMode = 'light' | 'dark';

interface AppState {
  session: AuthSession | null;
  themeMode: ThemeMode;
  cartLines: CartLineInput[];
  mixedCases: MixedCaseInput[];
  payments: PaymentInput[];
  selectedCustomerId: string | null;
  lastMessage: string | null;
  setSession: (session: AuthSession | null) => void;
  setThemeMode: (mode: ThemeMode) => void;
  setCustomer: (id: string | null) => void;
  addLine: (line: CartLineInput) => void;
  updateLineCount: (lineKey: string, delta: number) => void;
  addMixedCase: (item: MixedCaseInput) => void;
  clearCart: () => void;
  setPayments: (payments: PaymentInput[]) => void;
  notify: (message: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  session: null,
  themeMode: 'light',
  cartLines: [],
  mixedCases: [],
  payments: [],
  selectedCustomerId: null,
  lastMessage: null,

  setSession: (session) => set({ session }),
  setThemeMode: (themeMode) => set({ themeMode }),
  setCustomer: (selectedCustomerId) => set({ selectedCustomerId }),
  addLine: (line) =>
    set((state) => {
      const existing = state.cartLines.find((item) => item.line_key === line.line_key);
      if (existing) {
        return {
          cartLines: state.cartLines.map((item) =>
            item.line_key === line.line_key
              ? { ...item, unit_count: item.unit_count + line.unit_count }
              : item,
          ),
        };
      }
      return { cartLines: [...state.cartLines, line] };
    }),
  updateLineCount: (lineKey, delta) =>
    set((state) => ({
      cartLines: state.cartLines
        .map((item) =>
          item.line_key === lineKey ? { ...item, unit_count: item.unit_count + delta } : item,
        )
        .filter((item) => item.unit_count > 0),
    })),
  addMixedCase: (item) => set((state) => ({ mixedCases: [...state.mixedCases, item] })),
  clearCart: () => set({ cartLines: [], mixedCases: [], payments: [] }),
  setPayments: (payments) => set({ payments }),
  notify: (lastMessage) => set({ lastMessage }),
}));
