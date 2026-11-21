import { useRoute } from '@react-navigation/native';
import { useQuery } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Share, Text, View } from 'react-native';
import { Banner, Card, Field, Metric, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { pdfFileUrl, printReceiptPdf, writeReceiptPdf } from '../core/receiptPrinter';
import type { ReceiptPdfInput } from '../core/receiptPdf';
import { formatFcfa, formatQuantity, formatTimestamp } from '../core/format';
import type { PackagingAccount, SaleReceipt } from '../core/types';
import { useSales } from '../hooks/useCore';

const SALES_PAGE = 25;
const DAY_MS = 86400000;

type Period = 'ALL' | '7D' | '30D';

/**
 * Commandes clients : la liste est filtrée et paginée par le moteur (client,
 * période, crédit, recherche) et affiche les totaux de la sélection. Le détail
 * d'un reçu reste la seule source des lignes, paiements et emballages.
 */
export function ReceiptsScreen() {
  const c = useColors();
  // L'écran accepte un filtre client facultatif (ouverture depuis une fiche).
  const route = useRoute();
  const initialCustomer = (route.params as { customer_id?: string } | undefined)?.customer_id ?? '';
  const [customerId, setCustomerId] = useState(initialCustomer);
  const [search, setSearch] = useState('');
  const [creditOnly, setCreditOnly] = useState(false);
  const [period, setPeriod] = useState<Period>('ALL');
  const [limit, setLimit] = useState(SALES_PAGE);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [pdfMessage, setPdfMessage] = useState<string | null>(null);

  const fromMs = period === 'ALL' ? undefined : Date.now() - (period === '7D' ? 7 : 30) * DAY_MS;
  const sales = useSales({
    customer_id: customerId || undefined,
    search: search.trim() ? search.trim() : undefined,
    credit_only: creditOnly,
    from_ms: fromMs,
    limit,
  });

  const receipt = useQuery<SaleReceipt, Error>({
    queryKey: ['sale-receipt', selectedId],
    enabled: Boolean(selectedId),
    queryFn: () => depot.getSaleReceipt(selectedId ?? ''),
  });
  const expectedEmptyBottles = (receipt.data?.lines ?? []).reduce(
    (sum, line) => (line.unit === 'CASE' || line.unit === 'HALF_CASE' ? sum + line.physical_units : sum),
    0,
  );
  const expectedCrates = (receipt.data?.lines ?? []).filter(
    (line) => line.unit === 'CASE' || line.unit === 'HALF_CASE',
  ).length;
  const packaging = useQuery<PackagingAccount, Error>({
    queryKey: ['packaging-account', 'CUSTOMER', receipt.data?.customer_id],
    enabled: Boolean(receipt.data?.customer_id),
    queryFn: () => depot.getPackagingAccount('CUSTOMER', receipt.data?.customer_id ?? ''),
  });
  const balances = (packaging.data?.balances ?? []).filter((item) => item.status !== 'RETURNED');
  const receivedBottles = balances
    .filter((item) => item.item_kind === 'BOTTLE_PROFILE')
    .reduce((sum, item) => sum + item.received_qty, 0);
  const receivedCrates = balances
    .filter((item) => item.item_kind === 'CRATE')
    .reduce((sum, item) => sum + item.received_qty, 0);
  const outstanding = balances.reduce((sum, item) => sum + item.outstanding_qty, 0);

  const listed = sales.data?.sales ?? [];
  const totalCount = sales.data?.total_count ?? 0;

  async function receiptPdfInput(): Promise<ReceiptPdfInput> {
    const receiptData = receipt.data;
    if (!receiptData) throw new Error('Reçu indisponible.');
    return {
      depotName: 'DEPOTPRO - Dépôt Marcory',
      receiptNumber: receiptData.receipt_number,
      dateLabel: formatTimestamp(receiptData.occurred_at),
      customer: receiptData.customer_name,
      lines: receiptData.lines.map((line) => ({
        label: `${line.variant_name} (${line.unit})`,
        quantity: String(line.unit_count),
        amountFcfa: String(line.line_total_fcfa),
      })),
      subtotalFcfa: String(receiptData.subtotal_fcfa),
      discountFcfa: String(receiptData.discount_fcfa),
      totalFcfa: String(receiptData.total_fcfa),
      paidFcfa: String(receiptData.paid_confirmed_fcfa),
      creditFcfa: String(receiptData.credit_fcfa),
      packagingLines: [
        `Casiers attendus/recus: ${expectedCrates}/${receivedCrates}`,
        `Bouteilles vides attendues/recues: ${expectedEmptyBottles}/${receivedBottles}`,
        `Reste a restituer: ${outstanding}`,
      ],
      notes: [
        'Recu commercial - pas une facture normalisee.',
        'Aucun frais d emballage n est ajoute automatiquement.',
      ],
    };
  }

  async function exportPdf() {
    try {
      const path = await writeReceiptPdf(await receiptPdfInput());
      setPdfMessage(`PDF enregistre : ${path}`);
      await Share.share({ url: pdfFileUrl(path) });
    } catch (error) {
      setPdfMessage(error instanceof Error ? error.message : 'Export PDF impossible.');
    }
  }

  async function printPdf() {
    try {
      const path = await writeReceiptPdf(await receiptPdfInput());
      const started = await printReceiptPdf(path);
      setPdfMessage(started ? 'Dialogue d’impression ouvert.' : 'Impression annulée.');
    } catch (error) {
      setPdfMessage(error instanceof Error ? error.message : 'Impression impossible.');
    }
  }

  async function shareReceipt() {
    if (!receipt.data) return;
    const lines = receipt.data.lines.map(
      (line) => `${line.variant_name} · ${line.unit} × ${line.unit_count} · ${line.line_total_fcfa} FCFA`,
    );
    await Share.share({
      message: [
        `DEPOTPRO — Reçu ${receipt.data.receipt_number}`,
        `${formatTimestamp(receipt.data.occurred_at)} · ${receipt.data.customer_name}`,
        ...lines,
        `Total ${receipt.data.total_fcfa} FCFA · payé ${receipt.data.paid_confirmed_fcfa} FCFA`,
        `Emballages attendus : ${expectedEmptyBottles} bouteilles, ${expectedCrates} casiers`,
        'Reçu commercial — pas une facture normalisée.',
      ].join('\n'),
    });
  }

  return (
    <Screen title="Commandes clients" subtitle="Reçus commerciaux — pas une facture normalisée">
      <Banner message={sales.error?.message ?? null} />

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Sélection</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          <Metric
            label="Commandes"
            value={formatQuantity(totalCount)}
            hint={listed.length < totalCount ? `${formatQuantity(listed.length)} affichée(s)` : 'Toutes affichées'}
          />
          <Metric
            label="Chiffre d'affaires"
            value={formatFcfa(sales.data?.totals.total_fcfa ?? 0)}
            hint="Total de la sélection"
          />
          <Metric
            label="Encaissé confirmé"
            value={formatFcfa(sales.data?.totals.paid_confirmed_fcfa ?? 0)}
            hint="Hors crédit et mobile money en attente"
          />
          <Metric
            label="Crédit accordé"
            value={formatFcfa(sales.data?.totals.credit_fcfa ?? 0)}
            hint="Reste dû par les clients de la sélection"
          />
        </View>
        <Field label="Rechercher un reçu ou un client" value={search} onChangeText={setSearch} />
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 12 }}>
          {(
            [
              { id: 'ALL', label: 'Tout' },
              { id: '7D', label: '7 jours' },
              { id: '30D', label: '30 jours' },
            ] as Array<{ id: Period; label: string }>
          ).map((entry) => (
            <Pressable key={entry.id} onPress={() => setPeriod(entry.id)} accessibilityRole="button">
              <Text style={{ color: period === entry.id ? c.primary : c.subtitle, fontWeight: '700' }}>
                {entry.label}
              </Text>
            </Pressable>
          ))}
          <Pressable onPress={() => setCreditOnly((current) => !current)} accessibilityRole="button">
            <Text style={{ color: creditOnly ? c.primary : c.subtitle, fontWeight: '700' }}>
              {creditOnly ? 'Crédit uniquement (actif)' : 'Crédit uniquement'}
            </Text>
          </Pressable>
          {customerId ? (
            <Pressable onPress={() => setCustomerId('')} accessibilityRole="button">
              <Text style={{ color: c.primary, fontWeight: '700' }}>Retirer le filtre client</Text>
            </Pressable>
          ) : null}
        </View>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Commandes</Text>
        {listed.length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucune commande ne correspond à cette sélection.</Text>
        ) : (
          listed.map((sale) => (
            <Pressable key={sale.id} onPress={() => setSelectedId(sale.id)} accessibilityRole="button">
              <View style={{ paddingVertical: 6 }}>
                <Row label={`${sale.receipt_number} · ${sale.customer_name}`} value={formatFcfa(sale.total_fcfa)} />
                <Text style={{ color: c.subtitle, fontSize: 11 }}>
                  {`${formatTimestamp(sale.occurred_at)} · ${sale.sync_status} · ${formatQuantity(
                    sale.line_count,
                  )} ligne(s) · payé ${formatFcfa(sale.paid_confirmed_fcfa)}`}
                  {sale.credit_fcfa > 0 ? ` · crédit ${formatFcfa(sale.credit_fcfa)}` : ''}
                  {sale.pending_mobile_money_fcfa > 0
                    ? ` · Mobile Money en attente ${formatFcfa(sale.pending_mobile_money_fcfa)}`
                    : ''}
                  {sale.mixed_line_count > 0 ? ` · ${formatQuantity(sale.mixed_line_count)} ligne(s) mixte(s)` : ''}
                  {sale.packaging_line_count > 0
                    ? ` · ${formatQuantity(sale.packaging_line_count)} ligne(s) d'emballage`
                    : ''}
                </Text>
              </View>
            </Pressable>
          ))
        )}
        {totalCount > listed.length ? (
          <Pressable
            onPress={() => setLimit((current) => current + SALES_PAGE)}
            accessibilityRole="button"
            testID="sales-more"
          >
            <Text style={{ color: c.primary, fontWeight: '700' }}>
              Afficher {Math.min(SALES_PAGE, totalCount - listed.length)} commande(s) de plus
            </Text>
          </Pressable>
        ) : null}
      </Card>

      {receipt.data ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>{receipt.data.receipt_number}</Text>
          <Row label="Client" value={receipt.data.customer_name} />
          {(receipt.data.lines ?? []).map((line, index) => (
            <View key={`${line.variant_name}-${index}`} style={{ paddingVertical: 4 }}>
              <Row
                label={`${line.variant_name} · ${line.unit} × ${line.unit_count}`}
                value={formatFcfa(line.line_total_fcfa)}
              />
              {line.is_mixed || line.is_packaging ? (
                <Text style={{ color: c.subtitle, fontSize: 11 }}>
                  {[line.is_mixed ? 'casier incomplet / mixte' : null, line.is_packaging ? 'consignation' : null]
                    .filter(Boolean)
                    .join(' · ')}
                </Text>
              ) : null}
            </View>
          ))}
          <Row label="Sous-total" value={formatFcfa(receipt.data.subtotal_fcfa)} />
          <Row label="Remise" value={formatFcfa(receipt.data.discount_fcfa)} />
          <Row label="Total" value={formatFcfa(receipt.data.total_fcfa)} />
          <Row label="Payé" value={formatFcfa(receipt.data.paid_confirmed_fcfa)} />
          <Row label="Mobile Money en attente" value={formatFcfa(receipt.data.pending_mobile_money_fcfa)} />
          <Row label="Créance" value={formatFcfa(receipt.data.credit_fcfa)} />
          <Row label="Date" value={formatTimestamp(receipt.data.occurred_at)} />
          <Row label="Lignes" value={formatQuantity(receipt.data.lines.length)} />
          <Text style={{ color: c.text, fontWeight: '800', marginTop: 8 }}>Emballages</Text>
          <Row label="Casiers attendus / reçus" value={`${expectedCrates} / ${receivedCrates}`} />
          <Row
            label="Bouteilles vides attendues / reçues"
            value={`${formatQuantity(expectedEmptyBottles)} / ${formatQuantity(receivedBottles)}`}
          />
          <Row label="Restant à restituer" value={formatQuantity(outstanding)} />
          {balances.map((item) => (
            <Row
              key={`${item.item_kind}-${item.ref_id}`}
              label={`${item.item_kind} · ${item.ref_id} · ${item.status}`}
              value={`${item.received_qty}/${item.expected_qty}`}
            />
          ))}
          <Banner
            tone="info"
            message="Reçu commercial : il n'est pas présenté comme une facture normalisée. Aucun frais d'emballage n'est ajouté automatiquement."
          />
          <Pressable onPress={() => void shareReceipt()}>
            <Text style={{ color: c.primary, fontWeight: '700' }}>Partager le reçu (texte)</Text>
          </Pressable>
          <Pressable onPress={() => void exportPdf()}>
            <Text style={{ color: c.primary, fontWeight: '700' }}>Exporter en PDF</Text>
          </Pressable>
          <Pressable onPress={() => void printPdf()}>
            <Text style={{ color: c.primary, fontWeight: '700' }}>Imprimer le PDF</Text>
          </Pressable>
          <Banner tone="info" message={pdfMessage} />
        </Card>
      ) : null}
      {receipt.isError ? <Banner message={receipt.error.message} /> : null}
    </Screen>
  );
}
