import React from 'react';
import { Text } from 'react-native';
import { Card, Metric, Row, Screen, useColors } from '../components/ui';
import { formatFcfa } from '../core/format';
import { useReports } from '../hooks/useCore';

export function ReportsScreen() {
  const c = useColors();
  const report = useReports();
  const data = report.data;
  return (
    <Screen title="Rapports" subtitle="Données locales durables · chiffre d'affaires ≠ encaissements">
      <Card>
        <Metric label="Chiffre d'affaires 24 h" value={formatFcfa(data?.revenue_fcfa ?? 0)} />
        <Metric label="Encaissements 24 h" value={formatFcfa(data?.cash_collected_fcfa ?? 0)} />
        <Metric label="Crédits 24 h" value={formatFcfa(data?.credits_fcfa ?? 0)} />
        <Metric label="Dépenses 24 h" value={formatFcfa(data?.expenses_fcfa ?? 0)} />
      </Card>
      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Soldes et stock</Text>
        <Row label="Créances clients" value={formatFcfa(data?.receivables_fcfa ?? 0)} />
        <Row label="Dettes fournisseurs" value={formatFcfa(data?.supplier_debts_fcfa ?? 0)} />
        <Row label="Valeur indicative du stock" value={formatFcfa(data?.stock_value_fcfa ?? 0)} />
        <Row label="Marge estimée" value={formatFcfa(data?.estimated_margin_fcfa ?? 0)} />
      </Card>
      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Ventes par produit</Text>
        {(data?.by_product ?? []).map((item) => (
          <Row
            key={item.variant_name}
            label={`${item.variant_name} · ${item.bottles_sold} bouteilles`}
            value={formatFcfa(item.revenue_fcfa)}
          />
        ))}
      </Card>
      <Text style={{ color: c.subtitle, fontSize: 11 }}>
        Le bénéfice est une estimation locale : il ne remplace pas la comptabilité du serveur central.
      </Text>
    </Screen>
  );
}
