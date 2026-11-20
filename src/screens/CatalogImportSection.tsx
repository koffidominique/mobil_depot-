import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import { newRequestKey } from '../core/requestKey';
import type {
  CatalogImportPreview,
  CatalogImportReport,
  CatalogVariantRow,
} from '../core/types';
import {
  useCatalogDashboard,
  useCatalogImportHistory,
  useCatalogProducts,
  useCatalogProducers,
} from '../hooks/useCore';

/**
 * Import de catalogue brasseries (§12 à §15 du cahier des charges).
 *
 * React Native ne peut pas lire un fichier JSON sur le disque de l'appareil :
 * l'OWNER colle le contenu du fichier reçu de la brasserie, l'aperçu est
 * calculé sans rien écrire, puis l'application applique l'import avec une
 * `request_key` d'idempotence. Aucun prix local, aucune valeur de casier,
 * aucun stock et aucune activation ne sont écrasés ; aucune référence absente
 * du JSON n'est supprimée.
 */
export function CatalogImportSection() {
  const c = useColors();
  const queryClient = useQueryClient();
  const dashboard = useCatalogDashboard();
  const producers = useCatalogProducers();
  const runs = useCatalogImportHistory(10);
  const [raw, setRaw] = useState('');
  const [onlyToVerify, setOnlyToVerify] = useState(false);
  const products = useCatalogProducts({ only_to_verify: onlyToVerify, include_inactive: true });
  const [preview, setPreview] = useState<CatalogImportPreview | null>(null);
  const [report, setReport] = useState<CatalogImportReport | null>(null);
  const [requestKey, setRequestKey] = useState('');
  const [capacityAccepted, setCapacityAccepted] = useState(false);
  const [reason, setReason] = useState('Activation catalogue');
  const [producerCode, setProducerCode] = useState('');
  const [producerName, setProducerName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['catalog-dashboard'] }),
      queryClient.invalidateQueries({ queryKey: ['catalog-products'] }),
      queryClient.invalidateQueries({ queryKey: ['catalog-producers'] }),
      queryClient.invalidateQueries({ queryKey: ['catalog-imports'] }),
      queryClient.invalidateQueries({ queryKey: ['catalog-breweries'] }),
      queryClient.invalidateQueries({ queryKey: ['inventory'] }),
      queryClient.invalidateQueries({ queryKey: ['crate-types'] }),
    ]);
  };

  const analyse = useMutation({
    mutationFn: () => depot.previewCatalogImport({ json_text: raw }),
    onSuccess: (data) => {
      setPreview(data);
      setReport(null);
      setCapacityAccepted(false);
      setRequestKey(newRequestKey('catalog-import'));
      setError(null);
      setMessage(`Fichier reconnu : format ${data.detected_format}.`);
    },
    onError: (err: Error) => {
      setPreview(null);
      setError(err.message);
    },
  });

  const apply = useMutation({
    mutationFn: () =>
      depot.importCatalogJson({
        json_text: raw,
        request_key: requestKey || newRequestKey('catalog-import'),
      }),
    onSuccess: async (data) => {
      setReport(data);
      setMessage(
        `${formatQuantity(data.counts.variants_created)} variante(s) ajoutée(s), ` +
          `${formatQuantity(data.counts.variants_updated)} actualisée(s). Prix et stocks conservés.`,
      );
      setError(null);
      setRequestKey('');
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const toggleVariant = useMutation({
    mutationFn: (row: CatalogVariantRow) =>
      depot.updateCatalogVariant({
        variant_id: row.variant_id,
        is_active: !row.is_active,
        reason: reason.trim() || 'Activation catalogue',
      }),
    onSuccess: async () => {
      setMessage('Disponibilité de la variante mise à jour ; prix et stock inchangés.');
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const createProducer = useMutation({
    mutationFn: () =>
      depot.createCatalogProducer({ code: producerCode.trim(), name: producerName.trim() }),
    onSuccess: async () => {
      setProducerCode('');
      setProducerName('');
      setMessage('Brasserie enregistrée ; elle peut désormais recevoir un catalogue.');
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const capacitiesToVerify = preview?.preview.crate_capacities_to_verify ?? [];
  const blockedByCapacity = capacitiesToVerify.length > 0 && !capacityAccepted;
  const canApply = Boolean(preview?.can_apply) && !blockedByCapacity && raw.trim().length > 0;

  return (
    <>
      <Banner message={error} />
      <Banner tone="success" message={message} />

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Catalogue multi-brasseries</Text>
        <Row label="Brasseries" value={formatQuantity(dashboard.data?.producers_count ?? 0)} />
        <Row label="Marques" value={formatQuantity(dashboard.data?.brands_count ?? 0)} />
        <Row label="Variantes" value={formatQuantity(dashboard.data?.variants_count ?? 0)} />
        <Row label="Actives" value={formatQuantity(dashboard.data?.active_variants_count ?? 0)} />
        <Row label="Sans casier suivi" value={formatQuantity(dashboard.data?.variants_without_crate_count ?? 0)} />
        <Row label="Capacités à vérifier" value={formatQuantity(dashboard.data?.crate_capacities_to_verify_count ?? 0)} />
        <Row label="Imports" value={formatQuantity(dashboard.data?.imports_count ?? 0)} />
        {(producers.data?.producers ?? []).map((item) => (
          <Text key={item.producer_id} style={{ color: c.subtitle, fontSize: 11 }}>
            {item.code} · {item.name} · {formatQuantity(item.variants_count ?? 0)} variante(s)
          </Text>
        ))}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Importer un fichier JSON de brasserie</Text>
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          Collez le contenu du fichier reçu (formats SOLIBRA ancien et nouveau, BRASSIVOIRE, générique).
          Seul le type « plastic_crate » est traité comme casier ; les autres types de contenant restent
          des caractéristiques du produit.
        </Text>
        <Field
          label="Contenu du fichier"
          value={raw}
          onChangeText={(value) => {
            setRaw(value);
            setPreview(null);
            setReport(null);
          }}
          placeholder='{"supplier": {"code": "SOLIBRA"}, ...}'
          multiline
        />
        <PrimaryButton label="Analyser sans rien enregistrer" loading={analyse.isPending} onPress={() => analyse.mutate()} />
        {preview ? (
          <View style={{ gap: 4 }}>
            <Row label="Format détecté" value={preview.detected_format} />
            <Row label="Brasserie" value={`${preview.producer_code} · ${preview.producer_name}`} />
            <Row label="Variantes à créer" value={formatQuantity(preview.preview.variants_to_create.length)} />
            <Row label="Variantes à actualiser" value={formatQuantity(preview.preview.variants_to_update.length)} />
            <Row label="Casiers à créer" value={formatQuantity(preview.preview.crate_types_to_create.length)} />
            <Row label="Prix locaux conservés" value={preview.preview.prices_are_preserved ? 'oui' : 'non'} />
            <Row label="Références supprimées" value={formatQuantity(preview.preview.deleted_references.length)} />
            {capacitiesToVerify.length > 0 ? (
              <>
                <Text style={{ color: c.danger ?? c.text, fontSize: 12, fontWeight: '700' }}>
                  Capacité inconnue : {capacitiesToVerify.join(', ')}. Aucune capacité n'est inventée ;
                  ces casiers seront créés « à vérifier » et resteront bloqués pour les sorties tant que la
                  capacité n'est pas saisie.
                </Text>
                <Pressable onPress={() => setCapacityAccepted(!capacityAccepted)}>
                  <Text style={{ color: c.text, fontSize: 12 }}>
                    {capacityAccepted ? '✓ ' : '○ '}J'ai vérifié physiquement ces casiers
                  </Text>
                </Pressable>
              </>
            ) : null}
            {preview.warnings.map((warning, index) => (
              <Text key={`${warning.code}-${index}`} style={{ color: c.subtitle, fontSize: 11 }}>
                {warning.code} · {warning.message}
                {warning.sku ? ` (${warning.sku})` : ''}
              </Text>
            ))}
            <PrimaryButton
              label={apply.isPending ? 'Import en cours…' : 'Appliquer l’import'}
              loading={apply.isPending}
              disabled={!canApply}
              onPress={() => apply.mutate()}
            />
            {blockedByCapacity ? (
              <Text style={{ color: c.subtitle, fontSize: 11 }}>
                Confirmez la vérification des capacités avant d’appliquer l’import.
              </Text>
            ) : null}
          </View>
        ) : null}
        {report ? (
          <View>
            <Text style={{ color: c.text, fontWeight: '700' }}>
              Import {report.idempotent ? 'déjà appliqué (rejeu sans doublon)' : 'appliqué'}
            </Text>
            <Row label="Variantes créées" value={formatQuantity(report.counts.variants_created)} />
            <Row label="Variantes actualisées" value={formatQuantity(report.counts.variants_updated)} />
            <Row label="Casiers créés" value={formatQuantity(report.counts.crate_types_created)} />
            <Row label="Capacités à vérifier" value={formatQuantity(report.to_verify.crate_capacities.length)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>{report.note}</Text>
          </View>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Nouvelle brasserie</Text>
        <Field label="Code" value={producerCode} onChangeText={setProducerCode} placeholder="NBCI" />
        <Field label="Nom" value={producerName} onChangeText={setProducerName} placeholder="Nouvelle Brasserie" />
        <PrimaryButton
          label="Enregistrer la brasserie"
          loading={createProducer.isPending}
          onPress={() => createProducer.mutate()}
        />
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Variantes importées</Text>
        <Pressable onPress={() => setOnlyToVerify(!onlyToVerify)}>
          <Text style={{ color: c.text, fontSize: 12 }}>
            {onlyToVerify ? '✓ ' : '○ '}Afficher seulement ce qui reste à vérifier
          </Text>
        </Pressable>
        <Field label="Motif des modifications" value={reason} onChangeText={setReason} />
        {(products.data?.variants ?? []).map((row) => (
          <Pressable key={row.variant_id} onPress={() => toggleVariant.mutate(row)} style={{ paddingVertical: 6 }}>
            <Row
              label={`${row.name} · ${row.volume_label || 'contenance inconnue'} · ${row.sku}`}
              value={row.is_active ? 'Vendable' : 'Non vendable'}
            />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              {row.producer_name || row.brand} · casier {row.crate_code || 'aucun'} ·{' '}
              {row.crate_capacity > 0 ? `${row.crate_capacity} emplacements` : 'capacité à vérifier'} ·{' '}
              détail {formatFcfa(row.retail_case_fcfa)}
              {row.to_verify ? ' · à vérifier' : ''}
            </Text>
          </Pressable>
        ))}
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          Toucher une variante bascule sa disponibilité : l’import ne l’active jamais automatiquement et ne
          touche ni aux prix, ni au stock.
        </Text>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Derniers imports</Text>
        {(runs.data?.imports ?? []).map((run) => (
          <View key={run.import_run_id} style={{ paddingVertical: 4 }}>
            <Row label={`${run.format} · ${run.status}`} value={formatQuantity(run.created_count)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              {new Date(run.occurred_at).toLocaleString('fr-FR')} · actualisées{' '}
              {formatQuantity(run.updated_count)} · erreurs {formatQuantity(run.error_count)}
            </Text>
          </View>
        ))}
      </Card>
    </>
  );
}
