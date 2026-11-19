import React from 'react';
import { View } from 'react-native';
import { useDashboard, useSales } from '../hooks/useCore';
import { formatFcfa, formatTimestamp } from '../core/format';
import { Banner, Card, EmptyState, LoadingState, Metric, Row, Screen } from '../components/ui';
import { SyncIndicator } from '../components/SyncIndicator';

export function HomeScreen() {
  const dashboard = useDashboard();
  const sales = useSales();

  return (
    <Screen
      title="Accueil"
      subtitle={dashboard.data ? `Point de vente ${dashboard.data.branch_id}` : 'Chargement du tableau de bord…'}
    >
      <Banner message={dashboard.error?.message ?? null} />
      <SyncIndicator />
      {dashboard.isPending && !dashboard.data ? <LoadingState label="Ouverture du tableau de bord…" /> : null}
      <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 10 }}>
        <Metric
          label="CA du jour"
          value={formatFcfa(dashboard.data?.daily_revenue_fcfa ?? 0)}
          hint={`${dashboard.data?.daily_sales_count ?? 0} ventes`}
        />
        <Metric
          label="Créances clients"
          value={formatFcfa(dashboard.data?.receivables_fcfa ?? 0)}
          hint="Comptes financiers séparés des emballages"
        />
        <Metric
          label="Encaissements 24 h"
          value={formatFcfa(dashboard.data?.collected_fcfa ?? 0)}
        />
        <Metric
          label="Dépenses 24 h"
          value={formatFcfa(dashboard.data?.expenses_fcfa ?? 0)}
        />
        <Metric
          label="Crédits accordés 24 h"
          value={formatFcfa(dashboard.data?.credits_granted_fcfa ?? 0)}
        />
        <Metric
          label="Dettes fournisseurs"
          value={formatFcfa(dashboard.data?.supplier_debts_fcfa ?? 0)}
        />
        <Metric
          label="Valeur stock"
          value={formatFcfa(dashboard.data?.stock_value_fcfa ?? 0)}
          hint={`${dashboard.data?.low_stock_alerts ?? 0} bas stock · ${dashboard.data?.out_of_stock_alerts ?? 0} ruptures`}
        />
      </View>

      <Card>
        <Row
          label="Opérations en attente de synchronisation"
          value={String(dashboard.data?.pending_sync_operations ?? 0)}
        />
        <Row label="Mode" value="Offline-first · FCFA entier" />
        <Row
          label="CA confirmé serveur / local"
          value={`${formatFcfa(dashboard.data?.confirmed_revenue_fcfa ?? 0)} / ${formatFcfa(dashboard.data?.local_revenue_fcfa ?? 0)}`}
        />
        <Row label="Alertes emballages" value={String(dashboard.data?.packaging_alerts ?? 0)} />
      </Card>

      <Card>
        <View style={{ gap: 6 }}>
          <Row label="Activité récente" value={`${sales.data?.sales.length ?? 0} ventes locales`} />
          {sales.data?.sales.length === 0 ? (
            <EmptyState title="Aucune vente locale" message="Les ventes créées sur cet appareil apparaîtront ici." />
          ) : null}
          {(sales.data?.sales ?? []).slice(0, 5).map((sale) => (
            <Row
              key={sale.id}
              label={`${sale.receipt_number} · ${formatTimestamp(sale.occurred_at)}`}
              value={`${formatFcfa(sale.total_fcfa)} · ${sale.sync_status}`}
            />
          ))}
        </View>
      </Card>
    </Screen>
  );
}
