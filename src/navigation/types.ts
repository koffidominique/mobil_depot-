export type RootStackParamList = {
  Login: undefined;
  Unlock: undefined;
  Main: undefined;
  Sync: undefined;
  Purchases: undefined;
  Suppliers: undefined;
  Crates: undefined;
  Cash: undefined;
  Reports: undefined;
  Catalog: undefined;
  Inventories: undefined;
  MixedOrder: undefined;
  CustomerMerge: undefined;
  /** Commandes clients : filtre optionnel sur une fiche de clientèle. */
  Receipts: { customer_id?: string } | undefined;
  Statement: undefined;
  Desktop: undefined;
};

export type MainTabParamList = {
  Home: undefined;
  Sales: undefined;
  Stock: undefined;
  Customers: undefined;
  Menu: undefined;
};
