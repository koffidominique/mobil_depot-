import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity, formatTimestamp } from '../core/format';
import { useInventory, usePurchaseOrders, useSuppliers } from '../hooks/useCore';

export function PurchasesScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const suppliers = useSuppliers();
  const inventory = useInventory();
  const orders = usePurchaseOrders();
  const [supplierId, setSupplierId] = useState('');
  const [variantId, setVariantId] = useState('');
  const [quantity, setQuantity] = useState('1');
  const [orderId, setOrderId] = useState('');
  const [receivedQuantity, setReceivedQuantity] = useState('1');
  const [damagedQuantity, setDamagedQuantity] = useState('0');
  const [invoicePrice, setInvoicePrice] = useState('');
  const variance = useQuery({
    queryKey: ['purchase-variance', orderId],
    enabled: Boolean(orderId),
    queryFn: () => depot.getPurchaseVariance(orderId),
  });
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['purchase-orders'] }),
      queryClient.invalidateQueries({ queryKey: ['inventory'] }),
      queryClient.invalidateQueries({ queryKey: ['suppliers'] }),
      queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
    ]);
  };

  const createOrder = useMutation({
    mutationFn: () =>
      depot.createPurchaseOrder({
        supplier_id: supplierId,
        lines: [{ variant_id: variantId, quantity: Number.parseInt(quantity, 10) }],
      }),
    onSuccess: async (result) => {
      setMessage(`Bon ${result.reference} enregistré · ${formatFcfa(result.total_fcfa)}`);
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const receive = useMutation({
    mutationFn: () =>
      depot.receivePurchase({
        order_id: orderId,
        lines: [{
          variant_id: variantId,
          quantity: Number.parseInt(receivedQuantity, 10),
          damaged_qty: Number.parseInt(damagedQuantity, 10) || 0,
          ...(invoicePrice.trim() ? { unit_price_fcfa: Number.parseInt(invoicePrice, 10) } : {}),
          reason: 'Réception contrôlée',
        }],
      }),
    onSuccess: async (result) => {
      setMessage(`Réception partielle enregistrée · ${formatFcfa(result.received_fcfa)}`);
      setError(null);
      setDamagedQuantity('0');
      setInvoicePrice('');
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Achats & réceptions" subtitle="Bons de commande et réceptions partielles traçables">
      <Banner message={error} />
      <Banner tone="info" message={message} />
      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Nouveau bon de commande</Text>
        <Text style={{ color: c.subtitle, fontSize: 12 }}>Fournisseur</Text>
        {(suppliers.data?.suppliers ?? []).map((supplier) => (
          <Pressable key={supplier.id} onPress={() => setSupplierId(supplier.id)}>
            <Text style={{ color: supplierId === supplier.id ? c.primary : c.text }}>
              {supplier.name}
            </Text>
          </Pressable>
        ))}
        <Text style={{ color: c.subtitle, fontSize: 12 }}>Référence à commander</Text>
        {(inventory.data?.full_drinks ?? []).map((item) => (
          <Pressable key={item.variant_id} onPress={() => setVariantId(item.variant_id)}>
            <Text style={{ color: variantId === item.variant_id ? c.primary : c.text }}>
              {item.name} · {item.volume_label}
            </Text>
          </Pressable>
        ))}
        <Field label="Quantité (casiers)" value={quantity} keyboardType="numeric" onChangeText={setQuantity} />
        <PrimaryButton
          label="Créer le bon de commande"
          loading={createOrder.isPending}
          disabled={!supplierId || !variantId}
          onPress={() => createOrder.mutate()}
        />
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Réception partielle</Text>
        <Text style={{ color: c.subtitle, fontSize: 12 }}>Bon de commande</Text>
        {(orders.data?.orders ?? []).map((order) => (
          <Pressable key={order.order_id} onPress={() => setOrderId(order.order_id)}>
            <Text style={{ color: orderId === order.order_id ? c.primary : c.text }}>
              {order.reference} · {order.status}
            </Text>
          </Pressable>
        ))}
        <Text style={{ color: c.subtitle, fontSize: 12 }}>Référence concernée</Text>
        {(variance.data?.lines ?? []).map((line) => (
          <Pressable key={line.variant_id} onPress={() => {
            setVariantId(line.variant_id);
            setReceivedQuantity(String(line.received_qty));
            setInvoicePrice('');
          }}>
            <Text style={{ color: variantId === line.variant_id ? c.primary : c.text, fontWeight: '700' }}>
              {line.variant_name} · commandé {line.ordered_qty} · reçu {line.received_qty} · manquant {line.missing_qty}
            </Text>
          </Pressable>
        ))}
        <Field label="Quantité reçue" value={receivedQuantity} keyboardType="numeric" onChangeText={setReceivedQuantity} />
        <Field label="Quantité endommagée" value={damagedQuantity} keyboardType="numeric" onChangeText={setDamagedQuantity} />
        <Field label="Prix réellement facturé (FCFA/casier)" value={invoicePrice} keyboardType="numeric" onChangeText={setInvoicePrice} />
        <PrimaryButton
          label="Enregistrer la réception contrôlée"
          loading={receive.isPending}
          disabled={!orderId || !variantId}
          onPress={() => receive.mutate()}
        />
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          Les quantités manquantes ne sont jamais validées automatiquement.
          {variance.data?.auto_validated === false ? ' Assistant conforme.' : ''}
        </Text>
        {(variance.data?.lines ?? []).map((line) => (
          <Row
            key={`ecart-${line.variant_id}`}
            label={`${line.variant_name} · écart prix`}
            value={`${formatFcfa(line.price_difference_fcfa)} · ${line.status}`}
          />
        ))}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Historique des commandes</Text>
        {(orders.data?.orders ?? []).map((order) => (
          <View key={order.order_id} style={{ paddingVertical: 6 }}>
            <Row label={`${order.reference} · ${order.supplier_name}`} value={formatFcfa(order.total_fcfa)} />
            <Row
              label={`Reçu : ${formatFcfa(order.received_fcfa)}`}
              value={formatTimestamp(order.created_at)}
            />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              {formatQuantity(order.total_fcfa - order.received_fcfa)} FCFA non encore reçus
            </Text>
          </View>
        ))}
      </Card>
    </Screen>
  );
}
