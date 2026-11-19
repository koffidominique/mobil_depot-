import { useMutation } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Share, Text, TextInput } from 'react-native';
import RNFS from 'react-native-fs';
import { Banner, Card, Metric, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { formatTimestamp } from '../core/format';
import { formatFcfa } from '../core/format';
import { useReports } from '../hooks/useCore';

export function ReportsScreen() {
  const c = useColors();
  const [fromDate, setFromDate] = useState(() => new Date(Date.now() - 24 * 3600_000).toISOString().slice(0, 10));
  const [toDate, setToDate] = useState(() => new Date().toISOString().slice(0, 10));
  const [range, setRange] = useState<{ fromMs: number; toMs: number } | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const report = useReports(range?.fromMs, range?.toMs);
  const exportCsv = useMutation({
    mutationFn: async () => {
      const data = report.data;
      if (!data) throw new Error('Rapport indisponible.');
      const rows = [
        ['periode_debut', 'periode_fin', 'ca_fcfa', 'encaissements_fcfa', 'credits_fcfa', 'depenses_fcfa', 'marge_estimee_fcfa'],
        [formatTimestamp(data.from_ms), formatTimestamp(data.to_ms), data.revenue_fcfa, data.cash_collected_fcfa,
          data.credits_fcfa, data.expenses_fcfa, data.estimated_margin_fcfa],
        [], ['reference', 'quantite_bouteilles', 'ca_fcfa'],
        ...(data.by_product ?? []).map((item) => [item.variant_name, item.bottles_sold, item.revenue_fcfa]),
      ];
      const csv = rows.map((row) => row.join(';')).join('\n');
      const path = `${RNFS.DocumentDirectoryPath}/rapport-depotpro-${new Date().toISOString().slice(0, 10)}.csv`;
      await RNFS.writeFile(path, csv, 'utf8');
      return path;
    },
    onSuccess: async (path) => {
      setExportMessage(`Rapport exporté : ${path}`);
      await Share.share({ url: `file://${path}` });
    },
    onError: (err: Error) => setExportMessage(err.message),
  });

  function applyRange() {
    const fromMs = Date.parse(`${fromDate}T00:00:00`);
    const toMs = Date.parse(`${toDate}T23:59:59`);
    setRange(Number.isNaN(fromMs) || Number.isNaN(toMs) ? null : { fromMs, toMs });
  }
  const data = report.data;
  return (
    <Screen title="Rapports" subtitle="Données locales durables · chiffre d'affaires ≠ encaissements">
      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Période</Text>
        <TextInput value={fromDate} onChangeText={setFromDate} placeholder="AAAA-MM-JJ"
          placeholderTextColor={c.subtitle} style={{ borderWidth: 1, borderColor: c.border, borderRadius: 10, padding: 10, color: c.text }} />
        <TextInput value={toDate} onChangeText={setToDate} placeholder="AAAA-MM-JJ"
          placeholderTextColor={c.subtitle} style={{ borderWidth: 1, borderColor: c.border, borderRadius: 10, padding: 10, color: c.text }} />
        <Pressable onPress={applyRange}>
          <Text style={{ color: c.primary, fontWeight: '700' }}>Appliquer la période</Text>
        </Pressable>
        <PrimaryButton label="Exporter le rapport CSV" onPress={() => exportCsv.mutate()} loading={exportCsv.isPending} />
        <Banner tone="info" message={exportMessage} />
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          {range ? `Du ${formatTimestamp(range.fromMs)} au ${formatTimestamp(range.toMs)}` : '24 dernières heures'}
        </Text>
      </Card>
      <Card>
        <Metric label="Chiffre d'affaires de la période" value={formatFcfa(data?.revenue_fcfa ?? 0)} />
        <Metric label="Encaissements période" value={formatFcfa(data?.cash_collected_fcfa ?? 0)} />
        <Metric label="Crédits période" value={formatFcfa(data?.credits_fcfa ?? 0)} />
        <Metric label="Dépenses période" value={formatFcfa(data?.expenses_fcfa ?? 0)} />
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
