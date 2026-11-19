# Guide de déploiement serveur — état préparatoire

## 1. Ce qui est prêt dans ce dépôt

- `rust/` : moteur métier et interface native C, migrations locales 1–9.
- `server/` : adaptateur Axum `/api/v1` avec DTO typés, idempotence, accusés, conflits, devices, noyau SQLx de démonstration.
- `docs/API_SYNC_V1.md` : contrat cible.
- `docs/SYNC_GUIDE.md` : configuration et procédure de validation.

## 2. Conditions bloquantes avant production

1. Brancher `sync_router` dans `depotpro-server` du dépôt parent et déléguer chaque opération à `depotpro-core`. **Le parent n'expose pas encore ces routes.**
2. Ajouter le modèle multi-établissements (`tenant_id`, `branch_id`) au schéma central et appliquer les droits par établissement.
3. Remplacer le noyau SQLx de démonstration par l'implémentation de production, avec persistance réelle des clés d'idempotence et des curseurs.
4. Définir `DEPOTPRO_ALLOWED_ORIGINS` et, si nécessaire, `DEPOTPRO_TRUST_PROXY=1` derrière un proxy de confiance.
5. Terminer TLS, sauvegardes chiffrées et stratégie de rotation des jetons.
6. Exécuter les builds Android/iOS et les tests E2E sur appareils réels.

## 3. Vérification avant mise en service

```bash
cargo test --manifest-path rust/Cargo.toml
cargo test --manifest-path server/Cargo.toml
curl -fsS https://<serveur>/api/health
npm run bundle:check
node scripts/verify-web-preview.mjs
```

Aucun déploiement n'a été effectué et aucun secret n'est fourni dans ce dépôt. Le dépôt parent reste la source de vérité pour le serveur final ; ce dépôt ne doit pas introduire de second serveur métier concurrent.
