# Aperçu web interactif

L'application React Native est exécutable dans le navigateur via Vite + `react-native-web`. Cet aperçu sert à **visualiser et manipuler les écrans réels** (`src/screens/*`) sans émulateur ; il ne remplace ni le bundle Android/iOS, ni les tests Rust.

## Démarrage

```bash
npm install
npm run web        # http://localhost:5173, hôte 0.0.0.0, hosts autorisés pour la prévisualisation
```

Comptes pilotes du moteur simulé : `patron / 9999`, `gerant / 1234`, `caissier / 0000`.

## Ce qui est réel et ce qui est simulé

| Élément | Aperçu web | Application mobile |
| --- | --- | --- |
| Écrans, composants, état, thème, navigation entre écrans | code réel `src/` | code identique |
| Moteur de calcul/SQLite Rust | `web/mock/engine.ts` (jeu pilote fr-CI, montants entiers) | TurboModule Rust réel |
| Navigation native | shim `web/shims/navigation.tsx` | React Navigation natif |
| Keychain / Android Keystore | `localStorage` de démonstration | stockage sécurisé natif |
| RNFS / PDF / CSV | fichiers simulés en `localStorage`, téléchargement navigateur | système de fichiers natif |
| NetInfo / AppState | toujours en ligne | états réels |

Le moteur simulé n'implémente qu'un sous-ensemble des commandes Rust et **n'aucune valeur contractuelle** : il ne doit jamais être utilisé pour valider des règles métier, des montants ou une synchronisation. Les commandes non gérées renvoient `COMMAND_NOT_MOCKED`.

## Vérification headless

```bash
npm run web:typecheck
node scripts/verify-web-preview.mjs
```

Le script bundle l'aperçu avec les mêmes alias, le rend dans JSDOM, saisit le PIN `1234`, clique sur « Se connecter », puis **parcourt les écrans Stock, Inventaires, Produits et Commandes clients** (ouverture, contenu attendu, retour) avant de vérifier que l'accueil s'affiche sans aucune erreur console. Il est exécuté en CI dans le job TypeScript : une régression d'écran ou un avertissement React fait échouer la vérification.

## Limites

- Impression système absente : le bouton d'impression déclenche un téléchargement PDF.
- Aucun appel réseau réel : `/api/v1` n'est pas contacté ; la synchronisation reste simulée.
- Le rendu est adapté à une colonne mobile (460 px max) mais ne remplace pas les tests de responsive sur appareils.
- L'initialisation du moteur et les mutations sont en mémoire ; un rechargement rétablit le jeu pilote (la session, elle, est conservée dans `localStorage`).
